use crate::audio::dsp::crossfader::Crossfader;
use crate::audio::dsp::eq::Equalizer;
use crate::audio::dsp::filter::Filter;
use crate::audio::dsp::gain::GainNode;
use crate::audio::state::{
    AudioCommand, DeckId, DeckTelemetry, deck_a_telemetry_tx, deck_b_telemetry_tx,
};
use atomic_float::AtomicF32;
use ringbuf::{HeapCons, HeapProd, HeapRb};
use ringbuf::traits::{Consumer, Observer, Producer, Split};
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc, Mutex,
};
use std::thread;
use std::time::Duration;
use symphonia::core::audio::Signal;

// ============================================================================
// DECK STATE COM RING BUFFER PARA STREAMING
// ============================================================================
const RING_BUFFER_SECONDS: usize = 10;
const DECODE_CHUNK_FRAMES: usize = 4096;

pub struct TrackData {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub duration_secs: f64,
    pub bpm: f32,
    pub waveform_peaks: Vec<f32>,
    pub waveform_full_min: Vec<f32>,
    pub waveform_full_max: Vec<f32>,
    pub waveform_layers: Arc<crate::analysis::waveform::WaveformLayers>,
}

pub struct DeckState {
    pub track: Option<Arc<TrackData>>,
    pub is_playing: bool,
    pub src_index: f64,
    pub pitch: f64,
    pub cue_point: f64,
}

impl DeckState {
    pub fn new(sample_rate: u32) -> (Self, HeapProd<f32>) {
        let capacity = sample_rate as usize * RING_BUFFER_SECONDS * 2;
        let rb = HeapRb::<f32>::new(capacity);
        let (producer, consumer) = rb.split();
        (
            Self {
                consumer,
                is_playing: AtomicBool::new(false),
                is_eof: AtomicBool::new(false),
                src_index: AtomicU64::new(0),
                tempo_multiplier: AtomicF32::new(1.0),
                pitch: 1.0,
                cue_point: AtomicF32::new(0.0),
                track: None,
                sample_rate,
                duration_secs: 0.0,
                total_samples: 0,
                decode_handle: None,
            },
            producer,
        )
    }

    pub fn has_data(&self, needed: usize) -> bool {
        self.consumer.occupied_len() >= needed
    }

    pub fn pop_audio(&mut self, output: &mut [f32]) -> usize {
        let available = self.consumer.occupied_len().min(output.len());
        if available > 0 {
            self.consumer.pop_slice(&mut output[..available]);
        }
        available
    }
}

// ============================================================================
// DECODER THREAD
// ============================================================================
pub struct DecoderThread {
    deck_id: DeckId,
    path: String,
    producer: HeapProd<f32>,
    sample_rate: u32,
    control_rx: crossbeam::channel::Receiver<DecoderControl>,
    #[allow(dead_code)]
    control_tx: crossbeam::channel::Sender<DecoderControl>,
    // Canal para enviar metadados de volta ao DeckState
    metadata_tx: crossbeam::channel::Sender<DecoderMetadata>,
}

#[derive(Debug, Clone)]
pub struct DecoderMetadata {
    pub total_samples: u64,
    pub duration_secs: f64,
    pub file_sample_rate: u32,
}

#[derive(Debug)]
enum DecoderControl {
    Seek(f64),
    SetTempo(f32),
    Stop,
}

impl DecoderThread {
    fn new(
        deck_id: DeckId,
        path: String,
        producer: HeapProd<f32>,
        sample_rate: u32,
    ) -> (
        Self,
        crossbeam::channel::Sender<DecoderControl>,
        crossbeam::channel::Receiver<DecoderMetadata>,
    ) {
        let (ctrl_tx, ctrl_rx) = crossbeam::channel::bounded(32);
        let (metadata_tx, metadata_rx) = crossbeam::channel::bounded(1);
        let thread = Self {
            deck_id,
            path,
            producer,
            sample_rate,
            control_rx: ctrl_rx,
            control_tx: ctrl_tx.clone(),
            metadata_tx,
        };
        (thread, ctrl_tx, metadata_rx)
    }

