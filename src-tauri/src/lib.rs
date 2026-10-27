mod audio;
pub mod analysis;
mod commands;
mod state;
mod database;

// Re-export for integration tests
pub use audio::engine::{AudioEngine, DeckState, TrackData};
pub use analysis::waveform::WaveformLayers;
pub use analysis::bpm::detect_bpm;
pub use analysis::waveform::{generate_waveform_peaks, generate_waveform_layers};
pub use audio::decoder::AudioDecoder;

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
                    commands::get_playback_position,
                    commands::set_deck_eq,
                    commands::set_deck_gain,
                    commands::set_crossfader,
                    commands::set_deck_pitch,
                    commands::set_cue_point,
                    commands::jump_to_cue,
                    commands::jog_scrub,
                    commands::set_deck_filter,
                    commands::get_vu_levels,
                ])
        .run(tauri::generate_context!())
    {
        Ok(_) => println!("✅ MixLab finalizado normalmente"),
        Err(e) => {
            eprintln!("❌ Erro fatal ao rodar MixLab: {}", e);
            std::process::exit(1);
        }
    }
}