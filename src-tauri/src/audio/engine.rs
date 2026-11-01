use crate::analysis::bpm::detect_bpm;
use crate::analysis::waveform::{
    generate_waveform_layers, generate_waveform_peaks, WaveformLayers,
};
use crate::audio::decoder::AudioDecoder;
use crate::audio::dsp::crossfader::Crossfader;
use crate::audio::dsp::eq::Equalizer;
use crate::audio::dsp::filter::Filter;
use crate::audio::dsp::gain::GainNode;
use crate::audio::dsp::limiter::PeakLimiter;
use crate::audio::dsp::resample::resample_linear;
use rubato::{FftFixedInOut, Resampler};
use std::sync::Arc;

#[derive(Clone)]
pub struct TrackData {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub duration_secs: f64,
    pub bpm: f32,
    pub waveform_peaks: Vec<f32>,
    pub waveform_full_min: Vec<f32>,
    pub waveform_full_max: Vec<f32>,
    pub waveform_layers: Arc<WaveformLayers>,
}

#[derive(Clone)]
pub struct DeckState {
    pub track: Option<Arc<TrackData>>,
    pub is_playing: bool,
    pub src_index: f64,
    pub pitch: f64,
    pub cue_point: f64,
    pub key_lock: bool,
}

pub struct AudioEngine {
    pub sample_rate: u32,
    pub deck_a: DeckState,
    pub deck_b: DeckState,
    pub deck_a_eq: Equalizer,
    pub deck_a_gain: GainNode,
    pub deck_a_filter: Filter,
    pub deck_b_eq: Equalizer,
    pub deck_b_gain: GainNode,
    pub deck_b_filter: Filter,
    pub crossfader: Crossfader,
    pub deck_a_timestretch: Option<FftFixedInOut<f32>>,
    pub deck_b_timestretch: Option<FftFixedInOut<f32>>,
    pub master_limiter: PeakLimiter,
    pub temp_buffer_a: Vec<f32>,
    pub temp_buffer_b: Vec<f32>,
    pub temp_output: Vec<f32>,
    pub temp_ts_input_a: Vec<f32>,
    pub temp_ts_input_b: Vec<f32>,
    pub temp_ts_output_a: Vec<f32>,
    pub temp_ts_output_b: Vec<f32>,
    pub vu_level_a: f32,
    pub vu_level_b: f32,
}

impl AudioEngine {
    pub fn new(sample_rate: u32) -> Self {
        let sr_f32 = sample_rate as f32;
        Self {
            sample_rate,
            deck_a: DeckState {
                track: None,
                is_playing: false,
                src_index: 0.0,
                pitch: 1.0,
                cue_point: 0.0,
                key_lock: false,
            },
            deck_b: DeckState {
                track: None,
                is_playing: false,
                src_index: 0.0,
                pitch: 1.0,
                cue_point: 0.0,
                key_lock: false,
            },
            deck_a_eq: Equalizer::new(sr_f32),
            deck_a_gain: GainNode::new(1.0),
            deck_a_filter: Filter::new(sr_f32),
            deck_b_eq: Equalizer::new(sr_f32),
            deck_b_gain: GainNode::new(1.0),
            deck_b_filter: Filter::new(sr_f32),
            crossfader: Crossfader::new(),
            deck_a_timestretch: None,
            deck_b_timestretch: None,
            master_limiter: PeakLimiter::new(sample_rate),
            temp_buffer_a: vec![0.0; 2048],
            temp_buffer_b: vec![0.0; 2048],
            temp_output: vec![0.0; 2048],
            temp_ts_input_a: vec![0.0; 512],
            temp_ts_input_b: vec![0.0; 512],
            temp_ts_output_a: vec![0.0; 512],
            temp_ts_output_b: vec![0.0; 512],
            vu_level_a: 0.0,
            vu_level_b: 0.0,
        }
    }

