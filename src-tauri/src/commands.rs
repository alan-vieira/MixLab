use crate::audio::state::{DeckId, deck_a_telemetry_rx, deck_b_telemetry_rx};
use crate::state::AppState;
use serde::Serialize;
use std::path::Path;
use tauri::{Emitter, Manager, State};

/// Extensões de áudio permitidas (case-insensitive)
const ALLOWED_EXTENSIONS: &[&str] = &["mp3", "wav", "flac", "aac", "ogg", "m4a"];

/// Valida um caminho de arquivo recebido do frontend para evitar path traversal
/// e acesso a arquivos fora de diretórios permitidos.
///
/// Regras:
/// 1. Rejeita caracteres nulo
/// 2. Rejeita componentes `..` (path traversal)
/// 3. A extensão deve estar em ALLOWED_EXTENSIONS
///
/// Caminhos absolutos (ex: C:\música\faixa.mp3) são permitidos — é o uso normal
/// em um app de desktop Tauri onde o usuário seleciona arquivos locais.
fn validate_audio_path(path: &str) -> Result<(), String> {
    if path.contains('\0') {
        return Err("Caminho contém caractere nulo".to_string());
    }

    let path_buf = Path::new(path);

    // Rejeita componentes ".." (path traversal)
    for component in path_buf.components() {
        if let std::path::Component::ParentDir = component {
            return Err("Componentes '..' não são permitidos no caminho".to_string());
        }
    }

    // Valida extensão de arquivo
    let extension = path_buf
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if !ALLOWED_EXTENSIONS.contains(&extension.as_str()) {
        return Err(format!(
            "Extensão '{}' não permitida. Extensões válidas: {}",
            extension,
            ALLOWED_EXTENSIONS.join(", ")
        ));
    }

    Ok(())
}

#[allow(dead_code)]
type WaveformLayersResult = (Vec<f32>, Vec<f32>, Vec<f32>);

#[derive(Serialize, Clone)]
pub struct TrackInfo {
    pub duration_secs: f64,
    pub bpm: f32,
    pub peaks: Vec<f32>,
    pub waveform_low: Vec<f32>,
    pub waveform_mid: Vec<f32>,
    pub waveform_high: Vec<f32>,
}

#[derive(Serialize, Clone)]
struct AnalysisCompletePayload {
    deck: String,
    bpm: f32,
    peaks: Vec<f32>,
    waveform_low: Vec<f32>,
    waveform_mid: Vec<f32>,
    waveform_high: Vec<f32>,
}

#[derive(Serialize)]
pub struct DeckStatus {
    pub position: f32,
    pub vu_level: f32,
    pub is_playing: bool,
    pub pitch: f32,
    pub duration_secs: f64,
}

