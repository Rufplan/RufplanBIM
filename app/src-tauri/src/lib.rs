//! Rufplan Studio desktop shell. Tauri commands here are thin wrappers over core crates.

mod commands;

/// Builds and runs the Tauri application until the last window closes.
pub fn run() -> anyhow::Result<()> {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![commands::core_version])
        .run(tauri::generate_context!())?;
    Ok(())
}
