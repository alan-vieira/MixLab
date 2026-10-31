use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use atomic_float::AtomicF32;
use crossbeam::channel::{Receiver, Sender, bounded};
use std::sync::OnceLock;

// ============================================================================
// COMANDOS DISCRETOS UI -> AUDIO THREAD (Lock-Free)
// ============================================================================
#[derive(Debug, Clone)]
pub enum AudioCommand {
    Play { deck: DeckId },
    Pause { deck: DeckId },
    Stop { deck: DeckId },
    LoadTrack { path: String, deck: DeckId },
    Seek { time_secs: f64, deck: DeckId },
    SetTempo { multiplier: f32, deck: DeckId },
    SetGain { gain_db: f32, deck: DeckId },
    SetEq { low: f32, mid: f32, high: f32, deck: DeckId },
    SetFilter { position: f32, deck: DeckId },
    SetCrossfader { position: f32 },
    SetCuePoint { deck: DeckId },
    JumpToCue { deck: DeckId },
    JogScrub { delta: f32, deck: DeckId },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeckId {
    A,
    B,
}

// ============================================================================
// TELEMETRIA AUDIO -> UI (Lock-Free)
// ============================================================================
#[derive(Debug, Clone, Copy)]
pub struct DeckTelemetry {
    pub position: f32,
    pub vu_level: f32,
    pub is_playing: bool,
    pub pitch: f32,
    pub duration_secs: f64,
}

static DECK_A_TELEMETRY_TX_ONCE: OnceLock<Sender<DeckTelemetry>> = OnceLock::new();
static DECK_A_TELEMETRY_RX_ONCE: OnceLock<Receiver<DeckTelemetry>> = OnceLock::new();
static DECK_B_TELEMETRY_TX_ONCE: OnceLock<Sender<DeckTelemetry>> = OnceLock::new();
static DECK_B_TELEMETRY_RX_ONCE: OnceLock<Receiver<DeckTelemetry>> = OnceLock::new();

fn init_deck_channels() {
    let (tx_a, rx_a) = bounded(64);
    let (tx_b, rx_b) = bounded(64);
    let _ = DECK_A_TELEMETRY_TX_ONCE.set(tx_a);
    let _ = DECK_A_TELEMETRY_RX_ONCE.set(rx_a);
    let _ = DECK_B_TELEMETRY_TX_ONCE.set(tx_b);
    let _ = DECK_B_TELEMETRY_RX_ONCE.set(rx_b);
}

pub fn deck_a_telemetry_tx() -> &'static Sender<DeckTelemetry> {
    init_deck_channels();
    DECK_A_TELEMETRY_TX_ONCE.get().unwrap()
}
pub fn deck_a_telemetry_rx() -> &'static Receiver<DeckTelemetry> {
    init_deck_channels();
    DECK_A_TELEMETRY_RX_ONCE.get().unwrap()
}
pub fn deck_b_telemetry_tx() -> &'static Sender<DeckTelemetry> {
    init_deck_channels();
    DECK_B_TELEMETRY_TX_ONCE.get().unwrap()
}
pub fn deck_b_telemetry_rx() -> &'static Receiver<DeckTelemetry> {
    init_deck_channels();
    DECK_B_TELEMETRY_RX_ONCE.get().unwrap()
}

// ============================================================================
// ESTADO DA THREAD DE AUDIO (Lock-Free)
// ============================================================================
pub struct AudioThreadState {
    pub deck_a_playing: AtomicBool,
    pub deck_b_playing: AtomicBool,
    pub deck_a_tempo: AtomicF32,
    pub deck_b_tempo: AtomicF32,
    pub deck_a_gain: AtomicF32,
    pub deck_b_gain: AtomicF32,
    pub deck_a_eq_low: AtomicF32,
    pub deck_a_eq_mid: AtomicF32,
    pub deck_a_eq_high: AtomicF32,
    pub deck_b_eq_low: AtomicF32,
    pub deck_b_eq_mid: AtomicF32,
    pub deck_b_eq_high: AtomicF32,
    pub deck_a_filter: AtomicF32,
    pub deck_b_filter: AtomicF32,
    pub master_volume: AtomicF32,
    pub crossfader_pos: AtomicF32,
    pub deck_a_cue_point: AtomicF32,
    pub deck_b_cue_point: AtomicF32,
    pub deck_a_seek_target: AtomicU64,
    pub deck_b_seek_target: AtomicU64,
    pub command_rx: Receiver<AudioCommand>,
    pub command_tx: Sender<AudioCommand>,
    pub shutdown_requested: AtomicBool,
}

