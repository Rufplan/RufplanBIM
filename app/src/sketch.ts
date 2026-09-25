// Sketch mode actions (ADR-021): entering, finishing and editing a boundary sketch. The
// sketch lives in Rust; these wrap the IPC calls with the UI's settings.
import type { DrawOptions } from "./bindings/DrawOptions";
import { apply } from "./fileActions";
import { ipc, type ElementId } from "./ipc";
import { activeViewInfo, useAppStore } from "./store";

/** Floor (SB) or Sketch Ceiling (CS): enters sketch mode for a new one on this plan. */
export async function startSketch(kind: "Floor" | "Ceiling") {
  const s = useAppStore.getState();
  const v = activeViewInfo(s);
  // In 3D, floors and ceilings are placed by picking (Pick Walls, Auto Room).
  if (v?.viewType === "ThreeD") {
    s.setTool(kind === "Floor" ? "floorAuto" : "ceilingAuto");
    return;
  }
  if (!v || (v.viewType !== "Plan" && v.viewType !== "CeilingPlan")) {
    s.setError("Open a floor or ceiling plan to sketch a boundary.");
    return;
  }
  const type = kind === "Floor" ? s.toolTypes.floor : s.toolTypes.ceiling;
  // Revit starts floors with Pick Walls and ceilings with Line.
  s.setSketchUi({ mode: kind === "Floor" ? "PickWalls" : "Line", sel: [], tab: false });
  await apply(() => ipc.sketchBegin(v.id, kind, null, type));
}

/** Edit Boundary of a floor or ceiling (also on double-click). */
export async function editBoundary(id: ElementId) {
  const s = useAppStore.getState();
  const v = activeViewInfo(s);
  try {
    const sheet = await ipc.properties(id);
    if (sheet.category !== "Floor" && sheet.category !== "Ceiling") {
      s.setError("Select a floor or ceiling to edit its boundary.");
      return;
    }
    if (!v) return;
    s.setSketchUi({ mode: "Modify", sel: [], tab: false });
    await apply(() =>
      ipc.sketchBegin(v.id, sheet.category === "Floor" ? "Floor" : "Ceiling", id, null),
    );
  } catch {
    s.setError("Select a floor or ceiling to edit its boundary.");
  }
}

/** Finish Edit Mode (✓). Shows Revit's message and highlights the lines if the sketch
 * isn't valid. */
export async function finishSketchMode() {
  await apply(() => ipc.sketchFinish());
  const s = useAppStore.getState();
  const err = s.app?.sketch?.error;
  if (err) s.setError(err);
}

export const cancelSketchMode = () => apply(() => ipc.sketchCancel());

export function deleteSketchSelection() {
  const s = useAppStore.getState();
  const sel = s.sketchUi.sel;
  if (sel.length === 0) return;
  s.setSketchUi({ sel: [] });
  void apply(() => ipc.sketchDelete(sel));
}

export function flipSketchSelection() {
  const sel = useAppStore.getState().sketchUi.sel;
  if (sel.length > 0) void apply(() => ipc.sketchFlip(sel));
}

async function len(text: string, fallback: number): Promise<number> {
  const t = text.trim();
  if (!t) return fallback;
  const mm = await ipc.parseLength(t);
  return mm ?? fallback;
}

/** The options bar's settings for the draw tools, in mm. */
export async function drawOptions(): Promise<DrawOptions> {
  const ui = useAppStore.getState().sketchUi;
  return {
    offset: await len(ui.offset, 0),
    radius: ui.radiusOn ? await len(ui.radius, 304.8) : null,
    sides: Math.max(3, Math.min(64, Math.round(ui.sides) || 6)),
  };
}

/** Fillet Arc's radius (the options bar's, or 1'-0"). */
export async function filletRadius(): Promise<number> {
  return len(useAppStore.getState().sketchUi.radius, 304.8);
}
