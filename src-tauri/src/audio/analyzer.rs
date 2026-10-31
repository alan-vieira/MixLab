use std::f32::consts::PI;
use std::fs::File;
use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::default::get_probe;

use crate::analysis::bpm::detect_bpm;

/// Resultado da análise streaming (sem carregar todo o arquivo na RAM)
#[derive(Debug, Clone)]
pub struct TrackAnalysis {
    pub duration_secs: f64,
    pub sample_rate: u32,
    pub channels: u16,
    pub bpm: f32,
    pub peaks: Vec<f32>,
    pub full_waveform_min: Vec<f32>,
    pub full_waveform_max: Vec<f32>,
    pub layers_low: Vec<f32>,
    pub layers_mid: Vec<f32>,
    pub layers_high: Vec<f32>,
}

/// Número de samples mono acumulados antes de processar um bloco de análise.
/// Quanto maior, menos frequente a chamada ao callback, mais RAM usada temporariamente.
const ANALYSIS_CHUNK_SAMPLES: usize = 8192;

/// Número de pontos da waveform para visualização
const WAVEFORM_POINTS: usize = 1000;

/// Tamanho da janela FFT (deve ser potência de 2)
const FFT_SIZE: usize = 2048;

/// Número de samples entre janelas FFT consecutivas (overlap de 50%)
const FFT_HOP: usize = 1024;

/// Freqüência de corte Low/Mid (Hz)
const LOW_MID_CUTOFF: f32 = 250.0;

/// Freqüência de corte Mid/High (Hz)
const MID_HIGH_CUTOFF: f32 = 4000.0;

