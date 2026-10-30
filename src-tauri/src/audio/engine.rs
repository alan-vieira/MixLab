use crate::analysis::bpm::detect_bpm;
use crate::analysis::waveform::{
    generate_waveform_layers, generate_waveform_peaks, WaveformLayers,
};
use crate::audio::decoder::AudioDecoder;
use crate::audio::dsp::crossfader::Crossfader;
use crate::audio::dsp::eq::Equalizer;
use crate::audio::dsp::filter::Filter;
use crate::audio::dsp::gain::GainNode;
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
    pub temp_buffer_a: Vec<f32>,
    pub temp_buffer_b: Vec<f32>,
    pub temp_output: Vec<f32>,
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
            },
            deck_b: DeckState {
                track: None,
                is_playing: false,
                src_index: 0.0,
                pitch: 1.0,
                cue_point: 0.0,
            },
            deck_a_eq: Equalizer::new(sr_f32),
            deck_a_gain: GainNode::new(1.0),
            deck_a_filter: Filter::new(sr_f32),
            deck_b_eq: Equalizer::new(sr_f32),
            deck_b_gain: GainNode::new(1.0),
            deck_b_filter: Filter::new(sr_f32),
            crossfader: Crossfader::new(),
            temp_buffer_a: vec![0.0; 2048],
            temp_buffer_b: vec![0.0; 2048],
            temp_output: vec![0.0; 2048],
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

        // ✅ CORRIGIDO: Agora salvamos os dados corretamente
        let bpm = detect_bpm(&samples, sr as f32);
        let peaks = generate_waveform_peaks(&samples, 1000);
        let layers = generate_waveform_layers(&samples, sr as f32, 5000);

        let track = Arc::new(TrackData {
            samples,
            sample_rate: sr,
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
            self.deck_a_filter.reset();
        } else {
            self.deck_b.track = Some(track.clone());
            self.deck_b.src_index = 0.0;
            self.deck_b.is_playing = false;
            self.deck_b.pitch = 1.0;
            self.deck_b.cue_point = 0.0;
            self.deck_b_filter.reset();
        }

        Ok(track)
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

        // DECK A com interpolação linear
        if let Some(track) = &self.deck_a.track {
            if self.deck_a.is_playing {
                let pitch = self.deck_a.pitch;
                for sample in temp_a.iter_mut().take(frames_to_process) {
                    if self.deck_a.src_index >= track.samples.len() as f64 {
                        self.deck_a.is_playing = false;
                        // ✅ TRAVA EXATAMENTE NO FINAL
                        self.deck_a.src_index = track.samples.len() as f64;
                        break;
                    }
                    *sample = Self::interpolate_sample(&track.samples, self.deck_a.src_index);
                    self.deck_a.src_index += pitch;
                }
            }
        }

        // DECK B com interpolação linear
        if let Some(track) = &self.deck_b.track {
            if self.deck_b.is_playing {
                let pitch = self.deck_b.pitch;
                for sample in temp_b.iter_mut().take(frames_to_process) {
                    if self.deck_b.src_index >= track.samples.len() as f64 {
                        self.deck_b.is_playing = false;
                        // ✅ TRAVA EXATAMENTE NO FINAL
                        self.deck_b.src_index = track.samples.len() as f64;
                        break;
                    }
                    *sample = Self::interpolate_sample(&track.samples, self.deck_b.src_index);
                    self.deck_b.src_index += pitch;
                }
            }
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
