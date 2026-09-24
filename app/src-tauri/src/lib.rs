//! Rufplan Studio desktop shell. Tauri commands here are thin wrappers over core crates.

mod commands;
mod menu;
mod session;

use tauri::Emitter;

/// Builds and runs the Tauri application until the last window closes.
pub fn run() -> anyhow::Result<()> {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(commands::SessionState::default())
        .menu(menu::build)
        .on_menu_event(|app, event| {
            // Emitting only fails if the webview is gone, in which case there is no one to tell.
            let _ = app.emit(menu::MENU_EVENT, event.id().0.as_str());
        })
        .invoke_handler(tauri::generate_handler![
            commands::core_version,
            commands::project_status,
            commands::project_new,
            commands::project_open,
            commands::project_save,
        ])
        .run(tauri::generate_context!())?;
    Ok(())
}