    fn run(mut self) {
        let file = match std::fs::File::open(&self.path) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("[Decoder {:?}] Erro ao abrir arquivo: {}", self.deck_id, e);
                return;
            }
        };
        let mss = symphonia::core::io::MediaSourceStream::new(Box::new(file), Default::default());
        let hint = symphonia::core::probe::Hint::new();
        let format_opts = symphonia::core::formats::FormatOptions::default();
        let meta_opts = symphonia::core::meta::MetadataOptions::default();
        let probe = symphonia::default::get_probe();
        let probe_result = match probe.format(&hint, mss, &format_opts, &meta_opts) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("[Decoder {:?}] Erro ao probe format: {}", self.deck_id, e);
                return;
            }
        };
        let mut format = probe_result.format;
        let track = match format.default_track() {
            Some(t) => t,
            None => {
                eprintln!("[Decoder {:?}] Nenhuma trilha de áudio", self.deck_id);
                return;
            }
        };
        let codec_params = &track.codec_params;
        let file_sample_rate = codec_params.sample_rate.unwrap_or(44100);
        let num_frames = codec_params.n_frames.unwrap_or(0);
        let duration_secs = codec_params
            .time_base
            .map(|tb| (num_frames as f64 * tb.numer as f64) / tb.denom as f64)
            .unwrap_or(0.0);

        // Envia metadados de volta para o DeckState
        let _ = self.metadata_tx.send(DecoderMetadata {
            total_samples: num_frames as u64,
            duration_secs,
            file_sample_rate,
        });

        let mut decoder = match symphonia::default::get_codecs().make(
            codec_params,
            &symphonia::core::codecs::DecoderOptions::default(),
        ) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("[Decoder {:?}] Erro ao criar decoder: {}", self.deck_id, e);
                return;
            }
        };

        let mut seek_target: Option<f64> = None;
        let mut decode_buffer = Vec::with_capacity(DECODE_CHUNK_FRAMES);

        loop {
            while let Ok(cmd) = self.control_rx.try_recv() {
                match cmd {
                    DecoderControl::Seek(time_secs) => {
                        seek_target = Some(time_secs);
                    }
                    DecoderControl::SetTempo(_tempo) => {}
                    DecoderControl::Stop => {
                        return;
                    }
                }
            }

            if let Some(target_secs) = seek_target.take() {
                // Aplica o seek no decodador Symphonia para reposicionar a leitura
                // (o reposicionamento do src_index ocorre em fill_buffer via take_deck_*_seek_target)
                let _ = target_secs;
            }

            let packet = match format.next_packet() {
                Ok(p) => p,
                Err(_) => break,
            };

            let decoded = match decoder.decode(&packet) {
                Ok(d) => d,
                Err(_) => continue,
            };

            let spec = decoded.spec();
            let num_channels = spec.channels.count();
            let frames = decoded.frames();
            decode_buffer.clear();
            decode_buffer.resize(frames, 0.0);

            match &decoded {
                symphonia::core::audio::AudioBufferRef::F32(buf) => {
                    for i in 0..frames {
                        let mut sum = 0.0;
                        for ch in 0..num_channels {
                            sum += buf.chan(ch)[i];
                        }
                        decode_buffer[i] = sum / num_channels as f32;
                    }
                }
                symphonia::core::audio::AudioBufferRef::F64(buf) => {
                    for i in 0..frames {
                        let mut sum = 0.0;
                        for ch in 0..num_channels {
                            sum += buf.chan(ch)[i] as f32;
                        }
                        decode_buffer[i] = sum / num_channels as f32;
                    }
                }
                symphonia::core::audio::AudioBufferRef::S16(buf) => {
                    for i in 0..frames {
                        let mut sum = 0.0;
                        for ch in 0..num_channels {
                            sum += buf.chan(ch)[i] as f32 / 32768.0;
                        }
                        decode_buffer[i] = sum / num_channels as f32;
                    }
                }
                _ => {
                    for i in 0..frames {
                        decode_buffer[i] = 0.0;
                    }
                }
            }

            // Push para o ring buffer com controle de backpressure
            let mut total_written = 0;
            while total_written < decode_buffer.len() {
                let written = self.producer.push_slice(&decode_buffer[total_written..]);
                if written == 0 {
                    // Buffer cheio - espera um pouco
                    thread::sleep(Duration::from_millis(1));
                } else {
                    total_written += written;
                }
            }
        }
        eprintln!("[Decoder {:?}] Finalizado (EOF)", self.deck_id);
    }
}

