//! Rufplan Studio desktop shell. Tauri commands here are thin wrappers over core crates.

mod cloud;
mod commands;
mod editing;
mod menu;
mod session;

use tauri::{Emitter, Manager, WindowEvent};

/// Builds and runs the Tauri application until the last window closes.
pub fn run() -> anyhow::Result<()> {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(cloud::CloudState::default())
        .manage({
            let mut session = session::Session::default();
            commands::open_from_args(&mut session);
            commands::SessionState::new(session)
        })
        .menu(menu::build)
        .on_menu_event(|app, event| {
            // Emitting only fails if the webview is gone, in which case there is no one to tell.
            let _ = app.emit(menu::MENU_EVENT, event.id().0.as_str());
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let dirty = window
                    .app_handle()
                    .state::<commands::SessionState>()
                    .lock()
                    .map(|s| s.is_dirty())
                    .unwrap_or(false);
                if dirty {
                    // Let the UI ask Save / Don't Save / Cancel, then call app_exit.
                    api.prevent_close();
                    let _ = window.emit(menu::MENU_EVENT, menu::FILE_EXIT);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::core_version,
            commands::app_state,
            commands::project_new,
            commands::project_sample,
            commands::project_open,
            commands::project_save,
            commands::view_display_list,
            commands::view_meshes,
            commands::pick,
            commands::snap,
            commands::create_wall,
            commands::create_grid,
            commands::create_level,
            commands::create_floor,
            commands::create_ceiling,
            commands::opening_preview,
            commands::create_opening,
            commands::room_preview,
            commands::create_room,
            commands::move_elements,
            commands::create_section,
            commands::dimension_preview,
            commands::create_dimension,
            commands::create_text,
            commands::create_sheet,
            commands::place_view,
            commands::schedule_table,
            commands::export_pdf,
            commands::export_ifc,
            commands::tag_all,
            commands::issue_set,
            commands::delete_elements,
            commands::properties,
            commands::set_property,
            commands::undo,
            commands::redo,
            commands::app_exit,
            editing::parse_length,
            editing::handles,
            editing::drag_handle,
            editing::set_temp_dimension,
            editing::copy_elements,
            editing::rotate_elements,
            editing::mirror_elements,
            editing::trim_extend,
            editing::offset_preview,
            editing::offset_element,
            editing::split_wall,
            editing::flip_selection,
            editing::ref_line,
            editing::align,
            editing::create_roof,
            editing::create_stair,
            editing::add_project_parameter,
            editing::remove_project_parameter,
            cloud::cloud_status,
            cloud::cloud_sign_in,
            cloud::cloud_sign_in_google,
            cloud::cloud_sign_out,
            cloud::cloud_projects,
            cloud::link_rufplan,
            cloud::publish_options,
            cloud::publish_to_rufplan,
        ])
        .run(tauri::generate_context!())?;
    Ok(())
}
