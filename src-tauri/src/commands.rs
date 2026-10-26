use tauri::State;
use serde::Serialize;
use std::sync::Arc;
use crate::state::AppState;
use crate::audio::decoder::AudioDecoder;
use crate::analysis::waveform::{generate_waveform_peaks, generate_waveform_layers};
use crate::analysis::bpm::detect_bpm;
use crate::audio::engine::TrackData;

#[derive(Serialize)]
pub struct TrackInfo {
    pub duration_secs: f64,
    pub bpm: f32,
}

#[tauri::command]
pub async fn load_track(state: State<'_, AppState>, deck: String, path: String) -> Result<TrackInfo, String> {
    let deck_char = deck.chars().next().unwrap_or('A');
    let path_for_task = path.clone();

    // 1. Trabalho pesado (decodificação, BPM, waveform) em thread separada 
    // Usando o runtime assíncrono nativo do Tauri (sem precisar do crate tokio)
    let track_data = tauri::async_runtime::spawn_blocking(move || {
        let (samples, sr, _ch, dur) = AudioDecoder::load_and_decode(&path_for_task)
            .map_err(|e| format!("Falha ao decodificar: {}", e))?;
        
        let bpm = detect_bpm(&samples, sr as f32);
        let peaks = generate_waveform_peaks(&samples, 1000);
        let layers = generate_waveform_layers(&samples, sr as f32, 5000);
        
        Ok::<_, String>(TrackData {
            samples,
            sample_rate: sr,
            duration_secs: dur,
            bpm,
            waveform_peaks: peaks,
            waveform_layers: Arc::new(layers),
        })
    })
    .await
    .map_err(|e| format!("Erro na task de carregamento: {}", e))??;

    let duration_secs = track_data.duration_secs;
    let bpm = track_data.bpm;
    let track_arc = Arc::new(track_data);

    // 2. Bloqueia o motor de áudio POR MILISSEGUNDOS apenas para trocar a faixa
    let mut engine = state.engine.lock().map_err(|e| e.to_string())?;
    
    if deck_char == 'A' {
        engine.deck_a.track = Some(track_arc);
        engine.deck_a.src_index = 0.0;
        engine.deck_a.is_playing = false;
        engine.deck_a.pitch = 1.0;
        engine.deck_a.cue_point = 0.0;
        engine.deck_a_filter.reset();
    } else {
        engine.deck_b.track = Some(track_arc);
        engine.deck_b.src_index = 0.0;
        engine.deck_b.is_playing = false;
        engine.deck_b.pitch = 1.0;
        engine.deck_b.cue_point = 0.0;
        engine.deck_b_filter.reset();
    }
    
    Ok(TrackInfo { duration_secs, bpm })
}

#[tauri::command]
pub fn play_deck(state: State<'_, AppState>, deck: String) -> Result<(), String> {
    let deck_char = deck.chars().next().unwrap_or('A');
    let mut engine = state.engine.lock().map_err(|e| e.to_string())?;
    if deck_char == 'A' {
        engine.deck_a.is_playing = true;
    } else {
        engine.deck_b.is_playing = true;
    }
    Ok(())
}

#[tauri::command]
pub fn pause_deck(state: State<'_, AppState>, deck: String) -> Result<(), String> {
    let deck_char = deck.chars().next().unwrap_or('A');
    let mut engine = state.engine.lock().map_err(|e| e.to_string())?;
    if deck_char == 'A' {
        engine.deck_a.is_playing = false;
    } else {
        engine.deck_b.is_playing = false;
    }
    Ok(())
}

#[tauri::command]
pub fn get_waveform_peaks(state: State<'_, AppState>, deck: String) -> Result<Vec<f32>, String> {
    let deck_char = deck.chars().next().unwrap_or('A');
    let engine = state.engine.lock().map_err(|e| e.to_string())?;
    
    let track = if deck_char == 'A' { &engine.deck_a.track } else { &engine.deck_b.track };
    
    if let Some(track) = track {
        Ok(track.waveform_peaks.clone())
    } else {
        Ok(vec![])
    }
}

#[tauri::command]
pub fn get_waveform_layers(state: State<'_, AppState>, deck: String) -> Result<(Vec<f32>, Vec<f32>, Vec<f32>), String> {
    let deck_char = deck.chars().next().unwrap_or('A');
    let engine = state.engine.lock().map_err(|e| e.to_string())?;
    
    let track = if deck_char == 'A' { &engine.deck_a.track } else { &engine.deck_b.track };
    
    if let Some(track) = track {
        Ok((
            track.waveform_layers.low.clone(),
            track.waveform_layers.mid.clone(),
            track.waveform_layers.high.clone(),
        ))
    } else {
        Ok((vec![], vec![], vec![]))
    }
}

