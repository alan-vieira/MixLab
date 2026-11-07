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
use rubato::{SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction};
use std::num::{NonZeroU16, NonZeroU32};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use serde_json;
use wsola::{TimeStretch, MAX_TEMPO, MIN_TEMPO};

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
    pub key_lock_prev: bool, // rastreia se key_lock estava ativo no frame anterior (detecta transição)
    pub key_lock_fade: usize, // samples de fade-in restantes na transicao OFF->ON (evita clique)
    pub ultimo_sample: f32, // ultimo sample produzido por este deck (para fade na transicao)
    pub key_lock_ratio: Option<f64>,
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
            key_lock_prev: false,
            key_lock_fade: 0,
            ultimo_sample: 0.0,
            key_lock_ratio: None,
            trim: 1.0,
            loop_in: 0.0,
            loop_out: 0.0,
            loop_enabled: false,
            loop_active: false,
            loop_exit_index: 0.0,
        }
    }
}

/// Estado do time-stretcher WSOLA por deck.
///
/// O WSOLA (crate `wsola`) faz time-stretching **preservando o pitch** — que é
/// exatamente o que o KEY LOCK precisa. O Rubato faz *resampling* (pitch e
/// tempo mudam juntos), portanto não serve para key lock.
///
/// Topologia (streaming, mono -> interleaved):
///   track.samples (mono) -> [DSP chain] -> wsola.push() -> wsola.pull()
///     -> FIFO de saída -> interleave nos canais do dispositivo
///
/// O `wsola` trabalha com buffers *interleaved*. Como o decoder produz áudio
/// **mono** (`Vec<f32>`), usamos `channels = 1` e replicamos o resultado para
/// os canais de saída.
pub struct KeyLockStretch {
    /// O time-stretcher WSOLA (mono).
    stretcher: TimeStretch,
    /// FIFO de saída: amostras já esticadas aguardando o consumidor (CPAL).
    /// Armazenado mono; será replicado para os canais no momento da escrita.
    output_fifo: Vec<f32>,
    /// Cursor de leitura na track original (em amostras, mono).
    /// Gerenciado pelo engine para que `src_index` e o loop do deck fiquem
    /// consistentes com o que realmente foi consumido.
    read_cursor: f64,
    /// Tempo (playback speed) aplicado ao WSOLA. 1.0 = normal.
    tempo: f32,
    /// Hop do WSOLA em amostras mono (= `min_output_buff_size` com 1 canal).
    hop: usize,
}

impl KeyLockStretch {
    pub fn new(sample_rate: u32) -> Self {
        let sr = NonZeroU32::new(sample_rate.max(1)).expect("sample_rate >= 1");
        let ch = NonZeroU16::new(1).expect("1 canal");
        let stretcher = TimeStretch::new(sr, ch).expect("Falha ao criar WSOLA");
        let hop = stretcher.min_output_buff_size();
        Self {
            stretcher,
            output_fifo: Vec::new(),
            read_cursor: 0.0,
            tempo: 1.0,
            hop,
        }
    }

    /// Aplica o novo playback speed preservando o pitch.
    pub fn set_tempo(&mut self, tempo: f32) {
        let clamped = if tempo.is_finite() && tempo > 0.0 {
            tempo.clamp(MIN_TEMPO, MAX_TEMPO)
        } else {
            1.0
        };
        if (clamped - self.tempo).abs() > 0.001 {
            self.tempo = clamped;
            self.stretcher.set_tempo(clamped);
        }
    }

    pub fn tempo(&self) -> f32 {
        self.tempo
    }

    /// Posição de leitura na track original (amostras mono).
    pub fn read_cursor(&self) -> f64 {
        self.read_cursor
    }

    /// Define a posição de leitura (usado por seek / cue).
    pub fn set_read_cursor(&mut self, index: f64) {
        self.read_cursor = index;
    }

    /// Reposiciona o cursor de leitura e limpa o estado interno do WSOLA.
    pub fn seek(&mut self, index: f64) {
        self.read_cursor = index;
        self.output_fifo.clear();
        self.stretcher.reset();
    }

