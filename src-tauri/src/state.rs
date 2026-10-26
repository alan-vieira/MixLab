use std::sync::{Arc, Mutex};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crate::audio::engine::AudioEngine;

pub struct AppState {
    pub engine: Arc<Mutex<AudioEngine>>,
    pub _stream: Option<cpal::Stream>, // Mantém o stream vivo
}

impl AppState {
    pub fn new() -> Self {
        let sample_rate = 44100;
        let engine = Arc::new(Mutex::new(AudioEngine::new(sample_rate)));
        
        let stream = Self::start_audio_stream(Arc::clone(&engine), sample_rate);
        
        Self {
            engine,
            _stream: stream,
        }
    }

    fn start_audio_stream(engine: Arc<Mutex<AudioEngine>>, _sample_rate: u32) -> Option<cpal::Stream> {
        let host = cpal::default_host();
        let device = host.default_output_device()?;
        let config = device.default_output_config().ok()?;
        
        // Removido o '&' daqui embaixo 👇
        let stream_config: cpal::StreamConfig = config.clone().into();
        
        let stream = device.build_output_stream(
            stream_config, // 👈 Sem o '&'
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                // Esta closure é chamada pelo Windows em tempo real!
                if let Ok(mut eng) = engine.lock() {
                    eng.fill_buffer(data);
                }
            },
            move |err| {
                eprintln!("Erro no stream de áudio CPAL: {}", err);
            },
            None,
        ).ok()?;
        
        // Inicia o stream imediatamente
        stream.play().ok()?;
        Some(stream)
    }
}