use crate::analysis::bpm::detect_bpm;
use crate::analysis::waveform::{
    generate_full_waveform, generate_waveform_layers, generate_waveform_peaks, WaveformLayers,
};
use crate::audio::decoder::AudioDecoder;
use crate::audio::dsp::crossfader::Crossfader;
use crate::audio::dsp::eq::Equalizer;
use crate::audio::dsp::filter::Filter;
use crate::audio::dsp::gain::GainNode;
use crate::audio::dsp::limiter::PeakLimiter;
use crate::audio::dsp::resample::resample_linear;
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
    pub trim: f64,
    pub loop_in: f64,
    pub loop_out: f64,
    pub loop_enabled: bool,
    pub loop_active: bool,
    pub loop_exit_index: f64,
}

impl Default for DeckState {
    fn default() -> Self {
        Self {
            track: None,
            is_playing: false,
            src_index: 0.0,
            pitch: 1.0,
            cue_point: 0.0,
            key_lock: false,
            trim: 1.0,
            loop_in: 0.0,
            loop_out: 0.0,
            loop_enabled: false,
            loop_active: false,
            loop_exit_index: 0.0,
        }
    }
}

pub struct AudioEngine {
    pub sample_rate: u32,
    pub decks: Vec<DeckState>,
    pub eqs: Vec<Equalizer>,
    pub gains: Vec<GainNode>,
    pub filters: Vec<Filter>,
    pub crossfader: Crossfader,
    pub master_limiter: PeakLimiter,
    pub temp_buffers: Vec<Vec<f32>>,
    pub temp_ts_inputs: Vec<Vec<f32>>,
    pub temp_ts_outputs: Vec<Vec<f32>>,
    pub vu_levels: Vec<f32>,
    pub num_channels: u16,
    pub temp_output: Vec<f32>,
}

impl AudioEngine {
    pub fn new(sample_rate: u32, num_decks: usize, num_channels: u16) -> Self {
        let sr_f32 = sample_rate as f32;

        let mut decks = Vec::with_capacity(num_decks);
        let mut eqs = Vec::with_capacity(num_decks);
        let mut gains = Vec::with_capacity(num_decks);
        let mut filters = Vec::with_capacity(num_decks);
        let mut temp_buffers = Vec::with_capacity(num_decks);
        let mut temp_ts_inputs = Vec::with_capacity(num_decks);
        let mut temp_ts_outputs = Vec::with_capacity(num_decks);
        let mut vu_levels = Vec::with_capacity(num_decks);

        for _ in 0..num_decks {
            decks.push(DeckState::default());
            eqs.push(Equalizer::new(sr_f32));
            gains.push(GainNode::new(1.0));
            filters.push(Filter::new(sr_f32));
            temp_buffers.push(vec![0.0; 2048]);
            temp_ts_inputs.push(vec![0.0; 512]);
            temp_ts_outputs.push(vec![0.0; 512]);
            vu_levels.push(0.0);
        }

        Self {
            sample_rate,
            decks,
            eqs,
            gains,
            filters,
            crossfader: Crossfader::new(),
            master_limiter: PeakLimiter::new(sample_rate),
            temp_buffers,
            temp_ts_inputs,
            temp_ts_outputs,
            vu_levels,
            num_channels,
            temp_output: vec![0.0; 2048],
        }
    }

    pub fn num_decks(&self) -> usize {
        self.decks.len()
    }

    pub fn deck_state(&self, deck_idx: usize) -> Option<&DeckState> {
        self.decks.get(deck_idx)
    }