impl AudioThreadState {
    pub fn new() -> Self {
        let (cmd_tx, cmd_rx) = bounded(1024);
        Self {
            deck_a_playing: AtomicBool::new(false),
            deck_b_playing: AtomicBool::new(false),
            deck_a_tempo: AtomicF32::new(1.0),
            deck_b_tempo: AtomicF32::new(1.0),
            deck_a_gain: AtomicF32::new(1.0),
            deck_b_gain: AtomicF32::new(1.0),
            deck_a_eq_low: AtomicF32::new(0.0),
            deck_a_eq_mid: AtomicF32::new(0.0),
            deck_a_eq_high: AtomicF32::new(0.0),
            deck_b_eq_low: AtomicF32::new(0.0),
            deck_b_eq_mid: AtomicF32::new(0.0),
            deck_b_eq_high: AtomicF32::new(0.0),
            deck_a_filter: AtomicF32::new(0.5),
            deck_b_filter: AtomicF32::new(0.5),
            master_volume: AtomicF32::new(0.8),
            crossfader_pos: AtomicF32::new(0.5),
            deck_a_cue_point: AtomicF32::new(0.0),
            deck_b_cue_point: AtomicF32::new(0.0),
            deck_a_seek_target: AtomicU64::new((-1.0_f64).to_bits()),
            deck_b_seek_target: AtomicU64::new((-1.0_f64).to_bits()),
            command_rx: cmd_rx,
            command_tx: cmd_tx,
            shutdown_requested: AtomicBool::new(false),
        }
    }

    /// Sinaliza shutdown para limpeza ordenada. Chamado antes de encerrar o app.
    pub fn request_shutdown(&self) {
        self.shutdown_requested.store(true, Ordering::Relaxed);
    }

    pub fn is_shutdown_requested(&self) -> bool {
        self.shutdown_requested.load(Ordering::Relaxed)
    }

    pub fn process_commands(&self) {
        while let Ok(cmd) = self.command_rx.try_recv() {
            match cmd {
                AudioCommand::Play { deck } => match deck {
                    DeckId::A => self.deck_a_playing.store(true, Ordering::Relaxed),
                    DeckId::B => self.deck_b_playing.store(true, Ordering::Relaxed),
                },
                AudioCommand::Pause { deck } | AudioCommand::Stop { deck } => match deck {
                    DeckId::A => self.deck_a_playing.store(false, Ordering::Relaxed),
                    DeckId::B => self.deck_b_playing.store(false, Ordering::Relaxed),
                },
                AudioCommand::SetTempo { multiplier, deck } => {
                    let safe = multiplier.clamp(0.25, 4.0);
                    match deck {
                        DeckId::A => self.deck_a_tempo.store(safe, Ordering::Relaxed),
                        DeckId::B => self.deck_b_tempo.store(safe, Ordering::Relaxed),
                    }
                }
                AudioCommand::SetGain { gain_db, deck } => {
                    let linear = 10f32.powf(gain_db / 20.0);
                    match deck {
                        DeckId::A => self.deck_a_gain.store(linear, Ordering::Relaxed),
                        DeckId::B => self.deck_b_gain.store(linear, Ordering::Relaxed),
                    }
                }
                AudioCommand::SetEq { low, mid, high, deck } => match deck {
                    DeckId::A => {
                        self.deck_a_eq_low.store(low, Ordering::Relaxed);
                        self.deck_a_eq_mid.store(mid, Ordering::Relaxed);
                        self.deck_a_eq_high.store(high, Ordering::Relaxed);
                    }
                    DeckId::B => {
                        self.deck_b_eq_low.store(low, Ordering::Relaxed);
                        self.deck_b_eq_mid.store(mid, Ordering::Relaxed);
                        self.deck_b_eq_high.store(high, Ordering::Relaxed);
                    }
                },
                AudioCommand::SetFilter { position, deck } => {
                    let safe = position.clamp(0.0, 1.0);
                    match deck {
                        DeckId::A => self.deck_a_filter.store(safe, Ordering::Relaxed),
                        DeckId::B => self.deck_b_filter.store(safe, Ordering::Relaxed),
                    }
                }
                AudioCommand::SetCrossfader { position } => {
                    self.crossfader_pos.store(position.clamp(0.0, 1.0), Ordering::Relaxed);
                }
                AudioCommand::Seek { time_secs, deck } => {
                    match deck {
                        DeckId::A => self
                            .deck_a_seek_target
                            .store(time_secs.to_bits(), Ordering::Relaxed),
                        DeckId::B => self
                            .deck_b_seek_target
                            .store(time_secs.to_bits(), Ordering::Relaxed),
                    }
                }
                AudioCommand::LoadTrack { .. } => {} // Tratado diretamente em engine.load_track (criação de decoder thread)
                _ => {} // SetCuePoint, JumpToCue, JogScrub — aplicados fora (diretamente no engine)
            }
        }
    }

    /// Consome e retorna o target de seek pendente (se houver). Retorna -1.0 se não há seek pendente.
    pub fn take_deck_a_seek_target(&self) -> f64 {
        f64::from_bits(self.deck_a_seek_target.swap((-1.0_f64).to_bits(), Ordering::Relaxed))
    }

    pub fn take_deck_b_seek_target(&self) -> f64 {
        f64::from_bits(self.deck_b_seek_target.swap((-1.0_f64).to_bits(), Ordering::Relaxed))
    }
}