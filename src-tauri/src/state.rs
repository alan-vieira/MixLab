use crate::audio::engine::AudioEngine;
use crate::audio::dsp::eq::Equalizer;
use crate::audio::dsp::filter::Filter;
use crate::database::AnalysisDatabase;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};

pub struct AppState {
    pub engine: Arc<Mutex<AudioEngine>>,
    pub db: Arc<Mutex<AnalysisDatabase>>,
}

impl AppState {
    pub fn new() -> Self {
        let sample_rate = 44100;
        let engine = Arc::new(Mutex::new(AudioEngine::new(sample_rate)));
        let db = Arc::new(Mutex::new(
            AnalysisDatabase::new().expect("Falha ao criar banco de dados"),
        ));

        // Inicia o stream de áudio. Usamos mem::forget para mantê-lo vivo
        // sem violar as regras de Send/Sync do Tauri (cpal::Stream não é Send em todas as plataformas)
        Self::start_audio_stream(Arc::clone(&engine), sample_rate);

        Self { engine, db }
    }

    fn start_audio_stream(engine: Arc<Mutex<AudioEngine>>, _sample_rate: u32) {
            let host = cpal::default_host();
            let device = match host.default_output_device() {
                Some(d) => d,
                None => {
                    eprintln!("Nenhum dispositivo de saída de áudio encontrado.");
                    return;
                }
            };

            // Usa sample rate do dispositivo para evitar resampling e pitch shift
            let config = match device.default_output_config() {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Erro ao obter configuração de áudio: {}", e);
                    return;
                }
            };

            let device_sample_rate = config.sample_rate().0;
            let device_channels = config.channels();

            // 🔧 Buffer menor = menor latência (512 frames)
            let stream_config = cpal::StreamConfig {
                channels: device_channels,
                sample_rate: cpal::SampleRate(device_sample_rate),
                buffer_size: cpal::BufferSize::Fixed(512),
            };

            // Recria engine com sample rate correto do dispositivo
            {
                let mut eng = engine.lock().unwrap();
                eng.sample_rate = device_sample_rate;
                eng.deck_a_eq = Equalizer::new(device_sample_rate as f32);
                eng.deck_b_eq = Equalizer::new(device_sample_rate as f32);
                eng.deck_a_filter = Filter::new(device_sample_rate as f32);
                eng.deck_b_filter = Filter::new(device_sample_rate as f32);
            }

            let stream = match device.build_output_stream(
                &stream_config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    if let Ok(mut eng) = engine.lock() {
                        eng.fill_buffer(data);
                    }
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

            std::mem::forget(stream);
        }
}