/// Analisa um arquivo de áudio em streaming via symphonia.
///
/// Lê o arquivo packet por packet, convertendo para samples mono f32 e
/// alimentando as funções de análise (BPM, waveform) sem acumular todos os
/// samples na RAM. Os samples brutos são descartados após cada chunk.
///
/// Retorna metadados + resultados de análise prontos para salvar no banco/cache.
pub fn analyze_track_streaming(path: &str) -> Result<TrackAnalysis, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let hint = Hint::new();
    let format_opts = FormatOptions::default();
    let meta_opts = MetadataOptions::default();
    let probe = get_probe();
    let probe_result = probe.format(&hint, mss, &format_opts, &meta_opts)?;
    let mut format = probe_result.format;

    let track = format
        .default_track()
        .ok_or("Nenhuma trilha de áudio encontrada")?;
    let codec_params = &track.codec_params;
    let sample_rate = codec_params.sample_rate.unwrap_or(44100);
    let channels = codec_params.channels.map(|c| c.count() as u16).unwrap_or(2);
    let num_frames = codec_params.n_frames.unwrap_or(0);

    let duration_secs = codec_params
        .time_base
        .map(|tb| (num_frames as f64 * tb.numer as f64) / tb.denom as f64)
        .unwrap_or(0.0);

    let mut decoder =
        symphonia::default::get_codecs().make(codec_params, &DecoderOptions::default())?;

    // Estruturas de análise incremental
    // Para BPM: acumulamos samples para FFT (primeiros 60s são suficientes)
    let max_bpm_samples = (sample_rate as f32 * 60.0) as usize;
    let mut bpm_buffer: Vec<f32> = Vec::with_capacity(max_bpm_samples.min(44100 * 60));

    // Para waveform peaks/full: acumulamos min/max por ponto baseado na posição global
    let mut peak_accumulator: Vec<(f32, f32)> =
        vec![(f32::INFINITY, f32::NEG_INFINITY); WAVEFORM_POINTS];
    let estimated_total = if num_frames > 0 {
        (num_frames as usize) * (channels as usize)
    } else {
        44100 * 60 * 5 // Estimativa para 5 minutos
    };
    let samples_per_point = estimated_total / WAVEFORM_POINTS.max(1);

    // Buffer para acumular samples mono para FFT espectral
    // Usamos um buffer circular limitado para não carregar tudo na RAM de uma vez
    let mut spectral_buffer: Vec<f32> = Vec::with_capacity(FFT_SIZE * 4);

    let mut decode_buffer = Vec::with_capacity(ANALYSIS_CHUNK_SAMPLES);
    let mut samples_consumed: usize = 0;

    while let Ok(packet) = format.next_packet() {
        let decoded = match decoder.decode(&packet) {
            Ok(d) => d,
            Err(_) => continue,
        };

        let spec = decoded.spec();
        let num_channels = spec.channels.count();
        let frames = decoded.frames();

        // Converte packet para samples mono f32
        decode_buffer.clear();
        decode_buffer.resize(frames, 0.0);

        match &decoded {
            AudioBufferRef::F32(buf) => {
                for i in 0..frames {
                    let mut sum = 0.0;
                    for ch in 0..num_channels {
                        sum += buf.chan(ch)[i];
                    }
                    decode_buffer[i] = sum / num_channels as f32;
                }
            }
            AudioBufferRef::F64(buf) => {
                for i in 0..frames {
                    let mut sum = 0.0;
                    for ch in 0..num_channels {
                        sum += buf.chan(ch)[i] as f32;
                    }
                    decode_buffer[i] = sum / num_channels as f32;
                }
            }
            AudioBufferRef::S16(buf) => {
                for i in 0..frames {
                    let mut sum = 0.0;
                    for ch in 0..num_channels {
                        sum += buf.chan(ch)[i] as f32 / 32768.0;
                    }
                    decode_buffer[i] = sum / num_channels as f32;
                }
            }
            AudioBufferRef::S32(buf) => {
                for i in 0..frames {
                    let mut sum = 0.0;
                    for ch in 0..num_channels {
                        sum += buf.chan(ch)[i] as f32 / 2147483648.0;
                    }
                    decode_buffer[i] = sum / num_channels as f32;
                }
            }
            _ => {
                // Formato não suportado - silêncio
                for i in 0..frames {
                    decode_buffer[i] = 0.0;
                }
            }
        }

        // Alimenta BPM buffer (apenas primeiros 60s)
        if bpm_buffer.len() < max_bpm_samples {
            let remaining = max_bpm_samples - bpm_buffer.len();
            let to_take = decode_buffer.len().min(remaining);
            bpm_buffer.extend_from_slice(&decode_buffer[..to_take]);
        }

        // Alimenta waveform accumulator baseado na posição global
        for (i, &sample) in decode_buffer.iter().enumerate() {
            let global_idx = samples_consumed + i;
            let point_idx = global_idx / samples_per_point;
            if point_idx < WAVEFORM_POINTS {
                let (cur_min, cur_max) = &mut peak_accumulator[point_idx];
                if sample < *cur_min {
                    *cur_min = sample;
                }
                if sample > *cur_max {
                    *cur_max = sample;
                }
            }
        }

        // Alimenta buffer espectral para FFT (limita tamanho para não explodir RAM)
        spectral_buffer.extend_from_slice(&decode_buffer);
        if spectral_buffer.len() > FFT_SIZE * 4 {
            // Descarta a metade mais antiga para manter limite de RAM
            spectral_buffer.drain(0..FFT_SIZE * 2);
        }

        samples_consumed += decode_buffer.len();
    }

    // --- BPM ---
    let bpm = if bpm_buffer.len() >= (sample_rate as usize) {
        detect_bpm(&bpm_buffer, sample_rate as f32)
    } else {
        0.0
    };

    // --- Waveform peaks a partir do accumulator ---
    let peaks: Vec<f32> = peak_accumulator
        .iter()
        .map(|(min, max)| {
            if *min == f32::INFINITY {
                0.0
            } else {
                ((max - min) / 2.0).abs()
            }
        })
        .collect();

    // --- Full waveform min/max ---
    let full_min: Vec<f32> = peak_accumulator.iter().map(|(m, _)| *m).collect();
    let full_max: Vec<f32> = peak_accumulator.iter().map(|(_, m)| *m).collect();

    // --- Layers espectrais via FFT real por banda ---
    let (layers_low, layers_mid, layers_high) =
        compute_spectral_layers(&spectral_buffer, sample_rate, WAVEFORM_POINTS);

    Ok(TrackAnalysis {
        duration_secs,
        sample_rate,
        channels,
        bpm,
        peaks,
        full_waveform_min: full_min,
        full_waveform_max: full_max,
        layers_low,
        layers_mid,
        layers_high,
    })
}