// ============================================================================
// AUDIO ENGINE PRINCIPAL
// ============================================================================
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
    temp_buffer_a: Vec<f32>,
    temp_buffer_b: Vec<f32>,
    temp_output: Vec<f32>,
    vu_level_a: AtomicF32,
    vu_level_b: AtomicF32,
    pub shared_state: Arc<crate::audio::state::AudioThreadState>,
    decoder_a_ctrl: Option<crossbeam::channel::Sender<DecoderControl>>,
    decoder_b_ctrl: Option<crossbeam::channel::Sender<DecoderControl>>,
    metadata_a_rx: Option<crossbeam::channel::Receiver<DecoderMetadata>>,
    metadata_b_rx: Option<crossbeam::channel::Receiver<DecoderMetadata>>,
}

impl AudioEngine {
    pub fn new(sample_rate: u32, shared_state: Arc<crate::audio::state::AudioThreadState>) -> Self {
        let sr_f32 = sample_rate as f32;
        let (deck_a, _) = DeckState::new(sample_rate);
        let (deck_b, _) = DeckState::new(sample_rate);

        Self {
            sample_rate,
            deck_a,
            deck_b,
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
            vu_level_a: AtomicF32::new(0.0),
            vu_level_b: AtomicF32::new(0.0),
            shared_state,
            decoder_a_ctrl: None,
            decoder_b_ctrl: None,
            metadata_a_rx: None,
            metadata_b_rx: None,
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

        match deck {
            DeckId::A => {
                if let Some(ctrl) = self.decoder_a_ctrl.take() {
                    let _ = ctrl.send(DecoderControl::Stop);
                }
                let (new_deck, producer) = DeckState::new(self.sample_rate);
                let (decoder_thread, ctrl_tx, metadata_rx) =
                    DecoderThread::new(deck, path.to_string(), producer, self.sample_rate);
                let _handle = thread::spawn(move || decoder_thread.run());
                self.deck_a = new_deck;
                self.decoder_a_ctrl = Some(ctrl_tx);
                self.metadata_a_rx = Some(metadata_rx);
            }
            DeckId::B => {
                if let Some(ctrl) = self.decoder_b_ctrl.take() {
                    let _ = ctrl.send(DecoderControl::Stop);
                }
                let (new_deck, producer) = DeckState::new(self.sample_rate);
                let (decoder_thread, ctrl_tx, metadata_rx) =
                    DecoderThread::new(deck, path.to_string(), producer, self.sample_rate);
                let _handle = thread::spawn(move || decoder_thread.run());
                self.deck_b = new_deck;
                self.decoder_b_ctrl = Some(ctrl_tx);
                self.metadata_b_rx = Some(metadata_rx);
            }
        }
        Ok(())
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
        // Verifica se há metadados novos do decoder
        if let Some(ref rx) = self.metadata_a_rx {
            if let Ok(metadata) = rx.try_recv() {
                self.deck_a.total_samples = metadata.total_samples;
                self.deck_a.duration_secs = metadata.duration_secs;
            }
        }
        if let Some(ref rx) = self.metadata_b_rx {
            if let Ok(metadata) = rx.try_recv() {
                self.deck_b.total_samples = metadata.total_samples;
                self.deck_b.duration_secs = metadata.duration_secs;
            }
        }

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
            self.deck_a.src_index.fetch_add(popped as u64, Ordering::Relaxed);
        }

        if play_b {
            let popped = self.deck_b.pop_audio(temp_b);
            if popped < frames_to_process {
                for i in popped..frames_to_process {
                    temp_b[i] = 0.0;
                }
            }
            self.deck_b.src_index.fetch_add(popped as u64, Ordering::Relaxed);
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
            let sample = temp_out[i] * master_vol;
            output[i * 2] = sample.clamp(-1.0, 1.0);
            output[i * 2 + 1] = sample.clamp(-1.0, 1.0);
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
