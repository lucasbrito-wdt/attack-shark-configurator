//! Tauri shell around `shark-core`.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod catalog;
mod commands;
mod dto;
mod error;
mod runtime;
mod state;
mod tray;
mod tray_glyph;
mod tray_i18n;

#[cfg(target_os = "macos")]
use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_opener::init())
        .on_window_event(|window, event| {
            // closing sends the app to the tray (and off the Dock) so the tray keeps updating;
            // quit via the tray menu or Cmd+Q
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
                #[cfg(target_os = "macos")]
                let _ = window.app_handle().set_dock_visibility(false);
            }
        })
        .setup(|app| {
            runtime::start(app.handle());
            tray::init(app.handle())?;
            #[cfg(target_os = "macos")]
            if let Some(window) = app.get_webview_window("main") {
                use window_vibrancy::{NSVisualEffectMaterial, apply_vibrancy};
                if let Err(e) = apply_vibrancy(&window, NSVisualEffectMaterial::UnderWindowBackground, None, None) {
                    eprintln!("vibrancy unavailable: {e}");
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_snapshot,
            commands::list_profiles,
            commands::create_profile,
            commands::rename_profile,
            commands::delete_profile,
            commands::select_profile,
            commands::import_profile,
            commands::export_profile,
            commands::reset_profile,
            commands::update_settings,
            commands::set_button_action,
            commands::list_actions,
            commands::list_macros,
            commands::create_macro,
            commands::update_macro,
            commands::delete_macro,
            commands::apply_all,
            tray::set_locale,
            commands::read_back,
            commands::verify_dpi_encoding,
        ])
        .build(tauri::generate_context!())
        .expect("error while building the application")
        .run(|_app, _event| {
            // the close button only hides the window, so opening the app again has to bring it back
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen { .. } = _event {
                tray::show_window(_app);
            }
        });
}
