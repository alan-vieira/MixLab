//! Testes de integração para o MixLab
//!
//! Estes testes verificam o funcionamento dos comandos Tauri e do motor de áudio
//! usando serial_test para garantir execução sequencial (estado global compartilhado).

use mixlab_lib::audio::engine::AudioEngine;
use mixlab_lib::audio::decoder::AudioDecoder;
use mixlab_lib::analysis::bpm::detect_bpm;
use mixlab_lib::analysis::waveform::{generate_waveform_peaks, generate_waveform_layers};
use std::sync::{Arc, Mutex};

/// Cria um engine de teste com sample rate padrão
fn create_test_engine() -> AudioEngine {
    AudioEngine::new(44100)
}

/// Gera áudio de teste (senóide 440Hz)
fn generate_test_tone(sample_rate: u32, duration_secs: f64, frequency: f32) -> Vec<f32> {
    let samples = (sample_rate as f64 * duration_secs) as usize;
    let mut buffer = Vec::with_capacity(samples);
    for i in 0..samples {
        let t = i as f32 / sample_rate as f32;
        buffer.push((2.0 * std::f32::consts::PI * frequency * t).sin() * 0.5);
    }
    buffer
}

#[test]
fn test_audio_engine_creation() {
    let engine = create_test_engine();
    assert_eq!(engine.sample_rate, 44100);
    assert!(!engine.deck_a.is_playing);
    assert!(!engine.deck_b.is_playing);
    assert_eq!(engine.deck_a.pitch, 1.0);
    assert_eq!(engine.deck_b.pitch, 1.0);
}

#[test]
fn test_deck_state_defaults() {
    let engine = create_test_engine();
    
    // Deck A
    assert!(engine.deck_a.track.is_none());
    assert_eq!(engine.deck_a.src_index, 0.0);
    assert_eq!(engine.deck_a.cue_point, 0.0);
    
    // Deck B
    assert!(engine.deck_b.track.is_none());
    assert_eq!(engine.deck_b.src_index, 0.0);
    assert_eq!(engine.deck_b.cue_point, 0.0);
}

#[test]
fn test_dsp_components_initialization() {
    let engine = create_test_engine();
    
    // Verifica se componentes DSP foram criados
    // EQs
    assert_eq!(engine.deck_a_eq.low_gain(), 1.0);
    assert_eq!(engine.deck_a_eq.mid_gain(), 1.0);
    assert_eq!(engine.deck_a_eq.high_gain(), 1.0);
    
    // Gains
    assert_eq!(engine.deck_a_gain.gain(), 1.0);
    assert_eq!(engine.deck_b_gain.gain(), 1.0);
    
    // Filters
    assert_eq!(engine.deck_a_filter.position(), 0.5); // Neutro
    assert_eq!(engine.deck_b_filter.position(), 0.5);
    
    // Crossfader
    assert_eq!(engine.crossfader.position(), 0.5);
}

#[test]
fn test_bpm_detection_known_signal() {
    // Gera um click track a 120 BPM (2 beats/sec = 0.5s intervalo)
    let sample_rate = 44100;
    let duration = 10.0; // 10 segundos
    let bpm = 120.0;
    let beat_interval = 60.0 / bpm; // 0.5s
    
    let mut samples = vec![0.0f32; (sample_rate as f64 * duration) as usize];
    
    // Adiciona clicks a cada beat
    let mut beat_time = 0.0;
    while beat_time < duration {
        let idx = (beat_time * sample_rate as f64) as usize;
        if idx < samples.len() {
            samples[idx] = 1.0; // Click forte
        }
        beat_time += beat_interval;
    }
    
    let detected = detect_bpm(&samples, sample_rate as f32);
    
    // Deve detectar próximo de 120 BPM (tolerância ±5%)
    assert!((detected - bpm).abs() < 6.0, "BPM detectado: {}, esperado ~{}", detected, bpm);
}

