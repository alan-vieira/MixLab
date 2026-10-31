pub mod analysis;
mod audio;
mod commands;
mod database;
mod state;

// Re-export for integration tests
pub use analysis::bpm::detect_bpm;
pub use analysis::waveform::WaveformLayers;
pub use analysis::waveform::{generate_waveform_layers, generate_waveform_peaks};
pub use audio::analyzer::TrackAnalysis;
pub use audio::decoder::AudioDecoder;
pub use audio::engine::{AudioEngine, TrackData};
pub use audio::state::{AudioThreadState, DeckId, DeckTelemetry};

use state::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Captura panics e mostra no console para debug
    std::panic::set_hook(Box::new(|panic_info| {
        eprintln!("🔥 PANIC: {}", panic_info);
    }));

    println!(" Iniciando MixLab...");

    match tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            commands::load_track,
            commands::play_deck,
            commands::pause_deck,
            commands::get_waveform_peaks,
            commands::get_waveform_layers,
            commands::get_full_waveform,
            commands::set_deck_eq,
            commands::set_deck_gain,
            commands::set_crossfader,
            commands::set_deck_pitch,
            commands::set_cue_point,
            commands::jump_to_cue,
            commands::jog_scrub,
            commands::set_deck_filter,
            commands::seek_track,
            commands::get_deck_status,
            commands::subscribe_deck_telemetry,
        ])
        .setup(|app| {
            // Ouve o evento de fechamento da janela principal no Tauri v2
            if let Some(window) = app.get_webview_window("main") {
                let app_handle = app.handle().clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { .. } = event {
                        if let Some(state) = app_handle.try_state::<crate::state::AppState>() {
                            state.audio_thread_state.request_shutdown();
                        }
                    }
                });
            }
            Ok(())
        })
        .run(tauri::generate_context!()) {
        Ok(_) => println!("✅ MixLab finalizado normalmente"),
        Err(e) => {
            eprintln!("❌ Erro fatal ao rodar MixLab: {}", e);
            std::process::exit(1);
        }
    }
}
