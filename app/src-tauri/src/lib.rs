//! Rufplan Studio desktop shell. Tauri commands here are thin wrappers over core crates.

mod cloud;
mod commands;
mod detail_cmds;
mod detailing;
mod door_cmds;
mod editing;
mod ffe_cmds;
mod generate_cmds;
mod group_cmds;
mod inplace_cmds;
mod keynote_cmds;
mod lighting_cmds;
mod lines_cmds;
mod material_cmds;
mod menu;
mod mep_cmds;
mod model_edit_cmds;
mod plans_cmds;
mod planting_cmds;
mod project_cmds;
mod qa_cmds;
mod render_cmds;
mod samples;
mod session;
mod sheetset_cmds;
mod shortcut_cmds;
mod site_cmds;
mod sketching;
mod spec_cmds;
mod standards_cmds;
mod structural_cmds;
mod structure;
mod symbols_cmds;
mod window_cmds;
mod workset_cmds;

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
            commands::auto_render,
            commands::quit_app,
            commands::project_open,
            commands::project_import_ifc,
            commands::project_save,
            commands::view_display_list,
            commands::view_meshes,
            commands::section_caps,
            commands::site_terrain,
            commands::pick,
            commands::pick_cycle,
            commands::pick_in_rect,
            commands::element_categories,
            commands::grip_snap,
            commands::pick_candidates,
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
            commands::dimension_references,
            commands::dimension_string_preview,
            commands::create_dimension_string,
            commands::angular_preview,
            commands::create_angular_dimension,
            commands::create_text,
            commands::create_text_note,
            commands::text_note_info,
            commands::add_text_leader,
            commands::remove_text_leader,
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
            editing::viewport_info,
            editing::set_temp_dimension,
            editing::copy_elements,
            editing::rotate_elements,
            editing::mirror_elements,
            editing::trim_extend,
            editing::offset_preview,
            editing::offset_element,
            editing::split_wall,
            editing::flip_selection,
            editing::flip_opening,
            editing::fascia_catalog,
            editing::set_fascia,
            editing::save_rendering,
            editing::render_image,
            editing::ref_line,
            editing::align,
            editing::align_references,
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
            detailing::reference_targets,
            detailing::create_reference,
            detailing::reference_target,
            workset_cmds::worksets_list,
            workset_cmds::set_active_workset,
            workset_cmds::create_workset,
            workset_cmds::rename_workset,
            workset_cmds::delete_workset,
            workset_cmds::set_workset_visible_in_all_views,
            workset_cmds::set_workset_visible_in_view,
            workset_cmds::set_elements_workset,
            workset_cmds::element_worksets,
            structural_cmds::structural_suggest,
            structural_cmds::structural_generate,
            structural_cmds::structural_layer,
            structural_cmds::structural_overlay_2d,
            structural_cmds::structural_overlay_3d,
            structural_cmds::structural_pick,
            structural_cmds::structural_info,
            structural_cmds::structural_export_json,
            structural_cmds::structural_export_ifc,
            structural_cmds::structural_edit_rules,
            keynote_cmds::keynote_table,
            keynote_cmds::keynote_save,
            keynote_cmds::keynote_delete,
            keynote_cmds::keynote_set_numbering,
            keynote_cmds::keynote_import,
            keynote_cmds::keynote_export,
            keynote_cmds::keynote_assign,
            keynote_cmds::keynote_assignables,
            keynote_cmds::keynote_target,
            keynote_cmds::keynote_place,
            keynote_cmds::keynote_legend,
            mep_cmds::mep_suggest,
            mep_cmds::mep_generate,
            mep_cmds::mep_overlay_2d,
            mep_cmds::mep_overlay_3d,
            mep_cmds::mep_pick,
            mep_cmds::mep_info,
            mep_cmds::mep_export_json,
            mep_cmds::mep_edit_rules,
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
            material_cmds::paint_elements,
            material_cmds::render_materials,
            material_cmds::material_texture,
            sheetset_cmds::building_types,
            sheetset_cmds::sheet_set_plan,
            sheetset_cmds::create_sheet_sets,
            sheetset_cmds::export_sheet_sets,
            door_cmds::door_library,
            door_cmds::door_preview,
            door_cmds::load_door_types,
            door_cmds::opening_thumbnail,
            plans_cmds::plans_to_model,
            window_cmds::window_library,
            window_cmds::window_preview,
            window_cmds::load_window_types,
            shortcut_cmds::set_pinned,
            shortcut_cmds::select_all_instances,
            shortcut_cmds::selection_categories,
            ffe_cmds::ffe_library,
            ffe_cmds::load_ffe_types,
            ffe_cmds::ffe_thumbnail,
            ffe_cmds::create_ffe,
            lighting_cmds::lighting_library,
            lighting_cmds::load_lighting_types,
            lighting_cmds::fixture_thumbnail,
            lighting_cmds::create_lighting_fixture,
            lighting_cmds::set_lights,
            lighting_cmds::lights,
            lighting_cmds::set_sun_settings,
            lighting_cmds::sun_now,
            planting_cmds::planting_library,
            planting_cmds::load_planting_types,
            planting_cmds::plant_model,
            planting_cmds::plant_texture,
            planting_cmds::plant_instances,
            planting_cmds::create_plants,
            planting_cmds::set_base_ground,
            planting_cmds::ground_library,
            planting_cmds::planting_types,
            planting_cmds::grass_kinds,
            planting_cmds::paint_grass,
            planting_cmds::erase_grass,
            planting_cmds::grass_patches,
            lighting_cmds::sun_for,
            shortcut_cmds::tag_elements,
            shortcut_cmds::tag_room_in_view,
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
            lines_cmds::create_lines,
            lines_cmds::lines_preview,
            model_edit_cmds::model_edit_preview,
            model_edit_cmds::model_edit_apply,
            qa_cmds::qa_review,
            qa_cmds::qa_claude,
            qa_cmds::qa_fix_claude,
            qa_cmds::qa_export_pdf,
            qa_cmds::qa_fix_plan,
            qa_cmds::qa_fix_apply,
            group_cmds::group_create,
            group_cmds::group_ungroup,
            group_cmds::group_place,
            group_cmds::group_edit,
            group_cmds::group_add,
            group_cmds::group_remove,
            group_cmds::group_finish,
            group_cmds::group_cancel,
            group_cmds::group_delete_type,
            spec_cmds::spec_state,
            spec_cmds::spec_generate,
            spec_cmds::spec_update,
            spec_cmds::spec_set_section,
            spec_cmds::spec_library_section,
            spec_cmds::spec_add_library,
            spec_cmds::spec_add_custom,
            spec_cmds::spec_remove,
            spec_cmds::spec_set_included,
            spec_cmds::spec_set_settings,
            spec_cmds::spec_export,
            spec_cmds::spec_edit_preview,
            spec_cmds::spec_edit_apply,
            project_cmds::project_info_get,
            project_cmds::project_info_set,
            standards_cmds::standards_get,
            standards_cmds::standards_libraries,
            standards_cmds::standards_set,
            standards_cmds::standards_load_library,
            standards_cmds::standards_choices,
            symbols_cmds::create_spot_elevation,
            symbols_cmds::create_north_arrow,
            symbols_cmds::create_graphic_scale,
            symbols_cmds::create_key_plan,
            symbols_cmds::create_spot_slope,
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
            sketching::sketch_set_form,
            sketching::sketch_set_pattern,
            detail_cmds::detail_library,
            detail_cmds::detail_component_types,
            detail_cmds::detail_component_preview,
            detail_cmds::create_detail_component,
            detail_cmds::detail_preview,
            detail_cmds::detail_insert,
            detail_cmds::create_drafting_view,
            detail_cmds::detail_save,
            detail_cmds::create_plan_view,
            detail_cmds::create_3d_view,
            detail_cmds::duplicate_view,
            detail_cmds::duplicate_sheet,
            detail_cmds::detail_delete,
            inplace_cmds::in_place_categories,
            inplace_cmds::in_place_of,
            inplace_cmds::in_place_default_name,
            inplace_cmds::in_place_begin,
            inplace_cmds::in_place_edit,
            inplace_cmds::in_place_finish,
            inplace_cmds::in_place_cancel,
            inplace_cmds::in_place_form_begin,
            inplace_cmds::in_place_delete_form,
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
