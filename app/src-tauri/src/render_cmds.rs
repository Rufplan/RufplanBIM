//! IPC commands for cameras and renderings (ADR-027). Thin wrappers over studio-core.

use studio_core::camera::{self, CameraPose, SunPosition};
use studio_core::ElementId;
use studio_geom::Pt;
use tauri::{State, WebviewWindow};

use crate::commands::{finish, lock, CommandError, SessionState};
use crate::session::AppState;

type StateResult = Result<Option<AppState>, CommandError>;
type CommandResult<T> = Result<T, CommandError>;

/// Camera tool: a perspective view from `eye` toward `target` on the plan's level, `height`
/// above it. The new view is the last camera view in the state.
#[tauri::command]
pub fn create_camera(
    view: ElementId,
    eye: Pt,
    target: Pt,
    height: f64,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    let mut s = lock(&state)?;
    let level = s.view_level(view)?;
    s.edit(|d| camera::create_camera(d, level, eye, target, height))?;
    finish(&window, &s)
}

/// Saves a camera view's pose after orbiting or zooming it.
#[tauri::command]
pub fn set_camera_pose(
    view: ElementId,
    pose: CameraPose,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    let mut s = lock(&state)?;
    s.edit(|d| camera::set_pose(d, view, &pose))?;
    finish(&window, &s)
}

/// The sun at the site on a month and day at `hour` (local standard time).
#[tauri::command]
pub fn sun_position(
    month: u32,
    day: u32,
    hour: f64,
    state: State<'_, SessionState>,
) -> CommandResult<SunPosition> {
    let s = lock(&state)?;
    Ok(camera::project_sun(s.doc()?, month, day, hour))
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            let hex = std::str::from_utf8(&b[i + 1..i + 3]).unwrap_or("");
            if let Ok(v) = u8::from_str_radix(hex, 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Writes a rendered image. The bytes come as the raw request body and the file path
/// (percent-encoded) in the `path` header, so a 4K PNG isn't sent as JSON.
#[tauri::command]
pub fn save_render(request: tauri::ipc::Request<'_>) -> CommandResult<()> {
    let tauri::ipc::InvokeBody::Raw(bytes) = request.body() else {
        return Err(anyhow::anyhow!("expected the image's bytes").into());
    };
    let path = request
        .headers()
        .get("path")
        .and_then(|v| v.to_str().ok())
        .map(percent_decode)
        .ok_or_else(|| anyhow::anyhow!("no file chosen"))?;
    let lower = path.to_lowercase();
    if !(lower.ends_with(".png") || lower.ends_with(".jpg") || lower.ends_with(".jpeg")) {
        return Err(anyhow::anyhow!("save renderings as .png or .jpg").into());
    }
    std::fs::write(&path, bytes).map_err(|e| anyhow::anyhow!("couldn't save {path}: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn paths_decode_from_the_header() {
        assert_eq!(
            super::percent_decode("C%3A%5CRenders%5CHouse%20%C3%A9.png"),
            "C:\\Renders\\House é.png"
        );
        assert_eq!(super::percent_decode("a%2"), "a%2");
    }
}