#[tauri::command]
pub fn get_playback_position(state: State<'_, AppState>, deck: String) -> Result<f64, String> {
    let deck_char = deck.chars().next().unwrap_or('A');
    let engine = state.engine.lock().map_err(|e| e.to_string())?;
    
    let deck_state = if deck_char == 'A' { &engine.deck_a } else { &engine.deck_b };
    
    if let Some(track) = &deck_state.track {
        let position = deck_state.src_index as f64 / track.samples.len() as f64;
        Ok(position)
    } else {
        Ok(0.0)
    }
}

#[tauri::command]
pub fn set_deck_eq(state: State<'_, AppState>, deck: String, low: f32, mid: f32, high: f32) -> Result<(), String> {
    let deck_char = deck.chars().next().unwrap_or('A');
    let mut engine = state.engine.lock().map_err(|e| e.to_string())?;
    
    if deck_char == 'A' {
        engine.deck_a_eq.set_low_gain(low);
        engine.deck_a_eq.set_mid_gain(mid);
        engine.deck_a_eq.set_high_gain(high);
    } else {
        engine.deck_b_eq.set_low_gain(low);
        engine.deck_b_eq.set_mid_gain(mid);
        engine.deck_b_eq.set_high_gain(high);
    }
    Ok(())
}

#[tauri::command]
pub fn set_deck_gain(state: State<'_, AppState>, deck: String, gain_db: f32) -> Result<(), String> {
    let deck_char = deck.chars().next().unwrap_or('A');
    let mut engine = state.engine.lock().map_err(|e| e.to_string())?;
    
    let linear_gain = 10f32.powf(gain_db / 20.0);
    
    if deck_char == 'A' {
        engine.deck_a_gain.set_gain(linear_gain);
    } else {
        engine.deck_b_gain.set_gain(linear_gain);
    }
    Ok(())
}

#[tauri::command]
pub fn set_crossfader(state: State<'_, AppState>, position: f32) -> Result<(), String> {
    let mut engine = state.engine.lock().map_err(|e| e.to_string())?;
    let normalized = (position / 100.0).clamp(0.0, 1.0);
    engine.crossfader.set_position(normalized);
    Ok(())
}

#[tauri::command]
pub fn set_deck_pitch(state: State<'_, AppState>, deck: String, pitch: f32) -> Result<(), String> {
    let deck_char = deck.chars().next().unwrap_or('A');
    let mut engine = state.engine.lock().map_err(|e| e.to_string())?;
    
    let safe_pitch = pitch.clamp(0.5, 2.0);
    if deck_char == 'A' { engine.deck_a.pitch = safe_pitch; } 
    else { engine.deck_b.pitch = safe_pitch; }
    Ok(())
}

#[tauri::command]
pub fn set_cue_point(state: State<'_, AppState>, deck: String) -> Result<(), String> {
    let deck_char = deck.chars().next().unwrap_or('A');
    let mut engine = state.engine.lock().map_err(|e| e.to_string())?;
    
    if deck_char == 'A' { engine.deck_a.cue_point = engine.deck_a.src_index; } 
    else { engine.deck_b.cue_point = engine.deck_b.src_index; }
    Ok(())
}

#[tauri::command]
pub fn jump_to_cue(state: State<'_, AppState>, deck: String) -> Result<(), String> {
    let deck_char = deck.chars().next().unwrap_or('A');
    let mut engine = state.engine.lock().map_err(|e| e.to_string())?;
    
    if deck_char == 'A' { engine.deck_a.src_index = engine.deck_a.cue_point; } 
    else { engine.deck_b.src_index = engine.deck_b.cue_point; }
    Ok(())
}

#[tauri::command]
pub fn jog_scrub(state: State<'_, AppState>, deck: String, delta: f32) -> Result<(), String> {
    let deck_char = deck.chars().next().unwrap_or('A');
    let mut engine = state.engine.lock().map_err(|e| e.to_string())?;
    
    let scrub_amount = delta * 1000.0;
    if deck_char == 'A' {
        engine.deck_a.src_index = (engine.deck_a.src_index + scrub_amount).max(0.0);
    } else {
        engine.deck_b.src_index = (engine.deck_b.src_index + scrub_amount).max(0.0);
    }
    Ok(())
}

#[tauri::command]
pub fn set_deck_filter(state: State<'_, AppState>, deck: String, position: f32) -> Result<(), String> {
    let deck_char = deck.chars().next().unwrap_or('A');
    let mut engine = state.engine.lock().map_err(|e| e.to_string())?;
    
    let safe_position = position.clamp(0.0, 1.0);
    if deck_char == 'A' { engine.deck_a_filter.set_position(safe_position); } 
    else { engine.deck_b_filter.set_position(safe_position); }
    Ok(())
}

#[tauri::command]
pub fn get_vu_levels(state: State<'_, AppState>) -> Result<(f32, f32), String> {
    let engine = state.engine.lock().map_err(|e| e.to_string())?;
    Ok((engine.vu_level_a, engine.vu_level_b))
}