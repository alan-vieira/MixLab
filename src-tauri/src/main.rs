#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;
mod analysis;
mod state;
mod commands;

use state::AppState;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            commands::load_track,
            commands::play_deck,
            commands::pause_deck,
            commands::get_waveform_peaks,
            commands::get_waveform_layers,
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
        .expect("Erro ao rodar o aplicativo Tauri");
}