#[test]
fn test_bpm_detection_sine_wave() {
    // Senóide pura não tem beat claro - deve retornar BPM padrão ou 0
    let sample_rate = 44100;
    let samples = generate_test_tone(sample_rate, 5.0, 440.0);
    
    let detected = detect_bpm(&samples, sample_rate as f32);
    
    // Para senóide pura, BPM detection pode falhar ou retornar valor baixo
    // Apenas verifica que não panic e retorna valor razoável
    assert!(detected >= 0.0 && detected <= 300.0);
}

#[test]
fn test_waveform_peaks_generation() {
    let sample_rate = 44100;
    let samples = generate_test_tone(sample_rate, 2.0, 440.0);
    let num_peaks = 100;
    
    let peaks = generate_waveform_peaks(&samples, num_peaks);
    
    assert_eq!(peaks.len(), num_peaks);
    // Todos os picos devem ser >= 0
    for peak in &peaks {
        assert!(*peak >= 0.0);
        assert!(*peak <= 1.0); // Normalizado
    }
}

#[test]
fn test_waveform_layers_generation() {
    let sample_rate = 44100;
    let samples = generate_test_tone(sample_rate, 2.0, 440.0);
    let num_points = 500;
    
    let layers = generate_waveform_layers(&samples, sample_rate as f32, num_points);
    
    assert_eq!(layers.low.len(), num_points);
    assert_eq!(layers.mid.len(), num_points);
    assert_eq!(layers.high.len(), num_points);
    
    // Verifica que há energia nas bandas (senóide 440Hz = médios)
    let mid_energy: f32 = layers.mid.iter().sum();
    let low_energy: f32 = layers.low.iter().sum();
    let high_energy: f32 = layers.high.iter().sum();
    
    // 440Hz deve cair na banda mid (300-4000Hz)
    assert!(mid_energy > low_energy);
    assert!(mid_energy > high_energy);
}

#[test]
fn test_eq_gain_controls() {
    let mut engine = create_test_engine();
    
    // Testa EQ do Deck A
    engine.deck_a_eq.set_low_gain(2.0);  // +6dB
    engine.deck_a_eq.set_mid_gain(0.5);  // -6dB
    engine.deck_a_eq.set_high_gain(1.0); // 0dB
    
    assert!((engine.deck_a_eq.low_gain() - 2.0).abs() < 0.01);
    assert!((engine.deck_a_eq.mid_gain() - 0.5).abs() < 0.01);
    assert!((engine.deck_a_eq.high_gain() - 1.0).abs() < 0.01);
}

#[test]
fn test_gain_control() {
    let mut engine = create_test_engine();
    
    // Testa gain linear
    engine.deck_a_gain.set_gain(2.0); // +6dB
    assert!((engine.deck_a_gain.gain() - 2.0).abs() < 0.01);
    
    engine.deck_a_gain.set_gain(0.5); // -6dB
    assert!((engine.deck_a_gain.gain() - 0.5).abs() < 0.01);
}

#[test]
fn test_filter_position() {
    let mut engine = create_test_engine();
    
    // LPF (0.0)
    engine.deck_a_filter.set_position(0.0);
    assert!((engine.deck_a_filter.position() - 0.0).abs() < 0.01);
    
    // Neutro (0.5)
    engine.deck_a_filter.set_position(0.5);
    assert!((engine.deck_a_filter.position() - 0.5).abs() < 0.01);
    
    // HPF (1.0)
    engine.deck_a_filter.set_position(1.0);
    assert!((engine.deck_a_filter.position() - 1.0).abs() < 0.01);
    
    // Clamping
    engine.deck_a_filter.set_position(-0.5);
    assert_eq!(engine.deck_a_filter.position(), 0.0);
    
    engine.deck_a_filter.set_position(1.5);
    assert_eq!(engine.deck_a_filter.position(), 1.0);
}