    /// Reposiciona o cursor de leitura SEM limpar o estado interno do WSOLA.
    /// Usado na transição pitch_shift -> key_lock para preservar a fase e
    /// evitar clique de descontinuidade (salto de amostra).
    pub fn seek_sem_reset(&mut self, index: f64) {
        self.read_cursor = index;
        self.output_fifo.clear();
    }

    /// Quantidade de amostras esticadas prontas no FIFO.
    pub fn pending(&self) -> usize {
        self.output_fifo.len()
    }

    /// Alimenta o WSOLA com `samples` mono.
    pub fn push(&mut self, samples: &[f32]) {
        self.stretcher.push(samples);
    }

    /// Tenta extrair output esticado do WSOLA para o FIFO.
    ///
    /// O WSOLA produz `hop` amostras por `step` e exige um buffer de saída
    /// múltiplo de `hop`. Retorna `true` se produziu algo.
    fn drain_to_fifo(&mut self, scratch: &mut Vec<f32>) -> bool {
        let mut produced = false;
        // `pull` processa enquanto houver espaço; um buffer de 4 hops permite
        // vários steps por chamada sem alocar por step.
        let capacity = self.hop * 4;
        if scratch.len() != capacity {
            scratch.resize(capacity, 0.0);
        }
        scratch.fill(0.0);

        match self.stretcher.pull(scratch.as_mut_slice()) {
            Ok(written) => {
                if written > 0 {
                    self.output_fifo.extend_from_slice(&scratch[..written]);
                    produced = true;
                }
            }
            Err(_) => {
                // NotEnoughInput: ainda não há dados suficientes para um step.
                // Isso é NORMAL no regime de streaming — o próximo push resolve.
                // NÃO é erro, e NÃO deve avançar o cursor.
            }
        }
        produced
    }