#[tauri::command]
pub async fn load_track(
    state: State<'_, AppState>,
    deck: String,
    path: String,
    app: tauri::AppHandle,
) -> Result<TrackInfo, String> {
    // Validação de caminho para prevenir path traversal
    validate_audio_path(&path)?;

    let deck_char = deck.chars().next().unwrap_or('A');
    let deck_id = if deck_char == 'A' { DeckId::A } else { DeckId::B };
    let path_for_analysis = path.clone();
    let deck_for_event = deck.clone();
    let path_for_analysis_clone = path_for_analysis.clone();

    let _app_for_analysis = app.clone();
    let (bpm, peaks, _full_waveform, _layers, dur) = tauri::async_runtime::spawn_blocking(move || {
        let analysis = crate::audio::analyzer::analyze_track_streaming(&path_for_analysis_clone)
            .map_err(|e| format!("Falha na análise streaming: {}", e))?;
        Ok((
            analysis.bpm,
            analysis.peaks,
            (analysis.full_waveform_min, analysis.full_waveform_max),
            (analysis.layers_low, analysis.layers_mid, analysis.layers_high),
            analysis.duration_secs,
        ))
    })
    .await
    .map_err(|e| format!("Erro na análise: {}", e))?
    .map_err(|e: String| e)?;

    let app_for_db = app.clone();
    let path_for_db = path_for_analysis.clone();
    let peaks_clone = peaks.clone();
    let layers_clone = (_layers.0.clone(), _layers.1.clone(), _layers.2.clone());
    
    tauri::async_runtime::spawn_blocking(move || {
        let state = app_for_db.state::<AppState>();
        // parking_lot::Mutex::lock() retorna o Guard direto, sem Result
        let db = state.db.lock(); 
        let _ = db.save_analysis(
            &path_for_db,
            &crate::database::CachedAnalysis {
                bpm,
                duration_secs: dur,
                waveform_peaks: peaks_clone.clone(),
                waveform_full_min: _full_waveform.0.clone(),
                waveform_full_max: _full_waveform.1.clone(),
                waveform_low: layers_clone.0.clone(),
                waveform_mid: layers_clone.1.clone(),
                waveform_high: layers_clone.2.clone(),
            },
        );
    });

    // Sem .unwrap() aqui também
    state.engine.lock().load_track(deck_id, &path).map_err(|e| e.to_string())?;

    let _ = app.emit(
        "track_analysis_complete",
        AnalysisCompletePayload {
            deck: deck_for_event,
            bpm,
            peaks: peaks.clone(),
            waveform_low: _layers.0.clone(),
            waveform_mid: _layers.1.clone(),
            waveform_high: _layers.2.clone(),
        },
    );

    Ok(TrackInfo {
        duration_secs: dur,
        bpm,
        peaks,
        waveform_low: _layers.0,
        waveform_mid: _layers.1,
        waveform_high: _layers.2,
    })
}

#[tauri::command]
pub fn play_deck(state: State<'_, AppState>, deck: String) -> Result<(), String> {
    let deck_id = if deck.chars().next().unwrap_or('A') == 'A' { DeckId::A } else { DeckId::B };
    state.engine.lock().play(deck_id);
    Ok(())
}

#[tauri::command]
pub fn pause_deck(state: State<'_, AppState>, deck: String) -> Result<(), String> {
    let deck_id = if deck.chars().next().unwrap_or('A') == 'A' { DeckId::A } else { DeckId::B };
    state.engine.lock().pause(deck_id);
    Ok(())
}

#[tauri::command]
pub fn get_waveform_peaks(_state: State<'_, AppState>, _deck: String) -> Result<Vec<f32>, String> {
    Ok(vec![])
}

#[tauri::command]
pub fn get_waveform_layers(_state: State<'_, AppState>, _deck: String) -> Result<WaveformLayersResult, String> {
    Ok((vec![], vec![], vec![]))
}

#[tauri::command]
pub fn get_full_waveform(_state: State<'_, AppState>, _deck: String) -> Result<(Vec<f32>, Vec<f32>), String> {
    Ok((vec![], vec![]))
}

#[tauri::command]
pub fn set_deck_eq(state: State<'_, AppState>, deck: String, low: f32, mid: f32, high: f32) -> Result<(), String> {
    let deck_id = if deck.chars().next().unwrap_or('A') == 'A' { DeckId::A } else { DeckId::B };
    state.engine.lock().set_eq(deck_id, low, mid, high);
    Ok(())
}

#[tauri::command]
pub fn set_deck_gain(state: State<'_, AppState>, deck: String, gain_db: f32) -> Result<(), String> {
    let deck_id = if deck.chars().next().unwrap_or('A') == 'A' { DeckId::A } else { DeckId::B };
    state.engine.lock().set_gain(deck_id, gain_db);
    Ok(())
}

#[tauri::command]
pub fn set_crossfader(state: State<'_, AppState>, position: f32) -> Result<(), String> {
    let pos = (position / 100.0).clamp(0.0, 1.0);
    state.engine.lock().set_crossfader(pos);
    Ok(())
}

#[tauri::command]
pub fn set_deck_pitch(state: State<'_, AppState>, deck: String, pitch: f32) -> Result<(), String> {
    let deck_id = if deck.chars().next().unwrap_or('A') == 'A' { DeckId::A } else { DeckId::B };
    let safe_pitch = pitch.clamp(0.25, 4.0);
    state.engine.lock().set_tempo(deck_id, safe_pitch);
    Ok(())
}