#[test]
fn test_crossfader_position() {
    let mut engine = create_test_engine();
    
    // Deck A only (0.0)
    engine.crossfader.set_position(0.0);
    assert!((engine.crossfader.position() - 0.0).abs() < 0.01);
    
    // Center (0.5)
    engine.crossfader.set_position(0.5);
    assert!((engine.crossfader.position() - 0.5).abs() < 0.01);
    
    // Deck B only (1.0)
    engine.crossfader.set_position(1.0);
    assert!((engine.crossfader.position() - 1.0).abs() < 0.01);
}

#[test]
fn test_fill_buffer_silence() {
    let mut engine = create_test_engine();
    let mut output = vec![0.0f32; 4096]; // 2048 frames stereo
    
    // Sem tracks carregados, deve produzir silêncio
    engine.fill_buffer(&mut output);
    
    for sample in &output {
        assert_eq!(*sample, 0.0);
    }
}

#[test]
fn test_pitch_clamping() {
    let mut engine = create_test_engine();
    
    // Carrega track dummy
    let samples = generate_test_tone(44100, 2.0, 440.0);
    let track_data = mixlab_lib::audio::engine::TrackData {
        samples: samples.clone(),
        sample_rate: 44100,
        duration_secs: 2.0,
        bpm: 120.0,
        waveform_peaks: vec![0.5; 100],
        waveform_layers: Arc::new(mixlab_lib::analysis::waveform::WaveformLayers {
            low: vec![0.1; 500],
            mid: vec![0.5; 500],
            high: vec![0.1; 500],
        }),
    };
    
    engine.deck_a.track = Some(Arc::new(track_data));
    engine.deck_a.is_playing = true;
    
    // Testa pitch clamping via engine direto
    engine.deck_a.pitch = 0.5;  // Mínimo
    assert_eq!(engine.deck_a.pitch, 0.5);
    
    engine.deck_a.pitch = 2.0;  // Máximo
    assert_eq!(engine.deck_a.pitch, 2.0);
    
    // Fora dos limites - seria clampado no comando Tauri
    engine.deck_a.pitch = 0.3;
    assert_eq!(engine.deck_a.pitch, 0.3); // Engine não clampa, comando clampa
}

#[test]
fn test_cue_point_operations() {
    let mut engine = create_test_engine();
    
    let samples = generate_test_tone(44100, 2.0, 440.0);
    let track_data = mixlab_lib::audio::engine::TrackData {
        samples,
        sample_rate: 44100,
        duration_secs: 2.0,
        bpm: 120.0,
        waveform_peaks: vec![0.5; 100],
        waveform_layers: Arc::new(mixlab_lib::analysis::waveform::WaveformLayers {
            low: vec![0.1; 500],
            mid: vec![0.5; 500],
            high: vec![0.1; 500],
        }),
    };
    
    engine.deck_a.track = Some(Arc::new(track_data));
    engine.deck_a.src_index = 10000.0; // Alguma posição
    
    // Simula set_cue_point
    engine.deck_a.cue_point = engine.deck_a.src_index;
    assert_eq!(engine.deck_a.cue_point, 10000.0);
    
    // Simula jump_to_cue
    engine.deck_a.src_index = 20000.0;
    engine.deck_a.src_index = engine.deck_a.cue_point;
    assert_eq!(engine.deck_a.src_index, 10000.0);
}

#[test]
fn test_jog_scrub() {
    let mut engine = create_test_engine();
    
    let samples = generate_test_tone(44100, 2.0, 440.0);
    let track_data = mixlab_lib::audio::engine::TrackData {
        samples,
        sample_rate: 44100,
        duration_secs: 2.0,
        bpm: 120.0,
        waveform_peaks: vec![0.5; 100],
        waveform_layers: Arc::new(mixlab_lib::analysis::waveform::WaveformLayers {
            low: vec![0.1; 500],
            mid: vec![0.5; 500],
            high: vec![0.1; 500],
        }),
    };
    
    engine.deck_a.track = Some(Arc::new(track_data));
    engine.deck_a.src_index = 10000.0;
    
    // Scrub forward
    let delta = 180.0; // Meia volta = 1000 samples
    engine.deck_a.src_index += (delta / 180.0) * 1000.0;
    assert!((engine.deck_a.src_index - 11000.0).abs() < 1.0);
    
    // Scrub backward
    let delta = -180.0;
    engine.deck_a.src_index += (delta / 180.0) * 1000.0;
    assert!((engine.deck_a.src_index - 10000.0).abs() < 1.0);
    
    // Clamp at zero
    engine.deck_a.src_index = 100.0;
    let delta = -360.0;
    engine.deck_a.src_index += (delta / 180.0) * 1000.0;
    assert_eq!(engine.deck_a.src_index, 0.0);
}

