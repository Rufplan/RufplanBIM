//! IPC for detail lines and model lines (ADR-054): thin wrappers over studio-core `lines`.

use studio_core::lines::{self, LineStyle, LinesOn};
use studio_core::sketch::{self, DrawOptions, DrawTool, SketchCurve};
use studio_core::{ElementData, ElementId, ViewKind};
use studio_geom::Pt;
use tauri::{State, WebviewWindow};

use crate::commands::{edit, SessionState, StateResult};

/// Draws lines with a sketch tool: detail lines in `view`, or model lines on its level.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn create_lines(
    view: ElementId,
    model: bool,
    tool: DrawTool,
    pts: Vec<Pt>,
    options: DrawOptions,
    style: LineStyle,
    window: WebviewWindow,
    state: State<'_, SessionState>,
) -> StateResult {
    edit(&window, &state, |s| {
        s.edit(|d| {
            let on = if model {
                match d.data(view)? {
                    ElementData::View {
                        kind: ViewKind::FloorPlan { level } | ViewKind::CeilingPlan { level },
                        ..
                    } => LinesOn::Level(*level),
                    _ => {
                        return Err(studio_core::CoreError::Invalid(
                            "draw model lines in a plan (they go on its level)".into(),
                        ))
                    }
                }
            } else {
                LinesOn::View(view)
            };
            lines::create_lines(d, on, tool, &pts, &options, style).map(|_| ())
        })
    })
}

/// What a draw tool would make with `pts` and the cursor, for the rubber band.
#[tauri::command]
pub fn lines_preview(
    tool: DrawTool,
    pts: Vec<Pt>,
    cursor: Pt,
    options: DrawOptions,
) -> Vec<Vec<Pt>> {
    let mut all = pts;
    all.push(cursor);
    while all.len() < tool.points() {
        all.push(cursor);
    }
    all.truncate(tool.points());
    sketch::draw(tool, &all, &options)
        .unwrap_or_default()
        .iter()
        .map(SketchCurve::points)
        .collect()
}