/// Calcula layers espectrais (Low/Mid/High) via FFT real sobre janelas de áudio.
///
/// Divide o buffer de samples em janelas sobrepostas, aplica FFT em cada uma,
/// e separa a energia em três bandas freqüenciais:
/// - Low:  20–250 Hz
/// - Mid:  250–4000 Hz
/// - High: 4000–Nyquist Hz
///
/// Retorna três vetores com WAVEFORM_POINTS elementos cada, normalizados para [0, 1].
fn compute_spectral_layers(
    samples: &[f32],
    sample_rate: u32,
    num_points: usize,
) -> (Vec<f32>, Vec<f32>, Vec<f32>) {
    use num_complex::Complex32;
    use realfft::RealFftPlanner;

    let mut low = vec![0.0f32; num_points];
    let mut mid = vec![0.0f32; num_points];
    let mut high = vec![0.0f32; num_points];

    if samples.len() < FFT_SIZE {
        return (low, mid, high);
    }

    let mut planner = RealFftPlanner::<f32>::new();
    let r2c = planner.plan_fft_forward(FFT_SIZE);

    let mut fft_buffer = vec![0.0f32; FFT_SIZE];
    let mut spectrum = vec![Complex32::new(0.0, 0.0); FFT_SIZE / 2 + 1];

    // Bin width em Hz
    let bin_width = sample_rate as f32 / FFT_SIZE as f32;

    // Índices de bin para os cortes de banda
    let low_mid_bin = (LOW_MID_CUTOFF / bin_width) as usize;
    let mid_high_bin = (MID_HIGH_CUTOFF / bin_width) as usize;

    let num_windows = if samples.len() >= FFT_SIZE {
        (samples.len() - FFT_SIZE) / FFT_HOP + 1
    } else {
        1
    };

    // Mapeia janelas para pontos de saída (distribui uniformemente)
    for point in 0..num_points {
        // Escolhe qual janela FFT corresponde a este ponto de saída
        let window_idx = if num_windows > 1 {
            (point * (num_windows - 1)) / (num_points - 1)
        } else {
            0
        };
        let start = window_idx * FFT_HOP;

        if start + FFT_SIZE > samples.len() {
            break;
        }

        // Copia janela com função de janela Hann para reduzir vazamento espectral
        for i in 0..FFT_SIZE {
            let hann = 0.5 * (1.0 - (2.0 * PI * i as f32 / (FFT_SIZE - 1) as f32).cos());
            fft_buffer[i] = samples[start + i] * hann;
        }

        // Executa FFT
        r2c.process(&mut fft_buffer, &mut spectrum).ok();

        // Acumula energia por banda (ignorando o bin DC e Nyquist)
        let mut low_energy = 0.0f32;
        let mut mid_energy = 0.0f32;
        let mut high_energy = 0.0f32;

        for bin in 1..spectrum.len() {
            let magnitude = spectrum[bin].norm();
            let energy = magnitude * magnitude; // potência

            if bin < low_mid_bin {
                low_energy += energy;
            } else if bin < mid_high_bin {
                mid_energy += energy;
            } else {
                high_energy += energy;
            }
        }

        // Normalização por raiz quadrada da energia total (escala perceptiva)
        let total = low_energy + mid_energy + high_energy;
        if total > 0.0 {
            let scale = 1.0 / total.sqrt();
            low[point] = (low_energy * scale).clamp(0.0, 1.0);
            mid[point] = (mid_energy * scale).clamp(0.0, 1.0);
            high[point] = (high_energy * scale).clamp(0.0, 1.0);
        }
    }

    (low, mid, high)
}
