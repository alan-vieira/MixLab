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
        .run(tauri::generate_context!(), |app_handle, event| {
            // Sinaliza shutdown ordenado quando a janela principal fecha
            if let tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::WindowEvent { event: tauri::WindowEvent::CloseRequested { .. }, .. } = event {
                if let Some(state) = app_handle.try_state::<state::AppState>() {
                    state.audio_thread_state.request_shutdown();
                }
            }
        })
    {
        Ok(_) => println!("✅ MixLab finalizado normalmente"),
        Err(e) => {
            eprintln!("❌ Erro fatal ao rodar MixLab: {}", e);
            std::process::exit(1);
        }
    }
}
