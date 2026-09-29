// The Details tab (ADR-069): drafting views, the Detail Library and filled regions.
import type { ElementId } from "./bindings/ElementId";
import type { FillPattern } from "./bindings/FillPattern";
import { apply } from "./fileActions";
import { errorMessage, ipc } from "./ipc";
import { activeViewInfo, useAppStore } from "./store";

/** Inserts a library detail as a drafting view and opens it. */
export async function insertDetail(id: string) {
  const s = useAppStore.getState();
  try {
    const [view, state] = await ipc.detailInsert(id);
    if (state) s.setApp(state);
    useAppStore.getState().openView(view);
    return true;
  } catch (e) {
    s.setError(errorMessage(e));
    return false;
  }
}

/** View > Drafting View: a new empty one, opened. */
export async function createDraftingView(name: string, scale: number) {
  const s = useAppStore.getState();
  try {
    const [view, state] = await ipc.createDraftingView(name, scale);
    if (state) s.setApp(state);
    useAppStore.getState().openView(view);
    return true;
  } catch (e) {
    s.setError(errorMessage(e));
    return false;
  }
}

/** Filled Region: sketch its boundary in this 2D view (Revit starts with Line). */
export async function startFilledRegion() {
  const s = useAppStore.getState();
  const v = activeViewInfo(s);
  if (!v || v.viewType === "ThreeD" || v.viewType === "Schedule" || v.viewType === "Sheet") {
    s.setError("Open a plan, section, elevation or drafting view to sketch a filled region.");
    return false;
  }
  s.setSketchUi({ mode: "Line", sel: [], tab: false });
  return apply(() => ipc.sketchBegin(v.id, "FilledRegion", null, null));
}

/** Edit Boundary of a filled region. */
export async function editFilledRegion(id: ElementId) {
  const s = useAppStore.getState();
  const v = activeViewInfo(s);
  if (!v) return false;
  s.setSketchUi({ mode: "Modify", sel: [], tab: false });
  return apply(() => ipc.sketchBegin(v.id, "FilledRegion", id, null));
}

export const setRegionPattern = (p: FillPattern) => apply(() => ipc.sketchSetPattern(p));

/** The patterns as Revit names them. */
export const FILL_PATTERNS: [FillPattern, string][] = [
  ["Solid", "Solid Fill"],
  ["Gray", "Solid Gray"],
  ["Diagonal", "Diagonal Up"],
  ["CrossHatch", "Crosshatch"],
  ["Concrete", "Concrete"],
  ["Earth", "Earth"],
  ["Gravel", "Gravel"],
  ["Sand", "Sand / Gypsum"],
  ["Masonry", "Masonry - Brick"],
  ["RigidInsulation", "Rigid Insulation"],
  ["Wood", "Wood - Finish"],
  ["Steel", "Steel"],
];