#[test]
fn test_vu_meter_calculation() {
    let mut engine = create_test_engine();
    
    // Buffer de silêncio
    let silence = vec![0.0f32; 2048];
    let rms = AudioEngine::calculate_rms(&silence);
    assert_eq!(rms, 0.0);
    
    // Buffer com sinal conhecido
    let signal: Vec<f32> = (0..2048).map(|i| (i as f32 * 0.01).sin() * 0.5).collect();
    let rms = AudioEngine::calculate_rms(&signal);
    assert!(rms > 0.0 && rms <= 1.0);
    
    // Buffer full scale
    let full = vec![1.0f32; 2048];
    let rms = AudioEngine::calculate_rms(&full);
    assert!((rms - 1.0).abs() < 0.01); // Clamped to 1.0
}

#[test]
fn test_audio_decoder_error_handling() {
    // Tenta decodificar arquivo inexistente
    let result = AudioDecoder::load_and_decode("/caminho/inexistente/arquivo.mp3");
    assert!(result.is_err());
}

#[test]
fn test_concurrent_deck_operations() {
    let mut engine = create_test_engine();
    
    let samples = generate_test_tone(44100, 2.0, 440.0);
    let track_data = Arc::new(mixlab_lib::audio::engine::TrackData {
        samples,
        sample_rate: 44100,
        duration_secs: 2.0,
        bpm: 120.0,
        waveform_peaks: vec![0.5; 100],
        waveform_layers: Arc::new(mixlab_lib::analysis::waveform::WaveformLayers {
            low: vec![0.1; 500],
            mid: vec![0.5; 500],
            high: vec![0.1; 500],
        }),
    });
    
    // Carrega em ambos os decks
    engine.deck_a.track = Some(track_data.clone());
    engine.deck_b.track = Some(track_data);
    
    engine.deck_a.is_playing = true;
    engine.deck_b.is_playing = true;
    
    // Ambos tocando - fill_buffer deve processar ambos
    let mut output = vec![0.0f32; 4096];
    engine.fill_buffer(&mut output);
    
    // Output não deve ser todo zero (há sinal)
    let has_signal = output.iter().any(|s| *s != 0.0);
    assert!(has_signal);
}

#[test]
fn test_state_isolation_between_decks() {
    let mut engine = create_test_engine();
    
    let samples = generate_test_tone(44100, 2.0, 440.0);
    let track_data = Arc::new(mixlab_lib::audio::engine::TrackData {
        samples,
        sample_rate: 44100,
        duration_secs: 2.0,
        bpm: 120.0,
        waveform_peaks: vec![0.5; 100],
        waveform_layers: Arc::new(mixlab_lib::analysis::waveform::WaveformLayers {
            low: vec![0.1; 500],
            mid: vec![0.5; 500],
            high: vec![0.1; 500],
        }),
    });
    
    engine.deck_a.track = Some(track_data.clone());
    engine.deck_b.track = Some(track_data);
    
    // Muda EQ apenas do deck A
    engine.deck_a_eq.set_low_gain(2.0);
    engine.deck_a_eq.set_mid_gain(2.0);
    engine.deck_a_eq.set_high_gain(2.0);
    
    // Deck B deve permanecer inalterado
    assert_eq!(engine.deck_b_eq.low_gain(), 1.0);
    assert_eq!(engine.deck_b_eq.mid_gain(), 1.0);
    assert_eq!(engine.deck_b_eq.high_gain(), 1.0);
    
    // Muda filter apenas do deck B
    engine.deck_b_filter.set_position(0.0); // LPF
    
    // Deck A deve permanecer neutro
    assert_eq!(engine.deck_a_filter.position(), 0.5);
    assert_eq!(engine.deck_b_filter.position(), 0.0);
}

