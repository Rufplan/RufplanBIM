//! Rufplan Studio desktop shell. Tauri commands here are thin wrappers over core crates.

mod cloud;
mod commands;
mod detailing;
mod editing;
mod generate_cmds;
mod material_cmds;
mod menu;
mod render_cmds;
mod session;
mod shortcut_cmds;
mod site_cmds;
mod sketching;
mod structure;
mod window_cmds;

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
            structure::create_column,
            structure::columns_at_grids,
            structure::create_beam,
            structure::create_railing,
            structure::attach_wall_tops,
            structure::create_wall_located,
            structure::drawing_options,
            detailing::create_room_separator,
            detailing::create_callout,
            detailing::create_elevation_marker,
            detailing::opening_preview_3d,
            detailing::set_section_box,
            detailing::create_material,
            generate_cmds::claude_key_set,
            generate_cmds::claude_set_key,
            generate_cmds::generate_building,
            material_cmds::material_library,
            material_cmds::add_library_material,
            material_cmds::apply_material,
            material_cmds::render_materials,
            material_cmds::material_texture,
            window_cmds::window_library,
            window_cmds::window_preview,
            window_cmds::load_window_types,
            shortcut_cmds::set_pinned,
            shortcut_cmds::select_all_instances,
            shortcut_cmds::tag_element,
            shortcut_cmds::hide_elements,
            shortcut_cmds::set_category_visible,
            shortcut_cmds::unhide_all,
            shortcut_cmds::view_categories,
            site_cmds::site_keys,
            site_cmds::site_set_keys,
            site_cmds::site_parcel,
            site_cmds::site_set_lot,
            site_cmds::site_fetch_topo,
            site_cmds::site_imagery_frame,
            site_cmds::site_imagery,
            render_cmds::create_camera,
            render_cmds::set_camera_pose,
            render_cmds::sun_position,
            render_cmds::save_render,
            sketching::sketch_begin,
            sketching::sketch_draw,
            sketching::sketch_pick_walls,
            sketching::sketch_pick_line,
            sketching::sketch_hit,
            sketching::sketch_fillet,
            sketching::sketch_trim,
            sketching::sketch_delete,
            sketching::sketch_move_vertex,
            sketching::sketch_flip,
            sketching::sketch_undo,
            sketching::sketch_set_type,
            sketching::sketch_finish,
            sketching::sketch_cancel,
            sketching::sketch_preview,
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