    /// Retira até `out.len()` amostras do FIFO.
    pub fn drain_output(&mut self, out: &mut [f32]) -> usize {
        let n = out.len().min(self.output_fifo.len());
        out[..n].copy_from_slice(&self.output_fifo[..n]);
        self.output_fifo.drain(..n);
        n
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
    pub vu_peaks: Vec<f32>,
    pub vu_last_update_ms: Vec<f64>,
    pub num_channels: u16,
    pub temp_output: Vec<f32>,
    /// Time-stretchers WSOLA para KEY LOCK — um por deck (mono).
    pub key_lock_stretchers: Vec<KeyLockStretch>,
    /// Scratch buffer reutilizado para `wsola.pull` (evita alocação por callback).
    pub wsola_scratch: Vec<f32>,
    /// Resamplers Rubato para pitch shift (key lock OFF) — um por canal, por deck.
    pub ts_resamplers: Vec<Vec<SincFixedIn<f32>>>,
    /// Buffers de entrada (não-estirado, pitch 1.0) — intercalado por deck
    pub ts_input_buffers: Vec<Vec<f32>>,
    /// Buffers de saída do resampler Rubato — intercalado por deck
    pub ts_output_buffers: Vec<Vec<f32>>,
    /// Callback para emissão de eventos de deck (ex: `track_ended`).
    /// Recebe (nome_do_evento, payload_json).
    pub event_emitter: Option<Box<dyn Fn(&str, serde_json::Value) + Send + Sync>>,
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
        let mut vu_peaks = Vec::with_capacity(num_decks);
        let mut vu_last_update_ms = Vec::with_capacity(num_decks);

        let mut ts_resamplers = Vec::with_capacity(num_decks);
        let mut ts_input_buffers = Vec::with_capacity(num_decks);
        let mut ts_output_buffers = Vec::with_capacity(num_decks);
        let mut key_lock_stretchers = Vec::with_capacity(num_decks);

        let ts_ratio = 1.0;
        let frames_per_callback = 512;

        for _ in 0..num_decks {
            decks.push(DeckState::default());
            eqs.push(Equalizer::new(sr_f32));
            gains.push(GainNode::new(1.0));
            filters.push(Filter::new(sr_f32));
            temp_buffers.push(vec![0.0; 2048]);
            temp_ts_inputs.push(vec![0.0; 512]);
            temp_ts_outputs.push(vec![0.0; 512]);
            vu_levels.push(0.0);
            vu_peaks.push(0.0);
            vu_last_update_ms.push(Self::now_ms());

            // Um resampler Rubato por canal — Rubato espera AsRef<[f32]> / AsMut<[f32]>
            let mut deck_resamplers = Vec::with_capacity(num_channels as usize);
            for _ch in 0..num_channels {
                let resampler = SincFixedIn::<f32>::new(
                    ts_ratio,
                    1.0,
                    SincInterpolationParameters {
                        sinc_len: 256,
                        f_cutoff: 0.95,
                        interpolation: SincInterpolationType::Linear,
                        oversampling_factor: 256,
                        window: WindowFunction::BlackmanHarris2,
                    },
                    frames_per_callback,
                    1,
                )
                .expect("Falha ao criar resampler Rubato");
                deck_resamplers.push(resampler);
            }
            ts_resamplers.push(deck_resamplers);

            ts_input_buffers.push(vec![0.0; frames_per_callback * num_channels as usize]);
            ts_output_buffers.push(vec![0.0; frames_per_callback * num_channels as usize]);

            // KEY LOCK: WSOLA mono — preserva pitch enquanto altera o tempo.
            key_lock_stretchers.push(KeyLockStretch::new(sample_rate));
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
            vu_peaks,
            vu_last_update_ms,
            num_channels,
            temp_output: vec![0.0; 2048],
            key_lock_stretchers,
            wsola_scratch: Vec::new(),
            ts_resamplers,
            ts_input_buffers,
            ts_output_buffers,
            event_emitter: None,
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
        deck.key_lock_ratio = None;
        self.filters[deck_idx].reset();

        Ok(track)
    }

    pub fn calculate_rms(buffer: &[f32]) -> f32 {
        if buffer.is_empty() {
            return 0.0;
        }
        let sum_squares: f32 = buffer.iter().map(|s| s * s).sum();
        let rms = (sum_squares / buffer.len() as f32).sqrt();
        // Escala de VU em dB: 20*log10(rms) mapeada para 0..1
        // -60 dBFS = 0.0, 0 dBFS = 1.0
        let db = 20.0 * rms.log10();
        ((db + 60.0) / 60.0).clamp(0.0, 1.0)
    }

    /// Retorna o tempo atual em milissegundos desde a época Unix.
    fn now_ms() -> f64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs_f64() * 1000.0)
            .unwrap_or(0.0)
    }

    /// Injeta o emissor de eventos do Tauri para emissão de eventos (ex: `track_ended`).
    /// Chamado pela `AppState` após a construção; fica `None` em testes puro-Rust.
    pub fn set_event_emitter(&mut self, emitter: Option<Box<dyn Fn(&str, serde_json::Value) + Send + Sync>>) {
        self.event_emitter = emitter;
    }

    /// Emite o evento `track_ended` para o frontend quando uma track termina
    /// de tocar (EOF sem loop), permitindo o frontend re-sincronizar
    /// `isPlaying` imediatamente, sem esperar o próximo poll de 100ms.
    fn emit_track_ended(&self, deck_idx: usize) {
        let emitter = match &self.event_emitter {
            Some(e) => e,
            None => return,
        };
        let deck_label = if deck_idx == 0 { "A" } else { "B" };
        let _ = emitter("track_ended", serde_json::json!({ "deck": deck_label, "deck_idx": deck_idx }));
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

    /// Atualiza o VU meter de um deck após o preenchimento do buffer.
    fn update_vu(&mut self, deck_idx: usize, temp: &[f32]) {
        let rms = Self::calculate_rms(temp);
        let now = Self::now_ms();

        if rms > self.vu_peaks[deck_idx] {
            self.vu_peaks[deck_idx] = rms;
            self.vu_last_update_ms[deck_idx] = now;
        } else {
            let hold_ms = 100.0;
            let decay_per_ms = 0.02;
            let elapsed = now - self.vu_last_update_ms[deck_idx];
            if elapsed > hold_ms {
                let decay = (-(elapsed - hold_ms) * decay_per_ms).exp() as f32;
                self.vu_peaks[deck_idx] *= decay;
            }
        }

        self.vu_levels[deck_idx] = self.vu_peaks[deck_idx];
    }

    /// Renderiza o deck com KEY LOCK ATIVO usando WSOLA.
    ///
    /// Contrato:
    /// - O WSOLA recebe samples **mono** da track (o decoder já faz downmix).
    /// - O FIFO desacopla a granularidade do WSOLA (hop ~15ms) da do callback
    ///   CPAL (512 frames ~11.6ms), evitando underruns e o "pulo para o final".
    /// - O cursor da track só avança quando realmente consumimos da track.
    fn render_deck_key_lock(&mut self, deck_idx: usize, temp: &mut [f32]) {
        // `temp` guarda 1 amostra POR FRAME em mono (mesmo contrato de
        // `render_deck_pitch_shift`). O interleave para os canais do
        // dispositivo acontece em `fill_buffer`.
        //
        // BUG FIX: esta linha dividia por `num_ch` uma segunda vez. Como
        // `fill_buffer` ja entrega frames (nao samples interleaved), o
        // resultado era cada frame duplicado -> audio a metade da
        // velocidade com o tom pela metade (440Hz -> 220Hz).
        let output_frames_needed = temp.len();

        // Snapshot do estado do deck (evita borrow conflicts).
        let (
            track_len,
            is_playing,
            pitch,
            trim,
            loop_enabled,
            loop_active,
            loop_in,
            loop_out,
            src_index,
            key_lock,
            key_lock_prev,
        ) = {
            let deck = &self.decks[deck_idx];
            (
                deck.track.as_ref().map(|t| t.samples.len()).unwrap_or(0),
                deck.is_playing,
                deck.pitch,
                deck.trim,
                deck.loop_enabled,
                deck.loop_active,
                deck.loop_in,
                deck.loop_out,
                deck.src_index,
                deck.key_lock,
                deck.key_lock_prev,
            )
        };

        if track_len == 0 || !is_playing {
            self.vu_levels[deck_idx] = 0.0;
            return;
        }

        // Aplica o playback speed ao WSOLA (preserva pitch).
        self.key_lock_stretchers[deck_idx].set_tempo(pitch as f32);

        // Sincroniza o cursor do WSOLA com o src_index do deck (seek/cue).
        // Na transição OFF->ON do key_lock, faz seek incondicional para
        // alinhar o estado interno do WSOLA com src_index e evitar clique
        // de descontinuidade (salto de amostra) ao alternar modos.
        {
            let stretcher = &mut self.key_lock_stretchers[deck_idx];
            let entrou_no_key_lock = !key_lock_prev && key_lock;
            if entrou_no_key_lock || (stretcher.read_cursor() - src_index).abs() > 1.0 {
                // seek_sem_reset na transição preserva o estado interno do WSOLA
                // (janela de Hann acumulada), evitando clique de descontinuidade.
                stretcher.seek_sem_reset(src_index);
            }
            // Inicia fade-in ao entrar no key_lock para suavizar a transição
            if entrou_no_key_lock {
                self.decks[deck_idx].key_lock_fade = 1024; // ~23ms de fade-in a 44.1kHz
            }
        }

        // Buffer mono para coletar o output esticado do WSOLA.
        let mut mono_out = vec![0.0; output_frames_needed];
        let mut frames_ready = 0usize;

        // --- 1. Consome o que já está no FIFO ---
        frames_ready += self.key_lock_stretchers[deck_idx].drain_output(&mut mono_out);

        // --- 2. Produz mais output se necessário ---
        let mut cursor = self.key_lock_stretchers[deck_idx].read_cursor();
        let mut eof = false;

        // O WSOLA precisa de ~frame+search amostras de input para produzir o
        // primeiro hop. Alimentamos em blocos até ele produzir saída.
        // Limite de iterações evita loop infinito em bordas (EOF / loop).
        let mut guard = 0usize;
        const MAX_FEED_ITERATIONS: usize = 64;

        while frames_ready < output_frames_needed && guard < MAX_FEED_ITERATIONS {
            guard += 1;

            // Puxa o que houver no FIFO (pode ter sido produzido em pulls anteriores).
            if frames_ready < output_frames_needed {
                let n =
                    self.key_lock_stretchers[deck_idx].drain_output(&mut mono_out[frames_ready..]);
                frames_ready += n;
            }

            if frames_ready >= output_frames_needed {
                break;
            }

            // Alimenta o WSOLA com o próximo bloco da track.
            // O bloco precisa ser grande o bastante para o WSOLA produzir um hop.
            let block_frames = {
                let stretcher = &self.key_lock_stretchers[deck_idx];
                // frame (2*hop) + search + folga
                (stretcher.hop * 4).max(1024)
            };

            let mut track_samples: Vec<f32> = Vec::with_capacity(block_frames);
            {
                let deck = &self.decks[deck_idx];
                let track = deck.track.as_ref().unwrap();
                let mut c = cursor;
                for _ in 0..block_frames {
                    if c >= track.samples.len() as f64 {
                        eof = true;
                        break;
                    }
                    track_samples.push(Self::interpolate_sample(&track.samples, c));
                    c += 1.0;

                    // Loop management (espelhando a lógica do caminho sem key lock).
                    if loop_enabled {
                        if loop_active {
                            if c >= loop_out {
                                c = loop_in;
                            }
                        } else if c >= loop_out {
                            c = loop_in;
                        }
                    }
                }
                cursor = c;
            }

            if track_samples.is_empty() {
                eof = true;
                break;
            }

            // DSP chain sobre o bloco (pré-estiramento).
            for s in track_samples.iter_mut() {
                *s *= trim as f32;
            }
            // filter/eq/gain operam sample-a-sample; o bloco e mono.
            self.filters[deck_idx].process(&mut track_samples);
            self.eqs[deck_idx].process(&mut track_samples);
            self.gains[deck_idx].process(&mut track_samples);

            // Envia ao WSOLA e tenta extrair output.
            self.key_lock_stretchers[deck_idx].push(&track_samples);

            let mut scratch = std::mem::take(&mut self.wsola_scratch);
            let produced = self.key_lock_stretchers[deck_idx].drain_to_fifo(&mut scratch);
            self.wsola_scratch = scratch;

            if !produced && eof {
                // Fim da track sem output: sinaliza EOF e para.
                break;
            }
        }

        // Avança o cursor do WSOLA com o que realmente foi lido da track.
        self.key_lock_stretchers[deck_idx].set_read_cursor(cursor);

        // --- 3. Escreve o output mono (1 amostra por frame) ---
        // Aplica fade-in nos primeiros samples ao entrar no key_lock para
        // evitar clique de descontinuidade (salto entre pitch_shift e WSOLA).
        // Interpola do ultimo_sample (callback anterior) para mono_out (novo modo).
        if self.decks[deck_idx].key_lock_fade > 0 {
            let ultimo = self.decks[deck_idx].ultimo_sample;
            for i in 0..output_frames_needed {
                let fade_pos = 1024usize.saturating_sub(self.decks[deck_idx].key_lock_fade) as f32;
                let fade = (fade_pos / 1024.0).clamp(0.0, 1.0);
                temp[i] = ultimo * (1.0 - fade) + mono_out[i] * fade;
                self.decks[deck_idx].key_lock_fade = self.decks[deck_idx].key_lock_fade.saturating_sub(1);
            }
        } else {
            temp[..output_frames_needed].copy_from_slice(&mono_out[..output_frames_needed]);
        }

        // --- 4. Atualiza src_index e estado de playing ---
        {
            let deck = &mut self.decks[deck_idx];
            // src_index reflete a posição na track original, avançando conforme o
            // output produzido pelo WSOLA ajustado pelo pitch (tempo).
            // Se pitch=0.5 (mais lento), cada frame de output consome 0.5 frame
            // de input; se pitch=2.0 (mais rápido), consome 2 frames.
            // Usar `cursor` (input alimentado) diretamente faria o áudio tocar
            // sempre na velocidade normal, ignorando o pitch -> picote/lentidão.
            let output_produzido = frames_ready as f64;
            let mut novo_src_index = src_index + output_produzido * pitch as f64;

            // Aplica wrap-around do loop se houver loop ativo
            if loop_enabled {
                let loop_span = loop_out - loop_in;
                if loop_span > 0.0 && novo_src_index >= loop_out {
                    let excesso = (novo_src_index - loop_out) % loop_span;
                    novo_src_index = loop_in + excesso;
                }
            }

            deck.src_index = novo_src_index;

            if eof && !loop_enabled && self.key_lock_stretchers[deck_idx].pending() == 0 {
                deck.is_playing = false;
                deck.src_index = track_len as f64;
                self.emit_track_ended(deck_idx);
            }
        }

        // Atualiza o ultimo sample produzido (usado para fade na proxima transicao)
        if output_frames_needed > 0 {
            self.decks[deck_idx].ultimo_sample = temp[output_frames_needed - 1];
        }

        self.update_vu(deck_idx, temp);
    }

    /// Renderiza o deck com KEY LOCK DESATIVADO: pitch shift via resampling.
    ///
    /// Usa interpolação linear com passo `pitch` (tom muda junto com a
    /// velocidade) — comportamento clássico de deck sem key lock.
    fn render_deck_pitch_shift(&mut self, deck_idx: usize, temp: &mut [f32]) {
        temp.fill(0.0);

        let mut track_ended = false;
        {
            let deck = &mut self.decks[deck_idx];
            if let Some(track) = &deck.track {
                if deck.is_playing {
                    let pitch = deck.pitch;

                    // Loop management
                    if deck.loop_enabled {
                        if deck.loop_active {
                            if deck.src_index >= deck.loop_out {
                                deck.src_index = deck.loop_in;
                            }
                        } else if deck.src_index >= deck.loop_out {
                            deck.loop_active = true;
                            deck.src_index = deck.loop_in;
                        }
                    }

                    for sample in temp.iter_mut() {
                        if deck.src_index >= track.samples.len() as f64 {
                            deck.is_playing = false;
                            deck.src_index = track.samples.len() as f64;
                            track_ended = true;
                            break;
                        }
                        *sample = Self::interpolate_sample(&track.samples, deck.src_index);
                        deck.src_index += pitch;
                    }

                    // Aplica trim (ganho linear)
                    for sample in temp.iter_mut() {
                        *sample *= deck.trim as f32;
                    }

                    // DSP: filter -> EQ -> gain
                    self.filters[deck_idx].process(temp);
                    self.eqs[deck_idx].process(temp);
                    self.gains[deck_idx].process(temp);
                }
            }
        }

        if track_ended {
            self.emit_track_ended(deck_idx);
        }

        // Atualiza o ultimo sample produzido (usado para fade na proxima transicao)
        if !temp.is_empty() {
            self.decks[deck_idx].ultimo_sample = temp[temp.len() - 1];
        }

        self.update_vu(deck_idx, temp);
    }

    fn render_deck(&mut self, deck_idx: usize, temp: &mut [f32]) {
        temp.fill(0.0);

        let has_track = self.decks[deck_idx].track.is_some();
        if !has_track {
            self.vu_levels[deck_idx] = 0.0;
            return;
        }

        let key_lock = self.decks[deck_idx].key_lock;
        let is_playing = self.decks[deck_idx].is_playing;

        if !is_playing {
            self.vu_levels[deck_idx] = 0.0;
            return;
        }

        if key_lock {
            self.render_deck_key_lock(deck_idx, temp);
        } else {
            self.render_deck_pitch_shift(deck_idx, temp);
        }

        // Atualiza key_lock_prev para detectar transição no próximo frame
        self.decks[deck_idx].key_lock_prev = key_lock;
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