    pub fn load_track(
        &mut self,
        deck: char,
        path: &str,
    ) -> Result<Arc<TrackData>, Box<dyn std::error::Error>> {
        let (samples, sr, _ch, dur) = AudioDecoder::load_and_decode(path)?;

        // 🔧 Resample track para o sample rate do engine se necessário
        let samples = if sr != self.sample_rate {
            resample_linear(&samples, sr, self.sample_rate)
        } else {
            samples
        };

        // ✅ CORRIGIDO: Agora salvamos os dados corretamente
        let bpm = detect_bpm(&samples, self.sample_rate as f32);
        let peaks = generate_waveform_peaks(&samples, 1000);
        let layers = generate_waveform_layers(&samples, self.sample_rate as f32, 5000);

        let track = Arc::new(TrackData {
            samples,
            sample_rate: self.sample_rate,
            duration_secs: dur,
            bpm,
            waveform_peaks: peaks.clone(),
            waveform_full_min: peaks.clone(),
            waveform_full_max: peaks.clone(),
            waveform_layers: Arc::new(layers),
        });

        if deck == 'A' {
            self.deck_a.track = Some(track.clone());
            self.deck_a.src_index = 0.0;
            self.deck_a.is_playing = false;
            self.deck_a.pitch = 1.0;
            self.deck_a.cue_point = 0.0;
            self.deck_a.key_lock = false;
            self.deck_a_filter.reset();
            self.deck_a_timestretch = None; // será criado sob demanda em fill_buffer
        } else {
            self.deck_b.track = Some(track.clone());
            self.deck_b.src_index = 0.0;
            self.deck_b.is_playing = false;
            self.deck_b.pitch = 1.0;
            self.deck_b.cue_point = 0.0;
            self.deck_b.key_lock = false;
            self.deck_b_filter.reset();
            self.deck_b_timestretch = None;
        }

        Ok(track)
    }

    /// Cria (ou recria) o Rubato timestretcher para um deck quando key_lock muda ou pitch é ajustado.
    /// stretch_ratio = 1.0 / pitch (quanto mais rápido o pitch, mais lento o stretch para manter o tom)
    fn ensure_timestretch(&mut self, deck: char) {
        let (state, timestretch_field) = match deck {
            'A' => (&self.deck_a, &mut self.deck_a_timestretch),
            'B' => (&self.deck_b, &mut self.deck_b_timestretch),
            _ => return,
        };

        if !state.key_lock || state.track.is_none() {
            *timestretch_field = None;
            return;
        }

        let pitch = state.pitch;
        if (pitch - 1.0).abs() < 0.01 {
            // Pitch próximo de 1.0: não precisa de timestretch perceptível
            *timestretch_field = None;
            return;
        }

        let stretch_ratio = 1.0 / pitch;
        let to_rate = (self.sample_rate as f64 * stretch_ratio) as usize;

        if to_rate == 0 || to_rate == self.sample_rate as usize {
            *timestretch_field = None;
            return;
        }

        // FftFixedInOut: from=input_rate, to=output_rate, frames_per_block, channels
        *timestretch_field = Some(
            FftFixedInOut::<f32>::new(
                self.sample_rate as usize,
                to_rate,
                512,
                1,
            )
            .expect("Falha ao criar Rubato timestretcher"),
        );
    }

    pub fn calculate_rms(buffer: &[f32]) -> f32 {
        if buffer.is_empty() {
            return 0.0;
        }
        let sum_squares: f32 = buffer.iter().map(|s| s * s).sum();
        let rms = (sum_squares / buffer.len() as f32).sqrt();
        (rms * 2.0).min(1.0)
    }

    // ✅ NOVA FUNÇÃO: Interpolação linear para pitch suave e sincronizado
    fn interpolate_sample(samples: &[f32], index: f64) -> f32 {
        let idx = index as usize;
        let frac = index - idx as f64;

        if idx + 1 < samples.len() {
            let s0 = samples[idx];
            let s1 = samples[idx + 1];
            s0 * (1.0 - frac as f32) + s1 * frac as f32
        } else if idx < samples.len() {
            samples[idx]
        } else {
            0.0
        }
    }

