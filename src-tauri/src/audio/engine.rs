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
    pub src_index: f32,
    pub pitch: f32,
    pub cue_point: f32,
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

    // 👈 NOVO: Níveis para os VU Meters
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

        let _bpm = detect_bpm(&samples, sr as f32);
        let _peaks = generate_waveform_peaks(&samples, 1000);
        let _layers = generate_waveform_layers(&samples, sr as f32, 5000);

        let track = Arc::new(TrackData {
            samples,
            sample_rate: sr,
            duration_secs: dur,
            bpm: 0.0, // Valor placeholder, já que este método é legado
            waveform_peaks: vec![],
            waveform_full_min: vec![], // ✅ ADICIONE ESTA LINHA
            waveform_full_max: vec![], // ✅ ADICIONE ESTA LINHA
            waveform_layers: Arc::new(crate::analysis::waveform::WaveformLayers {
                low: vec![],
                mid: vec![],
                high: vec![],
            }),
        });

        if deck == 'A' {
            self.deck_a.track = Some(track.clone());
            self.deck_a.src_index = 0.0;
            self.deck_a.is_playing = false;
            self.deck_a.pitch = 1.0;
            self.deck_a_filter.reset();
        } else {
            self.deck_b.track = Some(track.clone());
            self.deck_b.src_index = 0.0;
            self.deck_b.is_playing = false;
            self.deck_b.pitch = 1.0;
            self.deck_b_filter.reset();
        }

        Ok(track)
    }

    // 👈 CORREÇÃO: Função associada que NÃO usa &self, evitando conflito de borrow
    pub fn calculate_rms(buffer: &[f32]) -> f32 {
        if buffer.is_empty() {
            return 0.0;
        }
        let sum_squares: f32 = buffer.iter().map(|s| s * s).sum();
        let rms = (sum_squares / buffer.len() as f32).sqrt();
        (rms * 2.0).min(1.0)
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

        if let Some(track) = &self.deck_a.track {
            if self.deck_a.is_playing {
                let pitch = self.deck_a.pitch;
                for sample in temp_a.iter_mut().take(frames_to_process) {
                    let idx = self.deck_a.src_index as usize;
                    if idx >= track.samples.len() {
                        self.deck_a.is_playing = false;
                        self.deck_a.src_index = 0.0;
                        break;
                    }
                    *sample = track.samples[idx];
                    self.deck_a.src_index += pitch;
                }
            }
        }

        if let Some(track) = &self.deck_b.track {
            if self.deck_b.is_playing {
                let pitch = self.deck_b.pitch;
                for sample in temp_b.iter_mut().take(frames_to_process) {
                    let idx = self.deck_b.src_index as usize;
                    if idx >= track.samples.len() {
                        self.deck_b.is_playing = false;
                        self.deck_b.src_index = 0.0;
                        break;
                    }
                    *sample = track.samples[idx];
                    self.deck_b.src_index += pitch;
                }
            }
        }

        self.deck_a_filter.process(temp_a);
        self.deck_b_filter.process(temp_b);

        self.deck_a_eq.process(temp_a);
        self.deck_a_gain.process(temp_a);
        self.deck_b_eq.process(temp_b);
        self.deck_b_gain.process(temp_b);

        self.crossfader.mix(temp_a, temp_b, temp_out);

        for i in 0..frames_to_process {
            output[i * 2] = temp_out[i];
            output[i * 2 + 1] = temp_out[i];
        }

        // 👈 CORREÇÃO: Chama como Self::calculate_rms (sem &self)
        self.vu_level_a = Self::calculate_rms(temp_a);
        self.vu_level_b = Self::calculate_rms(temp_b);
    }
}
