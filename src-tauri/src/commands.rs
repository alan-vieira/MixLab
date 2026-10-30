use crate::analysis::bpm::detect_bpm;
use crate::analysis::waveform::{
    generate_full_waveform, generate_waveform_layers, generate_waveform_peaks,
};
use crate::audio::decoder::AudioDecoder;
use crate::audio::engine::TrackData;
use crate::state::AppState;
use serde::Serialize;
use std::sync::Arc;
use tauri::{Emitter, Manager, State};

/// Camadas de waveform espectral (low, mid, high)
#[allow(dead_code)]
type WaveformLayersResult = (Vec<f32>, Vec<f32>, Vec<f32>);

#[derive(Serialize, Clone)]
pub struct TrackInfo {
    pub duration_secs: f64,
    pub bpm: f32,
}

#[derive(Serialize, Clone)]
struct AnalysisCompletePayload {
    deck: String,
    bpm: f32,
}

/// Status unificado do deck para sincronização perfeita (posição + VU + estado)
#[derive(Serialize)]
pub struct DeckStatus {
    pub position: f32, // Fração 0.0 a 1.0
    pub vu_level: f32, // Nível RMS do fill_buffer
    pub is_playing: bool,
}

#[tauri::command]
pub async fn load_track(
    state: State<'_, AppState>,
    deck: String,
    path: String,
    app: tauri::AppHandle,
) -> Result<TrackInfo, String> {
    let deck_char = deck.chars().next().unwrap_or('A');
    let path_for_task = path.clone();
    let deck_for_event = deck.clone();

    // FASE 1: Decodificação em background
    let track_data = tauri::async_runtime::spawn_blocking(move || {
        AudioDecoder::load_and_decode(&path_for_task)
            .map_err(|e| format!("Falha ao decodificar: {}", e))
    })
    .await
    .map_err(|e| format!("Erro na decodificação: {}", e))??;

    let (samples, sr, _channels, dur) = track_data;

    // FASE 2: Análise em background (NÃO TRAVA UI)
    let app_for_analysis = app.clone();
    let path_for_analysis = path.clone();
    let samples_for_analysis = samples.clone();
    let sr_for_analysis = sr;
    let dur_for_analysis = dur;

    let (bpm, peaks, full_waveform, layers) = tauri::async_runtime::spawn_blocking(move || {
        let state = app_for_analysis.state::<AppState>();
        let db = state.db.lock().map_err(|e| e.to_string())?;

        if let Ok(Some(cached)) = db.get_cached(&path_for_analysis) {
            Ok((
                cached.bpm,
                cached.waveform_peaks,
                crate::analysis::waveform::FullWaveform {
                    min: cached.waveform_full_min,
                    max: cached.waveform_full_max,
                },
                crate::analysis::waveform::WaveformLayers {
                    low: cached.waveform_low,
                    mid: cached.waveform_mid,
                    high: cached.waveform_high,
                },
            ))
        } else {
            let bpm = detect_bpm(&samples_for_analysis, sr_for_analysis as f32);
            let peaks = generate_waveform_peaks(&samples_for_analysis, 1000);
            let full_waveform = generate_full_waveform(&samples_for_analysis, 1000);
            let layers =
                generate_waveform_layers(&samples_for_analysis, sr_for_analysis as f32, 1000);

            let _ = db.save_analysis(
                &path_for_analysis,
                &crate::database::CachedAnalysis {
                    bpm,
                    duration_secs: dur_for_analysis,
                    waveform_peaks: peaks.clone(),
                    waveform_full_min: full_waveform.min.clone(),
                    waveform_full_max: full_waveform.max.clone(),
                    waveform_low: layers.low.clone(),
                    waveform_mid: layers.mid.clone(),
                    waveform_high: layers.high.clone(),
                },
            );

            Ok((bpm, peaks, full_waveform, layers))
        }
    })
    .await
    .map_err(|e| format!("Erro na análise: {}", e))?
    .map_err(|e: String| e)?;

    // FASE 3: Instala a track
    let track = Arc::new(TrackData {
        samples,
        sample_rate: sr,
        duration_secs: dur,
        bpm,
        waveform_peaks: peaks,
        waveform_full_min: full_waveform.min,
        waveform_full_max: full_waveform.max,
        waveform_layers: Arc::new(layers),
    });

    let mut engine = state.engine.lock().map_err(|e| e.to_string())?;

    if deck_char == 'A' {
        engine.deck_a.track = Some(track);
        engine.deck_a.src_index = 0.0;
        engine.deck_a.is_playing = false;
        engine.deck_a.pitch = 1.0;
        engine.deck_a.cue_point = 0.0;
        engine.deck_a_filter.reset();
    } else {
        engine.deck_b.track = Some(track);
        engine.deck_b.src_index = 0.0;
        engine.deck_b.is_playing = false;
        engine.deck_b.pitch = 1.0;
        engine.deck_b.cue_point = 0.0;
        engine.deck_b_filter.reset();
    }

    let _ = app.emit(
        "track_analysis_complete",
        AnalysisCompletePayload {
            deck: deck_for_event,
            bpm,
        },
    );

    Ok(TrackInfo {
        duration_secs: dur,
        bpm,
    })
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
    let track = if deck_char == 'A' {
        &engine.deck_a.track
    } else {
        &engine.deck_b.track
    };
    if let Some(track) = track {
        Ok(track.waveform_peaks.clone())
    } else {
        Ok(vec![])
    }
}

