//! Native menu bar. Item clicks are forwarded to the UI as a `menu` event carrying the
//! item id; the UI owns the file dialogs and calls back into the project commands.

use tauri::menu::{Menu, MenuBuilder, MenuItemBuilder, SubmenuBuilder};
use tauri::{AppHandle, Runtime};

/// Event name the UI listens on.
pub const MENU_EVENT: &str = "menu";

/// Menu item ids, mirrored in `app/src/fileActions.ts`.
pub const FILE_NEW: &str = "file.new";
pub const FILE_OPEN: &str = "file.open";
pub const FILE_SAVE: &str = "file.save";
pub const FILE_SAVE_AS: &str = "file.save_as";

pub fn build<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let item = |id: &str, label: &str, accel: &str| {
        MenuItemBuilder::with_id(id, label)
            .accelerator(accel)
            .build(app)
    };
    let file = SubmenuBuilder::new(app, "&File")
        .item(&item(FILE_NEW, "&New", "CmdOrCtrl+N")?)
        .item(&item(FILE_OPEN, "&Open…", "CmdOrCtrl+O")?)
        .separator()
        .item(&item(FILE_SAVE, "&Save", "CmdOrCtrl+S")?)
        .item(&item(FILE_SAVE_AS, "Save &As…", "CmdOrCtrl+Shift+S")?)
        .separator()
        .quit_with_text("E&xit")
        .build()?;
    MenuBuilder::new(app).item(&file).build()
}