    pub fn fill_buffer(&mut self, output: &mut [f32]) {
        // (Re)cria timestretchers se necessário (quando key_lock/pitch mudou)
        self.ensure_timestretch('A');
        self.ensure_timestretch('B');

        let frames_to_process = output.len() / 2;

        if self.temp_buffer_a.len() < frames_to_process {
            self.temp_buffer_a.resize(frames_to_process, 0.0);
            self.temp_buffer_b.resize(frames_to_process, 0.0);
            self.temp_output.resize(frames_to_process, 0.0);
        }

        let temp_a = &mut self.temp_buffer_a[..frames_to_process];
        let temp_b = &mut self.temp_buffer_b[..frames_to_process];
        let temp_out = &mut self.temp_output[..frames_to_process];

        temp_a.fill(0.0);
        temp_b.fill(0.0);

        // DECK A com ou sem timestretch (key_lock)
        if let Some(track) = &self.deck_a.track {
            if self.deck_a.is_playing {
                let pitch = self.deck_a.pitch;

                if self.deck_a.key_lock {
                    if let Some(ts) = &mut self.deck_a_timestretch {
                        // 1. Lê samples na velocidade original (sem pitch shift)
                        let samples_available = (track.samples.len() as f64 - self.deck_a.src_index).max(0.0) as usize;
                        let input_needed = ts.input_frames_next();
                        let input_to_read = samples_available.min(input_needed).min(frames_to_process);

                        for i in 0..input_to_read {
                            self.temp_ts_input_a[i] = Self::interpolate_sample(
                                &track.samples,
                                self.deck_a.src_index + i as f64,
                            );
                        }
                        self.deck_a.src_index += input_to_read as f64;

                        // 2. Processa pelo Rubato (time-stretch preserva o tom)
                        let out = ts.process(&[&self.temp_ts_input_a[..input_to_read]], None)
                            .expect("Falha no processamento Rubato deck A");

                        // 3. Coloca saída no temp_a
                        let mut out_idx = 0;
                        for ch in out.iter() {
                            for &sample in ch.iter() {
                                if out_idx < frames_to_process {
                                    temp_a[out_idx] = sample;
                                    out_idx += 1;
                                }
                            }
                        }
                    }
                } else {
                    // Sem key_lock: comportamento original (pitch shift direto)
                    for sample in temp_a.iter_mut().take(frames_to_process) {
                        if self.deck_a.src_index >= track.samples.len() as f64 {
                            self.deck_a.is_playing = false;
                            self.deck_a.src_index = track.samples.len() as f64;
                            break;
                        }
                        *sample = Self::interpolate_sample(&track.samples, self.deck_a.src_index);
                        self.deck_a.src_index += pitch;
                    }
                }
            }
        }

        // DECK B com ou sem timestretch (key_lock)
        if let Some(track) = &self.deck_b.track {
            if self.deck_b.is_playing {
                let pitch = self.deck_b.pitch;

                if self.deck_b.key_lock {
                    if let Some(ts) = &mut self.deck_b_timestretch {
                        let samples_available = (track.samples.len() as f64 - self.deck_b.src_index).max(0.0) as usize;
                        let input_needed = ts.input_frames_next();
                        let input_to_read = samples_available.min(input_needed).min(frames_to_process);

                        for i in 0..input_to_read {
                            self.temp_ts_input_b[i] = Self::interpolate_sample(
                                &track.samples,
                                self.deck_b.src_index + i as f64,
                            );
                        }
                        self.deck_b.src_index += input_to_read as f64;

                        let out = ts.process(&[&self.temp_ts_input_b[..input_to_read]], None)
                            .expect("Falha no processamento Rubato deck B");

                        let mut out_idx = 0;
                        for ch in out.iter() {
                            for &sample in ch.iter() {
                                if out_idx < frames_to_process {
                                    temp_b[out_idx] = sample;
                                    out_idx += 1;
                                }
                            }
                        }
                    }
                } else {
                    // Sem key_lock: comportamento original (pitch shift direto)
                    for sample in temp_b.iter_mut().take(frames_to_process) {
                        if self.deck_b.src_index >= track.samples.len() as f64 {
                            self.deck_b.is_playing = false;
                            self.deck_b.src_index = track.samples.len() as f64;
                            break;
                        }
                        *sample = Self::interpolate_sample(&track.samples, self.deck_b.src_index);
                        self.deck_b.src_index += pitch;
                    }
                }
            }
        }

        // Preenche resto do buffer com silêncio (caso timestretch não preencheu tudo)
        let tail_start = frames_to_process.min(temp_a.len());
        for sample in &mut temp_a[tail_start..] {
            *sample = 0.0;
        }
        let tail_start_b = frames_to_process.min(temp_b.len());
        for sample in &mut temp_b[tail_start_b..] {
            *sample = 0.0;
        }

        // DEPOIS aplica o processamento DSP
        self.deck_a_filter.process(temp_a);
        self.deck_b_filter.process(temp_b);
        self.deck_a_eq.process(temp_a);
        self.deck_a_gain.process(temp_a);
        self.deck_b_eq.process(temp_b);
        self.deck_b_gain.process(temp_b);

        // ✅ VU CALCULADO APÓS GAIN (reflete o volume real do canal)
        self.vu_level_a = Self::calculate_rms(temp_a);
        self.vu_level_b = Self::calculate_rms(temp_b);

        self.crossfader.mix(temp_a, temp_b, temp_out);

        // 🔇 MASTER LIMITER — previne clipping no output final
        let limiter_gr = self.master_limiter.process(temp_out);

        for i in 0..frames_to_process {
            output[i * 2] = temp_out[i];
            output[i * 2 + 1] = temp_out[i];
        }
    }

    pub fn get_position(&self, deck: char) -> Option<(f64, f32)> {
        let state = match deck {
            'A' => &self.deck_a,
            'B' => &self.deck_b,
            _ => return None,
        };

        if let Some(track) = &state.track {
            let total_samples = track.samples.len();

            if total_samples == 0 {
                return None;
            }

            let percent = (state.src_index / total_samples as f64).min(1.0) as f32;
            let seconds = (state.src_index / track.sample_rate as f64).min(track.duration_secs);

            Some((seconds, percent))
        } else {
            None
        }
    }
}
