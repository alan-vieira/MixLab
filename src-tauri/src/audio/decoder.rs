use std::fs::File;
use std::sync::Arc;
use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::conv::FromSample;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::default::get_probe;

/// Resultado da decodificação: (samples mono, sample_rate, channels, duração)
type DecodeResult = (Vec<f32>, u32, u16, f64);

/// Resultado da decodificação rápida: (preview_samples, sample_rate, channels, duração, full_samples_arc)
type QuickDecodeResult = (Vec<f32>, u32, u16, f64, Arc<Vec<f32>>);

pub struct AudioDecoder;

impl AudioDecoder {
    /// Decodifica o arquivo inteiro para um vetor de samples mono (f32)
    /// OTIMIZADO: Pré-aloca vetor e usa downmix mais eficiente
    #[allow(clippy::type_complexity)]
    pub fn load_and_decode(path: &str) -> Result<DecodeResult, Box<dyn std::error::Error>> {
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

        // OTIMIZAÇÃO: Pré-aloca o vetor com tamanho estimado
        let estimated_samples = if num_frames > 0 {
            (num_frames as usize) * (channels as usize)
        } else {
            44100 * 60 * 5 // Estimativa para 5 minutos
        };
        let mut all_samples = Vec::with_capacity(estimated_samples);

        while let Ok(packet) = format.next_packet() {
            let decoded = decoder.decode(&packet)?;
            let spec = decoded.spec();
            let num_channels = spec.channels.count();
            let frames = decoded.frames();

            // OTIMIZAÇÃO: Processa por frames em vez de sample por sample
            match &decoded {
                AudioBufferRef::U8(b) => {
                    for i in 0..frames {
                        let sum: f32 = (0..num_channels)
                            .map(|ch| f32::from_sample(b.chan(ch)[i]))
                            .sum();
                        all_samples.push(sum / num_channels as f32);
                    }
                }
                AudioBufferRef::U16(b) => {
                    for i in 0..frames {
                        let sum: f32 = (0..num_channels)
                            .map(|ch| f32::from_sample(b.chan(ch)[i]))
                            .sum();
                        all_samples.push(sum / num_channels as f32);
                    }
                }
                AudioBufferRef::U24(b) => {
                    for i in 0..frames {
                        let sum: f32 = (0..num_channels)
                            .map(|ch| f32::from_sample(b.chan(ch)[i]))
                            .sum();
                        all_samples.push(sum / num_channels as f32);
                    }
                }
                AudioBufferRef::U32(b) => {
                    for i in 0..frames {
                        let sum: f32 = (0..num_channels)
                            .map(|ch| f32::from_sample(b.chan(ch)[i]))
                            .sum();
                        all_samples.push(sum / num_channels as f32);
                    }
                }
                AudioBufferRef::S8(b) => {
                    for i in 0..frames {
                        let sum: f32 = (0..num_channels)
                            .map(|ch| f32::from_sample(b.chan(ch)[i]))
                            .sum();
                        all_samples.push(sum / num_channels as f32);
                    }
                }
                AudioBufferRef::S16(b) => {
                    for i in 0..frames {
                        let sum: f32 = (0..num_channels)
                            .map(|ch| f32::from_sample(b.chan(ch)[i]))
                            .sum();
                        all_samples.push(sum / num_channels as f32);
                    }
                }
                AudioBufferRef::S24(b) => {
                    for i in 0..frames {
                        let sum: f32 = (0..num_channels)
                            .map(|ch| f32::from_sample(b.chan(ch)[i]))
                            .sum();
                        all_samples.push(sum / num_channels as f32);
                    }
                }
                AudioBufferRef::S32(b) => {
                    for i in 0..frames {
                        let sum: f32 = (0..num_channels)
                            .map(|ch| f32::from_sample(b.chan(ch)[i]))
                            .sum();
                        all_samples.push(sum / num_channels as f32);
                    }
                }
                AudioBufferRef::F32(b) => {
                    for i in 0..frames {
                        let sum: f32 = (0..num_channels)
                            .map(|ch| f32::from_sample(b.chan(ch)[i]))
                            .sum();
                        all_samples.push(sum / num_channels as f32);
                    }
                }
                AudioBufferRef::F64(b) => {
                    for i in 0..frames {
                        let sum: f32 = (0..num_channels)
                            .map(|ch| f32::from_sample(b.chan(ch)[i]))
                            .sum();
                        all_samples.push(sum / num_channels as f32);
                    }
                }
            }
        }

        Ok((all_samples, sample_rate, channels, duration_secs))
    }

