//! Native menu bar. Item clicks are forwarded to the UI as a `menu` event carrying the
//! item id; the UI owns dialogs and calls back into the commands.

use tauri::menu::{Menu, MenuBuilder, MenuItemBuilder, SubmenuBuilder};
use tauri::{AppHandle, Runtime};

/// Event name the UI listens on.
pub const MENU_EVENT: &str = "menu";

// Menu item ids, mirrored in `app/src/fileActions.ts`.
pub const FILE_NEW: &str = "file.new";
pub const FILE_OPEN: &str = "file.open";
pub const FILE_SAVE: &str = "file.save";
pub const FILE_SAVE_AS: &str = "file.save_as";
pub const FILE_EXIT: &str = "file.exit";
pub const FILE_EXPORT_PDF: &str = "file.export_pdf";
pub const FILE_EXPORT_IFC: &str = "file.export_ifc";
pub const EDIT_UNDO: &str = "edit.undo";
pub const EDIT_REDO: &str = "edit.redo";
pub const EDIT_DELETE: &str = "edit.delete";

pub fn build<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let item = |id: &str, label: &str, accel: Option<&str>| {
        let b = MenuItemBuilder::with_id(id, label);
        match accel {
            Some(a) => b.accelerator(a).build(app),
            None => b.build(app),
        }
    };
    let file = SubmenuBuilder::new(app, "&File")
        .item(&item(FILE_NEW, "&New", Some("CmdOrCtrl+N"))?)
        .item(&item(FILE_OPEN, "&Open…", Some("CmdOrCtrl+O"))?)
        .separator()
        .item(&item(FILE_SAVE, "&Save", Some("CmdOrCtrl+S"))?)
        .item(&item(FILE_SAVE_AS, "Save &As…", Some("CmdOrCtrl+Shift+S"))?)
        .separator()
        .item(&item(FILE_EXPORT_PDF, "&Export PDF…", Some("CmdOrCtrl+P"))?)
        .item(&item(FILE_EXPORT_IFC, "Export &IFC…", None)?)
        .separator()
        .item(&item(FILE_EXIT, "E&xit", None)?)
        .build()?;
    // Undo/redo/delete keys are handled by the UI so they keep working as normal text
    // editing keys inside input fields; the tab shows the shortcut as a hint only.
    let edit = SubmenuBuilder::new(app, "&Edit")
        .item(&item(EDIT_UNDO, "&Undo\tCtrl+Z", None)?)
        .item(&item(EDIT_REDO, "&Redo\tCtrl+Y", None)?)
        .separator()
        .item(&item(EDIT_DELETE, "&Delete\tDel", None)?)
        .build()?;
    MenuBuilder::new(app).item(&file).item(&edit).build()
}
