use crate::analysis::bpm::detect_bpm;
use crate::analysis::waveform::{
    generate_full_waveform, generate_waveform_layers, generate_waveform_peaks,
};
use crate::audio::decoder::AudioDecoder;
use crate::audio::dsp::resample::resample_linear;
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
    deck: usize,
    path: String,
    app: tauri::AppHandle,
) -> Result<TrackInfo, String> {
    let path_for_task = path.clone();
    let deck_for_event = deck.to_string();

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

    // FASE 3: Instala a track com resample para sample rate do engine
    let engine_sample_rate = {
        let eng = state.engine.lock().map_err(|e| e.to_string())?;
        eng.sample_rate
    };

    // Resample samples se necessário
    let final_samples = if sr != engine_sample_rate {
        resample_linear(&samples, sr, engine_sample_rate)
    } else {
        samples
    };

    let track = Arc::new(TrackData {
        samples: final_samples,
        sample_rate: engine_sample_rate,
        duration_secs: dur,
        bpm,
        waveform_peaks: peaks.clone(),
        waveform_full_min: full_waveform.min.clone(),
        waveform_full_max: full_waveform.max.clone(),
        waveform_layers: Arc::new(layers),
    });

    let mut engine = state.engine.lock().map_err(|e| e.to_string())?;
    if deck >= engine.decks.len() {
        return Err(format!(
            "Deck index {} out of range (max {})",
            deck,
            engine.decks.len()
        ));
    }
    let deck_state = engine
        .decks
        .get_mut(deck)
        .ok_or("Deck index out of range")?;
    deck_state.track = Some(track.clone());
    deck_state.src_index = 0.0;
    deck_state.is_playing = false;
    deck_state.pitch = 1.0;
    deck_state.cue_point = 0.0;
    deck_state.key_lock = false;
    deck_state.trim = 1.0;
    engine.filters[deck].reset();

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
pub fn play_deck(state: State<'_, AppState>, deck: usize) -> Result<(), String> {
    let mut engine = state.engine.lock().map_err(|e| e.to_string())?;
    let deck_state = engine
        .decks
        .get_mut(deck)
        .ok_or("Deck index out of range")?;
    eprintln!(
        "[RUST] play_deck called for deck {}, track loaded: {}",
        deck,
        deck_state.track.is_some()
    );
    deck_state.is_playing = true;
    // Reseta posição se a faixa já terminou
    if let Some(track) = &deck_state.track {
        if deck_state.src_index >= track.samples.len() as f64 {
            deck_state.src_index = 0.0;
        }
    }
    Ok(())
}

#[tauri::command]
pub fn pause_deck(state: State<'_, AppState>, deck: usize) -> Result<(), String> {
    let mut engine = state.engine.lock().map_err(|e| e.to_string())?;
    let deck_state = engine
        .decks
        .get_mut(deck)
        .ok_or("Deck index out of range")?;
    eprintln!("[RUST] pause_deck called for deck {}", deck);
    deck_state.is_playing = false;
    Ok(())
}

#[tauri::command]
pub fn get_waveform_peaks(state: State<'_, AppState>, deck: usize) -> Result<Vec<f32>, String> {
    let engine = state.engine.lock().map_err(|e| e.to_string())?;
    let track = &engine
        .decks
        .get(deck)
        .ok_or("Deck index out of range")?
        .track;
    if let Some(track) = track {
        Ok(track.waveform_peaks.clone())
    } else {
        Ok(vec![])
    }
}

#[tauri::command]
pub fn get_waveform_layers(
    state: State<'_, AppState>,
    deck: usize,
) -> Result<WaveformLayersResult, String> {
    let engine = state.engine.lock().map_err(|e| e.to_string())?;
    let track = &engine
        .decks
        .get(deck)
        .ok_or("Deck index out of range")?
        .track;
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
    deck: usize,
) -> Result<(Vec<f32>, Vec<f32>), String> {
    let engine = state.engine.lock().map_err(|e| e.to_string())?;
    let track = &engine
        .decks
        .get(deck)
        .ok_or("Deck index out of range")?
        .track;
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
pub fn get_playback_position(state: State<'_, AppState>, deck: usize) -> Result<f64, String> {
    let engine = state.engine.lock().map_err(|e| e.to_string())?;
    let deck_state = engine.decks.get(deck).ok_or("Deck index out of range")?;
    if let Some(track) = &deck_state.track {
        Ok(deck_state.src_index / track.samples.len() as f64)
    } else {
        Ok(0.0)
    }
}

#[tauri::command]
pub fn set_deck_eq(
    state: State<'_, AppState>,
    deck: usize,
    low: f32,
    mid: f32,
    high: f32,
) -> Result<(), String> {
    let mut engine = state.engine.lock().map_err(|e| e.to_string())?;
    let eq = &mut engine.eqs[deck];
    eq.set_low_gain(low);
    eq.set_mid_gain(mid);
    eq.set_high_gain(high);
    Ok(())
}

#[tauri::command]
pub fn set_deck_gain(state: State<'_, AppState>, deck: usize, gain_db: f32) -> Result<(), String> {
    let mut engine = state.engine.lock().map_err(|e| e.to_string())?;
    let linear_gain = 10f32.powf(gain_db / 20.0);
    let gain = &mut engine.gains[deck];
    gain.set_gain(linear_gain);
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
pub fn set_deck_pitch(state: State<'_, AppState>, deck: usize, pitch: f64) -> Result<(), String> {
    let mut engine = state.engine.lock().map_err(|e| e.to_string())?;
    let safe_pitch = pitch.clamp(0.5, 2.0);
    let deck_state = engine
        .decks
        .get_mut(deck)
        .ok_or("Deck index out of range")?;
    deck_state.pitch = safe_pitch;
    Ok(())
}

#[tauri::command]
pub fn set_deck_trim(state: State<'_, AppState>, deck: usize, trim: f64) -> Result<(), String> {
    let mut engine = state.engine.lock().map_err(|e| e.to_string())?;
    let safe_trim = trim.clamp(0.0, 2.0);
    let deck_state = engine
        .decks
        .get_mut(deck)
        .ok_or("Deck index out of range")?;
    deck_state.trim = safe_trim;
    Ok(())
}

#[tauri::command]
pub fn set_key_lock(state: State<'_, AppState>, deck: usize, enabled: bool) -> Result<(), String> {
    let mut engine = state.engine.lock().map_err(|e| e.to_string())?;
    let deck_state = engine
        .decks
        .get_mut(deck)
        .ok_or("Deck index out of range")?;
    eprintln!(
        "[RUST] set_key_lock called for deck {}, enabled: {}",
        deck, enabled
    );
    deck_state.key_lock = enabled;
    Ok(())
}

/// Loop In: marca o ponto de entrada do loop na posição atual
#[tauri::command]
pub fn set_loop_in(state: State<'_, AppState>, deck: usize) -> Result<(), String> {
    let mut engine = state.engine.lock().map_err(|e| e.to_string())?;
    let deck_state = engine
        .decks
        .get_mut(deck)
        .ok_or("Deck index out of range")?;
    if let Some(track) = &deck_state.track {
        deck_state.loop_in = deck_state.src_index.min(track.samples.len() as f64);
        deck_state.loop_enabled = true;
        deck_state.loop_active = false;
    }
    Ok(())
}

/// Loop Out: marca o ponto de saída do loop na posição atual e ativa o loop
#[tauri::command]
pub fn set_loop_out(state: State<'_, AppState>, deck: usize) -> Result<(), String> {
    let mut engine = state.engine.lock().map_err(|e| e.to_string())?;
    let deck_state = engine
        .decks
        .get_mut(deck)
        .ok_or("Deck index out of range")?;
    if let Some(track) = &deck_state.track {
        deck_state.loop_out = deck_state.src_index.min(track.samples.len() as f64);
        deck_state.loop_enabled = true;
        deck_state.loop_active = true;
        deck_state.src_index = deck_state.loop_in;
    }
    Ok(())
}

/// Toggle Loop: ativa/desativa o loop armado
#[tauri::command]
pub fn toggle_loop(state: State<'_, AppState>, deck: usize) -> Result<bool, String> {
    let mut engine = state.engine.lock().map_err(|e| e.to_string())?;
    let deck_state = engine
        .decks
        .get_mut(deck)
        .ok_or("Deck index out of range")?;
    deck_state.loop_enabled = !deck_state.loop_enabled;
    deck_state.loop_active = deck_state.loop_enabled;
    Ok(deck_state.loop_enabled)
}

/// ReLoop: se loop ativo, volta para loop_in; se armado mas inativo, ativa
#[tauri::command]
pub fn loop_reloop(state: State<'_, AppState>, deck: usize) -> Result<(), String> {
    let mut engine = state.engine.lock().map_err(|e| e.to_string())?;
    let deck_state = engine
        .decks
        .get_mut(deck)
        .ok_or("Deck index out of range")?;
    if deck_state.loop_enabled {
        deck_state.loop_active = true;
        deck_state.src_index = deck_state.loop_in;
    }
    Ok(())
}

/// Clear Loop: desativa o loop completamente
#[tauri::command]
pub fn clear_loop(state: State<'_, AppState>, deck: usize) -> Result<(), String> {
    let mut engine = state.engine.lock().map_err(|e| e.to_string())?;
    let deck_state = engine
        .decks
        .get_mut(deck)
        .ok_or("Deck index out of range")?;
    deck_state.loop_enabled = false;
    deck_state.loop_active = false;
    deck_state.loop_in = 0.0;
    deck_state.loop_out = 0.0;
    Ok(())
}

#[tauri::command]
pub fn set_cue_point(state: State<'_, AppState>, deck: usize) -> Result<(), String> {
    let mut engine = state.engine.lock().map_err(|e| e.to_string())?;
    let deck_state = engine
        .decks
        .get_mut(deck)
        .ok_or("Deck index out of range")?;
    deck_state.cue_point = deck_state.src_index;
    Ok(())
}

#[tauri::command]
pub fn jump_to_cue(state: State<'_, AppState>, deck: usize) -> Result<(), String> {
    let mut engine = state.engine.lock().map_err(|e| e.to_string())?;
    let deck_state = engine
        .decks
        .get_mut(deck)
        .ok_or("Deck index out of range")?;
    deck_state.src_index = deck_state.cue_point;
    Ok(())
}

#[tauri::command]
pub fn jog_scrub(state: State<'_, AppState>, deck: usize, delta: f64) -> Result<(), String> {
    let mut engine = state.engine.lock().map_err(|e| e.to_string())?;
    let sample_rate = engine.sample_rate as f64;
    let scrub_amount = delta * sample_rate;

    let deck_state = engine
        .decks
        .get_mut(deck)
        .ok_or("Deck index out of range")?;
    let max_index = deck_state
        .track
        .as_ref()
        .map(|t| t.samples.len() as f64)
        .unwrap_or(f64::MAX);
    deck_state.src_index = (deck_state.src_index + scrub_amount)
        .max(0.0)
        .min(max_index);

    Ok(())
}

#[tauri::command]
pub fn set_deck_filter(
    state: State<'_, AppState>,
    deck: usize,
    position: f32,
) -> Result<(), String> {
    let mut engine = state.engine.lock().map_err(|e| e.to_string())?;
    let safe_position = position.clamp(0.0, 1.0);
    let filter = &mut engine.filters[deck];
    filter.set_position(safe_position);
    Ok(())
}

#[tauri::command]
pub fn get_vu_levels(state: State<'_, AppState>) -> Result<Vec<f32>, String> {
    let engine = state.engine.lock().map_err(|e| e.to_string())?;
    Ok(engine.vu_levels.clone())
}

#[tauri::command]
pub fn get_deck_status(deck: usize, state: State<'_, AppState>) -> Result<DeckStatus, String> {
    let engine = state.engine.lock().map_err(|e| e.to_string())?;
    let deck_state = engine.decks.get(deck).ok_or("Deck index out of range")?;
    let vu_level = engine.vu_levels[deck];

    eprintln!("[DEBUG] get_deck_status deck={} vu_level={} is_playing={} track_loaded={} src_index={}", deck, vu_level, deck_state.is_playing, deck_state.track.is_some(), deck_state.src_index);

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