    pub fn load_track(
        &mut self,
        deck_idx: usize,
        path: &str,
    ) -> Result<Arc<TrackData>, Box<dyn std::error::Error>> {
        if deck_idx >= self.decks.len() {
            return Err(format!(
                "Deck index {} out of range (max {})",
                deck_idx,
                self.decks.len()
            )
            .into());
        }

        let (samples, sr, _ch, dur) = AudioDecoder::load_and_decode(path)?;

        // Resample track para o sample rate do engine se necessário
        let samples = if sr != self.sample_rate {
            resample_linear(&samples, sr, self.sample_rate)
        } else {
            samples.to_vec()
        };

        let bpm = detect_bpm(&samples, self.sample_rate as f32);
        let peaks = generate_waveform_peaks(&samples, 1000);
        let full = generate_full_waveform(&samples, 1000);
        let layers = generate_waveform_layers(&samples, self.sample_rate as f32, 5000);

        let track = Arc::new(TrackData {
            samples,
            sample_rate: self.sample_rate,
            duration_secs: dur,
            bpm,
            waveform_peaks: peaks,
            waveform_full_min: full.min,
            waveform_full_max: full.max,
            waveform_layers: Arc::new(layers),
        });

        let deck = &mut self.decks[deck_idx];
        deck.track = Some(track.clone());
        deck.src_index = 0.0;
        deck.is_playing = false;
        deck.pitch = 1.0;
        deck.cue_point = 0.0;
        deck.key_lock = false;
        self.filters[deck_idx].reset();

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

    pub fn interpolate_sample(samples: &[f32], index: f64) -> f32 {
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

    fn render_deck(&mut self, deck_idx: usize, temp: &mut [f32]) {
        let deck = &mut self.decks[deck_idx];
        temp.fill(0.0);

        if let Some(track) = &deck.track {
            if deck.is_playing {
                // Key lock ativo: preserva o tom (pitch = 1.0) independente do pitch fader
                let pitch = if deck.key_lock { 1.0 } else { deck.pitch };

                // Loop management
                if deck.loop_enabled {
                    if deck.loop_active {
                        if deck.src_index >= deck.loop_out {
                            deck.src_index = deck.loop_in;
                        }
                    } else {
                        if deck.src_index >= deck.loop_out {
                            deck.loop_active = true;
                            deck.src_index = deck.loop_in;
                        }
                    }
                }

                for sample in temp.iter_mut() {
                    if deck.src_index >= track.samples.len() as f64 {
                        deck.is_playing = false;
                        deck.src_index = track.samples.len() as f64;
                        break;
                    }
                    *sample = Self::interpolate_sample(&track.samples, deck.src_index);
                    deck.src_index += pitch;
                }

                // Aplica trim (ganho linear)
                for sample in temp.iter_mut() {
                    *sample *= deck.trim as f32;
                }
            }
        }

        // DSP: filter -> EQ -> gain
        self.filters[deck_idx].process(temp);
        self.eqs[deck_idx].process(temp);
        self.gains[deck_idx].process(temp);

        // VU level after gain (reflects real channel volume)
        self.vu_levels[deck_idx] = Self::calculate_rms(temp);
    }

    pub fn fill_buffer(&mut self, output: &mut [f32]) {
        let frames_to_process = output.len() / self.num_channels as usize;

        // Garante que temp_output tenha tamanho suficiente
        if self.temp_output.len() < frames_to_process {
            self.temp_output.resize(frames_to_process, 0.0);
        }

        // Renderiza cada deck em buffer separado (self emprestado mutavelmente, depois liberado)
        let mut deck_buffers: Vec<Vec<f32>> = Vec::new();
        for deck_idx in 0..self.decks.len() {
            let mut buf = vec![0.0; frames_to_process];
            self.render_deck(deck_idx, &mut buf);
            deck_buffers.push(buf);
        }

        // Debug: ver se há áudio sendo produzido
        let _deck0_rms = deck_buffers
            .first()
            .map(|b| b.iter().map(|s| s * s).sum::<f32>() / b.len() as f32)
            .unwrap_or(0.0)
            .sqrt();
        let _deck1_rms = deck_buffers
            .get(1)
            .map(|b| b.iter().map(|s| s * s).sum::<f32>() / b.len() as f32)
            .unwrap_or(0.0)
            .sqrt();

        // Agora self está livre — cria temp_out e acumula com crossfader
        let temp_out = &mut self.temp_output[..frames_to_process];
        temp_out.fill(0.0);

        // Calcula ganhos do crossfader (Equal Power)
        let (gain_a, gain_b) = self.crossfader.calculate_gains();

        for (deck_idx, buf) in deck_buffers.iter().enumerate() {
            let gain = if deck_idx == 0 { gain_a } else { gain_b };
            for (i, sample) in buf.iter().enumerate().take(frames_to_process) {
                temp_out[i] += sample * gain;
            }
        }

        // Master limiter — previne clipping no output final
        let _limiter_gr = self.master_limiter.process(temp_out);

        // Escreve para output (multi-canal)
        for i in 0..frames_to_process {
            for ch in 0..self.num_channels {
                output[i * self.num_channels as usize + ch as usize] = temp_out[i];
            }
        }
    }

    pub fn get_position(&self, deck_idx: usize) -> Option<(f64, f32)> {
        let state = self.decks.get(deck_idx)?;

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