#[tauri::command]
pub fn set_cue_point(state: State<'_, AppState>, deck: String) -> Result<(), String> {
    let deck_id = if deck.chars().next().unwrap_or('A') == 'A' { DeckId::A } else { DeckId::B };
    state.engine.lock().set_cue_point(deck_id);
    Ok(())
}

#[tauri::command]
pub fn jump_to_cue(state: State<'_, AppState>, deck: String) -> Result<(), String> {
    let deck_id = if deck.chars().next().unwrap_or('A') == 'A' { DeckId::A } else { DeckId::B };
    state.engine.lock().jump_to_cue(deck_id);
    Ok(())
}

#[tauri::command]
pub fn jog_scrub(state: State<'_, AppState>, deck: String, delta: f32) -> Result<(), String> {
    let deck_id = if deck.chars().next().unwrap_or('A') == 'A' { DeckId::A } else { DeckId::B };
    state.engine.lock().jog_scrub(deck_id, delta);
    Ok(())
}

#[tauri::command]
pub fn set_deck_filter(state: State<'_, AppState>, deck: String, position: f32) -> Result<(), String> {
    let deck_id = if deck.chars().next().unwrap_or('A') == 'A' { DeckId::A } else { DeckId::B };
    let safe_position = position.clamp(0.0, 1.0);
    state.engine.lock().set_filter(deck_id, safe_position);
    Ok(())
}

#[tauri::command]
pub fn seek_track(state: State<'_, AppState>, deck: String, time_secs: f64) -> Result<(), String> {
    let deck_id = if deck.chars().next().unwrap_or('A') == 'A' { DeckId::A } else { DeckId::B };
    state.engine.lock().seek(deck_id, time_secs);
    Ok(())
}

#[tauri::command]
pub fn get_deck_status(deck: String, state: State<'_, AppState>) -> Result<DeckStatus, String> {
    let telemetry = if deck == "A" {
        deck_a_telemetry_rx().try_recv().ok()
    } else {
        deck_b_telemetry_rx().try_recv().ok()
    };

    if let Some(t) = telemetry {
        Ok(DeckStatus {
            position: t.position,
            vu_level: t.vu_level,
            is_playing: t.is_playing,
            pitch: t.pitch,
            duration_secs: t.duration_secs,
        })
    } else {
        let shared = &state.audio_thread_state;
        let (playing, pitch) = if deck == "A" {
            (
                shared.deck_a_playing.load(std::sync::atomic::Ordering::Relaxed),
                shared.deck_a_tempo.load(std::sync::atomic::Ordering::Relaxed),
            )
        } else {
            (
                shared.deck_b_playing.load(std::sync::atomic::Ordering::Relaxed),
                shared.deck_b_tempo.load(std::sync::atomic::Ordering::Relaxed),
            )
        };
        Ok(DeckStatus {
            position: 0.0,
            vu_level: 0.0,
            is_playing: playing,
            pitch,
            duration_secs: 0.0,
        })
    }
}

#[tauri::command]
pub async fn subscribe_deck_telemetry(
    deck: String,
    channel: tauri::ipc::Channel<DeckStatus>,
    _state: State<'_, AppState>,
) -> Result<(), String> {
    let deck_id = if deck == "A" { DeckId::A } else { DeckId::B };
    
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(33));
        loop {
            interval.tick().await;
            let status = if deck_id == DeckId::A {
                deck_a_telemetry_rx().try_recv().ok().map(|t| DeckStatus {
                    position: t.position,
                    vu_level: t.vu_level,
                    is_playing: t.is_playing,
                    pitch: t.pitch,
                    duration_secs: t.duration_secs,
                })
            } else {
                deck_b_telemetry_rx().try_recv().ok().map(|t| DeckStatus {
                    position: t.position,
                    vu_level: t.vu_level,
                    is_playing: t.is_playing,
                    pitch: t.pitch,
                    duration_secs: t.duration_secs,
                })
            };
            
            if let Some(s) = status {
                if channel.send(s).is_err() {
                    break;
                }
            }
        }
    });
    Ok(())
}