#[tauri::command]
pub fn get_waveform_layers(
    state: State<'_, AppState>,
    deck: String,
) -> Result<WaveformLayersResult, String> {
    let deck_char = deck.chars().next().unwrap_or('A');
    let engine = state.engine.lock().map_err(|e| e.to_string())?;
    let track = if deck_char == 'A' {
        &engine.deck_a.track
    } else {
        &engine.deck_b.track
    };
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
pub fn get_full_waveform(
    state: State<'_, AppState>,
    deck: String,
) -> Result<(Vec<f32>, Vec<f32>), String> {
    let deck_char = deck.chars().next().unwrap_or('A');
    let engine = state.engine.lock().map_err(|e| e.to_string())?;
    let track = if deck_char == 'A' {
        &engine.deck_a.track
    } else {
        &engine.deck_b.track
    };
    if let Some(track) = track {
        Ok((
            track.waveform_full_min.clone(),
            track.waveform_full_max.clone(),
        ))
    } else {
        Ok((vec![], vec![]))
    }
}

#[tauri::command]
pub fn get_playback_position(state: State<'_, AppState>, deck: String) -> Result<f64, String> {
    let deck_char = deck.chars().next().unwrap_or('A');
    let engine = state.engine.lock().map_err(|e| e.to_string())?;
    let deck_state = if deck_char == 'A' {
        &engine.deck_a
    } else {
        &engine.deck_b
    };
    if let Some(track) = &deck_state.track {
        Ok(deck_state.src_index / track.samples.len() as f64)
    } else {
        Ok(0.0)
    }
}

#[tauri::command]
pub fn set_deck_eq(
    state: State<'_, AppState>,
    deck: String,
    low: f32,
    mid: f32,
    high: f32,
) -> Result<(), String> {
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
    engine
        .crossfader
        .set_position((position / 100.0).clamp(0.0, 1.0));
    Ok(())
}

#[tauri::command]
pub fn set_deck_pitch(state: State<'_, AppState>, deck: String, pitch: f64) -> Result<(), String> {
    let deck_char = deck.chars().next().unwrap_or('A');
    let mut engine = state.engine.lock().map_err(|e| e.to_string())?;
    let safe_pitch = pitch.clamp(0.5, 2.0);
    if deck_char == 'A' {
        engine.deck_a.pitch = safe_pitch;
    } else {
        engine.deck_b.pitch = safe_pitch;
    }
    Ok(())
}

#[tauri::command]
pub fn set_cue_point(state: State<'_, AppState>, deck: String) -> Result<(), String> {
    let deck_char = deck.chars().next().unwrap_or('A');
    let mut engine = state.engine.lock().map_err(|e| e.to_string())?;
    if deck_char == 'A' {
        engine.deck_a.cue_point = engine.deck_a.src_index;
    } else {
        engine.deck_b.cue_point = engine.deck_b.src_index;
    }
    Ok(())
}

#[tauri::command]
pub fn jump_to_cue(state: State<'_, AppState>, deck: String) -> Result<(), String> {
    let deck_char = deck.chars().next().unwrap_or('A');
    let mut engine = state.engine.lock().map_err(|e| e.to_string())?;
    if deck_char == 'A' {
        engine.deck_a.src_index = engine.deck_a.cue_point;
    } else {
        engine.deck_b.src_index = engine.deck_b.cue_point;
    }
    Ok(())
}

#[tauri::command]
pub fn jog_scrub(state: State<'_, AppState>, deck: String, delta: f64) -> Result<(), String> {
    let deck_char = deck.chars().next().unwrap_or('A');
    let mut engine = state.engine.lock().map_err(|e| e.to_string())?;

    // ✅ CORRIGIDO: Usando f64 para combinar com o src_index do engine.rs
    let scrub_amount = delta * 1000.0;

    if deck_char == 'A' {
        engine.deck_a.src_index = (engine.deck_a.src_index + scrub_amount).max(0.0);
    } else {
        engine.deck_b.src_index = (engine.deck_b.src_index + scrub_amount).max(0.0);
    }

    Ok(())
}

#[tauri::command]
pub fn set_deck_filter(
    state: State<'_, AppState>,
    deck: String,
    position: f32,
) -> Result<(), String> {
    let deck_char = deck.chars().next().unwrap_or('A');
    let mut engine = state.engine.lock().map_err(|e| e.to_string())?;
    let safe_position = position.clamp(0.0, 1.0);
    if deck_char == 'A' {
        engine.deck_a_filter.set_position(safe_position);
    } else {
        engine.deck_b_filter.set_position(safe_position);
    }
    Ok(())
}

#[tauri::command]
pub fn get_vu_levels(state: State<'_, AppState>) -> Result<(f32, f32), String> {
    let engine = state.engine.lock().map_err(|e| e.to_string())?;
    Ok((engine.vu_level_a, engine.vu_level_b))
}

#[tauri::command]
pub fn get_deck_status(deck: String, state: State<'_, AppState>) -> Result<DeckStatus, String> {
    let engine = state.engine.lock().map_err(|e| e.to_string())?;
    let (deck_state, vu_level) = if deck == "A" {
        (&engine.deck_a, engine.vu_level_a)
    } else {
        (&engine.deck_b, engine.vu_level_b)
    };

    // ✅ Conversão segura de f64 para f32 APENAS para a fração de 0.0 a 1.0
    let position = if let Some(track) = &deck_state.track {
        (deck_state.src_index / track.samples.len() as f64).clamp(0.0, 1.0) as f32
    } else {
        0.0
    };

    Ok(DeckStatus {
        position,
        vu_level,
        is_playing: deck_state.is_playing,
    })
}
