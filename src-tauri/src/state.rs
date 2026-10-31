use crate::audio::engine::AudioEngine;
use crate::audio::state::AudioThreadState;
use crate::database::AnalysisDatabase;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use parking_lot::Mutex;
use std::sync::Arc;

pub struct AppState {
    pub engine: Arc<Mutex<AudioEngine>>,
    pub db: Arc<Mutex<AnalysisDatabase>>,
    pub audio_thread_state: Arc<AudioThreadState>,
}

impl AppState {
    pub fn new() -> Self {
        let sample_rate = 44100;
        let audio_thread_state = Arc::new(AudioThreadState::new());

        let engine = Arc::new(Mutex::new(AudioEngine::new(
            sample_rate,
            Arc::clone(&audio_thread_state),
        )));

        let db = Arc::new(Mutex::new(
            AnalysisDatabase::new().expect("Falha ao criar banco de dados"),
        ));

        Self::start_audio_stream(Arc::clone(&engine), sample_rate);

        Self {
            engine,
            db,
            audio_thread_state,
        }
    }

    pub fn start_audio_stream(engine: Arc<Mutex<AudioEngine>>, _sample_rate: u32) {
        let host = cpal::default_host();
        let device = match host.default_output_device() {
            Some(d) => d,
            None => {
                eprintln!("Nenhum dispositivo de saída de áudio encontrado.");
                return;
            }
        };

        let config = match device.default_output_config() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Erro ao obter configuração de áudio: {}", e);
                return;
            }
        };
        let stream_config: cpal::StreamConfig = config.clone().into();

        let stream = match device.build_output_stream(
            &stream_config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                // parking_lot::Mutex::lock() é extremamente rápido e não retorna Result
                engine.lock().fill_buffer(data);
            },
            move |err| {
                eprintln!("Erro no stream de áudio CPAL: {}", err);
            },
            None,
        ) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Erro ao criar stream de áudio: {}", e);
                return;
            }
        };

        if let Err(e) = stream.play() {
            eprintln!("Erro ao iniciar stream de áudio: {}", e);
            return;
        }

        // O stream CPAL não implementa Send + Sync, então não pode ser armazenado
        // no AppState (que precisa ser Send + Sync para o Tauri). Usamos mem::forget
        // para manter o stream ativo durante todo o ciclo de vida do aplicativo.
        std::mem::forget(stream);
    }
}