    /// Decodifica apenas os primeiros 30 segundos + metadados, retorna samples completos em Arc
    /// OTIMIZADO: Para fast load - preview imediato + análise completa em background
    #[allow(clippy::type_complexity)]
    pub fn load_and_decode_quick(path: &str) -> Result<QuickDecodeResult, Box<dyn std::error::Error>> {
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

        // Preview: apenas primeiros 30 segundos
        let preview_frames_limit = (sample_rate * 30) as usize * (channels as usize);
        let mut preview_samples = Vec::with_capacity(preview_frames_limit.min(44100 * 30 * 2));
        let mut all_samples = Vec::with_capacity(if num_frames > 0 {
            (num_frames as usize) * (channels as usize)
        } else {
            44100 * 60 * 5
        });

        let mut total_frames_decoded = 0;
        while let Ok(packet) = format.next_packet() {
            let decoded = decoder.decode(&packet)?;
            let spec = decoded.spec();
            let num_channels = spec.channels.count();
            let frames = decoded.frames();

            match &decoded {
                AudioBufferRef::U8(b) => {
                    for i in 0..frames {
                        let sum: f32 = (0..num_channels)
                            .map(|ch| f32::from_sample(b.chan(ch)[i]))
                            .sum();
                        let mono = sum / num_channels as f32;
                        all_samples.push(mono);
                        if total_frames_decoded < (sample_rate as usize * 30) {
                            preview_samples.push(mono);
                        }
                        total_frames_decoded += 1;
                    }
                }
                AudioBufferRef::U16(b) => {
                    for i in 0..frames {
                        let sum: f32 = (0..num_channels)
                            .map(|ch| f32::from_sample(b.chan(ch)[i]))
                            .sum();
                        let mono = sum / num_channels as f32;
                        all_samples.push(mono);
                        if total_frames_decoded < (sample_rate as usize * 30) {
                            preview_samples.push(mono);
                        }
                        total_frames_decoded += 1;
                    }
                }
                AudioBufferRef::U24(b) => {
                    for i in 0..frames {
                        let sum: f32 = (0..num_channels)
                            .map(|ch| f32::from_sample(b.chan(ch)[i]))
                            .sum();
                        let mono = sum / num_channels as f32;
                        all_samples.push(mono);
                        if total_frames_decoded < (sample_rate as usize * 30) {
                            preview_samples.push(mono);
                        }
                        total_frames_decoded += 1;
                    }
                }
                AudioBufferRef::U32(b) => {
                    for i in 0..frames {
                        let sum: f32 = (0..num_channels)
                            .map(|ch| f32::from_sample(b.chan(ch)[i]))
                            .sum();
                        let mono = sum / num_channels as f32;
                        all_samples.push(mono);
                        if total_frames_decoded < (sample_rate as usize * 30) {
                            preview_samples.push(mono);
                        }
                        total_frames_decoded += 1;
                    }
                }
                AudioBufferRef::S8(b) => {
                    for i in 0..frames {
                        let sum: f32 = (0..num_channels)
                            .map(|ch| f32::from_sample(b.chan(ch)[i]))
                            .sum();
                        let mono = sum / num_channels as f32;
                        all_samples.push(mono);
                        if total_frames_decoded < (sample_rate as usize * 30) {
                            preview_samples.push(mono);
                        }
                        total_frames_decoded += 1;
                    }
                }
                AudioBufferRef::S16(b) => {
                    for i in 0..frames {
                        let sum: f32 = (0..num_channels)
                            .map(|ch| f32::from_sample(b.chan(ch)[i]))
                            .sum();
                        let mono = sum / num_channels as f32;
                        all_samples.push(mono);
                        if total_frames_decoded < (sample_rate as usize * 30) {
                            preview_samples.push(mono);
                        }
                        total_frames_decoded += 1;
                    }
                }
                AudioBufferRef::S24(b) => {
                    for i in 0..frames {
                        let sum: f32 = (0..num_channels)
                            .map(|ch| f32::from_sample(b.chan(ch)[i]))
                            .sum();
                        let mono = sum / num_channels as f32;
                        all_samples.push(mono);
                        if total_frames_decoded < (sample_rate as usize * 30) {
                            preview_samples.push(mono);
                        }
                        total_frames_decoded += 1;
                    }
                }
                AudioBufferRef::S32(b) => {
                    for i in 0..frames {
                        let sum: f32 = (0..num_channels)
                            .map(|ch| f32::from_sample(b.chan(ch)[i]))
                            .sum();
                        let mono = sum / num_channels as f32;
                        all_samples.push(mono);
                        if total_frames_decoded < (sample_rate as usize * 30) {
                            preview_samples.push(mono);
                        }
                        total_frames_decoded += 1;
                    }
                }
                AudioBufferRef::F32(b) => {
                    for i in 0..frames {
                        let sum: f32 = (0..num_channels)
                            .map(|ch| f32::from_sample(b.chan(ch)[i]))
                            .sum();
                        let mono = sum / num_channels as f32;
                        all_samples.push(mono);
                        if total_frames_decoded < (sample_rate as usize * 30) {
                            preview_samples.push(mono);
                        }
                        total_frames_decoded += 1;
                    }
                }
                AudioBufferRef::F64(b) => {
                    for i in 0..frames {
                        let sum: f32 = (0..num_channels)
                            .map(|ch| f32::from_sample(b.chan(ch)[i]))
                            .sum();
                        let mono = sum / num_channels as f32;
                        all_samples.push(mono);
                        if total_frames_decoded < (sample_rate as usize * 30) {
                            preview_samples.push(mono);
                        }
                        total_frames_decoded += 1;
                    }
                }
            }
        }

        Ok((preview_samples, sample_rate, channels, duration_secs, Arc::new(all_samples)))
    }
}
