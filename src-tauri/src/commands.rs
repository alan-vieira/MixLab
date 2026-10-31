use crate::analysis::bpm::detect_bpm;
use crate::analysis::waveform::{
    generate_full_waveform, generate_waveform_layers, generate_waveform_peaks, WaveformLayers,
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
    pub sample_rate: u32,
    pub bpm: f32,
    pub waveform_peaks: Vec<f32>,
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

    // ⚡ FASE 1: Decodificação RÁPIDA - apenas primeiros 30s + metadados
    let quick_decode = tauri::async_runtime::spawn_blocking(move || {
        AudioDecoder::load_and_decode_quick(&path_for_task)
            .map_err(|e| format!("Falha ao decodificar: {}", e))
    })
    .await
    .map_err(|e| format!("Erro na decodificação: {}", e))??;

    let (preview_samples, sr, _channels, dur, full_samples_arc) = quick_decode;

    // ⚡ FASE 2: Waveform PREVIEW rápida (100 pontos) + instala track para tocar AGORA
    let preview_peaks = generate_waveform_peaks(&preview_samples, 100);
    
    let track = Arc::new(TrackData {
        samples: preview_samples.to_vec(), // Apenas preview para começar
        sample_rate: sr,
        duration_secs: dur,
        bpm: 0.0, // Será atualizado pela análise completa
        waveform_peaks: preview_peaks.clone(),
        waveform_full_min: preview_peaks.clone(),
        waveform_full_max: preview_peaks.clone(),
        waveform_layers: Arc::new(WaveformLayers {
            low: vec![0.1; 100],
            mid: vec![0.5; 100],
            high: vec![0.1; 100],
        }),
    });

    // Instala track IMEDIATAMENTE para poder tocar
    {
        let mut engine = state.engine.lock().map_err(|e| e.to_string())?;
        if deck_char == 'A' {
            engine.deck_a.track = Some(track.clone());
            engine.deck_a.src_index = 0.0;
            engine.deck_a.is_playing = false;
            engine.deck_a.pitch = 1.0;
            engine.deck_a.cue_point = 0.0;
            engine.deck_a_filter.reset();
        } else {
            engine.deck_b.track = Some(track.clone());
            engine.deck_b.src_index = 0.0;
            engine.deck_b.is_playing = false;
            engine.deck_b.pitch = 1.0;
            engine.deck_b.cue_point = 0.0;
            engine.deck_b_filter.reset();
        }
    }

    // ⚡ FASE 3: Análise COMPLETA em background (não bloqueia UI)
    let app_for_analysis = app.clone();
    let path_for_analysis = path.clone();
    let deck_for_analysis = deck.clone();
    let full_samples = full_samples_arc.clone();
    let sr_for_analysis = sr;
    let dur_for_analysis = dur;

    tauri::async_runtime::spawn(async move {
        let result = tauri::async_runtime::spawn_blocking(move || {
            let state = app_for_analysis.state::<AppState>();
            let db = state.db.lock().map_err(|e| e.to_string())?;

            let (bpm, peaks, full_waveform, layers) = if let Ok(Some(cached)) = db.get_cached(&path_for_analysis) {
                (
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
                )
            } else {
                let bpm = detect_bpm(&full_samples, sr_for_analysis as f32);
                let peaks = generate_waveform_peaks(&full_samples, 1000);
                let full_waveform = generate_full_waveform(&full_samples, 1000);
                let layers = generate_waveform_layers(&full_samples, sr_for_analysis as f32, 5000);

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

                (bpm, peaks, full_waveform, layers)
            };

            // Atualiza engine com dados completos
            let mut engine = state.engine.lock().map_err(|e| e.to_string())?;
            let deck_state = if deck_for_analysis == "A" {
                &mut engine.deck_a
            } else {
                &mut engine.deck_b
            };
            
            if let Some(ref track) = deck_state.track {
                // Cria novo TrackData com dados completos (preservando sample_rate e duration)
                let new_track = Arc::new(TrackData {
                    samples: (*full_samples).clone(),
                    sample_rate: track.sample_rate,
                    duration_secs: track.duration_secs,
                    bpm,
                    waveform_peaks: peaks.clone(),
                    waveform_full_min: full_waveform.min.clone(),
                    waveform_full_max: full_waveform.max.clone(),
                    waveform_layers: Arc::new(layers),
                });
                // Substitui o track
                deck_state.track = Some(new_track);
            }

            // Emite evento para frontend atualizar BPM + waveform
            let _ = app_for_analysis.emit(
                "track_analysis_complete",
                serde_json::json!({
                    "deck": deck_for_analysis,
                    "bpm": bpm,
                    "duration": dur_for_analysis
                }),
            );

            Ok::<_, String>(())
        })
        .await;

        if let Err(e) = result {
            eprintln!("Erro na análise de fundo: {}", e);
        }
    });

    // Retorna IMEDIATAMENTE com preview
    Ok(TrackInfo {
        duration_secs: dur,
        sample_rate: sr,
        bpm: 0.0, // Será atualizado via evento
        waveform_peaks: preview_peaks,
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

    // ✅ CORRIGIDO: delta vem do frontend como graus/180
    // Frontend: 360° = 2 segundos → delta = 2.0
    // Backend: deve mover 2 * sample_rate samples
    let sample_rate = 44100.0; // mesmo sample_rate do engine
    let scrub_amount = delta * sample_rate;

    if deck_char == 'A' {
        engine.deck_a.src_index = (engine.deck_a.src_index + scrub_amount)
            .max(0.0)
            .min(engine.deck_a.track.as_ref().map(|t| t.samples.len() as f64).unwrap_or(f64::MAX));
    } else {
        engine.deck_b.src_index = (engine.deck_b.src_index + scrub_amount)
            .max(0.0)
            .min(engine.deck_b.track.as_ref().map(|t| t.samples.len() as f64).unwrap_or(f64::MAX));
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