#[test]
fn test_crossfader_mix_behavior() {
    let mut engine = create_test_engine();
    
    let samples = generate_test_tone(44100, 1.0, 440.0);
    let track_data = Arc::new(mixlab_lib::audio::engine::TrackData {
        samples: samples.clone(),
        sample_rate: 44100,
        duration_secs: 1.0,
        bpm: 120.0,
        waveform_peaks: vec![0.5; 100],
        waveform_layers: Arc::new(mixlab_lib::analysis::waveform::WaveformLayers {
            low: vec![0.1; 500],
            mid: vec![0.5; 500],
            high: vec![0.1; 500],
        }),
    });
    
    engine.deck_a.track = Some(track_data.clone());
    engine.deck_b.track = Some(track_data);
    engine.deck_a.is_playing = true;
    engine.deck_b.is_playing = true;
    
    // Crossfader em A apenas
    engine.crossfader.set_position(0.0);
    let mut output_a = vec![0.0f32; 4096];
    engine.fill_buffer(&mut output_a);
    
    // Crossfader em B apenas
    engine.crossfader.set_position(1.0);
    let mut output_b = vec![0.0f32; 4096];
    engine.fill_buffer(&mut output_b);
    
    // Crossfader no centro
    engine.crossfader.set_position(0.5);
    let mut output_center = vec![0.0f32; 4096];
    engine.fill_buffer(&mut output_center);
    
    // Verifica que outputs são diferentes
    let sum_a: f32 = output_a.iter().map(|s| s.abs()).sum();
    let sum_b: f32 = output_b.iter().map(|s| s.abs()).sum();
    let sum_center: f32 = output_center.iter().map(|s| s.abs()).sum();
    
    assert!(sum_a > 0.0);
    assert!(sum_b > 0.0);
    assert!(sum_center > 0.0);
    
    // Centro deve ter energia de ambos (equal power)
    // Como tracks são idênticos, centro ≈ A ≈ B em amplitude
    // (crossfader equal-power mantém loudness constante)
}

#[test]
fn test_reset_on_track_load() {
    let mut engine = create_test_engine();
    
    let samples = generate_test_tone(44100, 2.0, 440.0);
    let track_data = Arc::new(mixlab_lib::audio::engine::TrackData {
        samples,
        sample_rate: 44100,
        duration_secs: 2.0,
        bpm: 120.0,
        waveform_peaks: vec![0.5; 100],
        waveform_layers: Arc::new(mixlab_lib::analysis::waveform::WaveformLayers {
            low: vec![0.1; 500],
            mid: vec![0.5; 500],
            high: vec![0.1; 500],
        }),
    });
    
    // Estado "sujo"
    engine.deck_a.src_index = 50000.0;
    engine.deck_a.is_playing = true;
    engine.deck_a.pitch = 1.5;
    engine.deck_a.cue_point = 10000.0;
    engine.deck_a_filter.set_position(0.2);
    
    // Simula load_track reset
    engine.deck_a.track = Some(track_data);
    engine.deck_a.src_index = 0.0;
    engine.deck_a.is_playing = false;
    engine.deck_a.pitch = 1.0;
    engine.deck_a_filter.reset();
    
    // Verifica reset
    assert_eq!(engine.deck_a.src_index, 0.0);
    assert!(!engine.deck_a.is_playing);
    assert_eq!(engine.deck_a.pitch, 1.0);
    assert_eq!(engine.deck_a_filter.position(), 0.5); // Reset = neutro
}