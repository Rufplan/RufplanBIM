// Model In-Place (ADR-068): Revit's in-place families. The element and its forms live in
// Rust; these wrap the IPC calls with the UI's view, level and sketch settings.
import type { Category } from "./bindings/Category";
import type { FormKind } from "./bindings/FormKind";
import { apply } from "./fileActions";
import { ipc, type ElementId } from "./ipc";
import { activeViewInfo, useAppStore } from "./store";

/** The forms the editor's Create panel makes, as Revit names them. */
export type FormName = "Extrusion" | "Blend" | "Sweep" | "VoidExtrusion";

/** Model In-Place: opens Revit's Family Category and Parameters dialog. */
export function modelInPlace() {
  const s = useAppStore.getState();
  if (s.app?.inPlace) {
    s.setError("Finish or cancel the model you're editing first.");
    return;
  }
  if (!activeViewInfo(s)) {
    s.setError("Open a view to model in place.");
    return;
  }
  s.setUi({ viewDialog: "inPlace" });
}

/** Creates the element and opens the In-Place Editor on it. */
export async function beginInPlace(category: Category, name: string) {
  const s = useAppStore.getState();
  const v = activeViewInfo(s);
  if (!v) return false;
  const level = v.viewType === "ThreeD" ? (s.level3d ?? s.app?.levels[0]?.id ?? null) : null;
  s.select([]);
  return apply(() => ipc.inPlaceBegin(v.id, category, name.trim() || null, level));
}

/** Edit In-Place of a selected in-place element (also on double-click). */
export async function editInPlace(id: ElementId) {
  const s = useAppStore.getState();
  s.select([]);
  return apply(() => ipc.inPlaceEdit(id));
}

/** Sketches a new form (or `index`'s sketch again), in plan. Revit starts a form's sketch
 * with Rectangle; a sweep's path with Line. */
export async function startForm(kind: FormName, index: number | null = null) {
  const s = useAppStore.getState();
  const v = activeViewInfo(s);
  if (!v) return false;
  s.setSketchUi({
    mode: index !== null ? "Modify" : kind === "Sweep" ? "Line" : "Rectangle",
    sel: [],
    tab: false,
  });
  const ok = await apply(() => ipc.inPlaceFormBegin(v.id, kind, index));
  // Forms are sketched in plan (or in 3D on the level's work plane): from an elevation or
  // section, go to the level's plan the sketch is in.
  const sketchView = useAppStore.getState().app?.sketch?.view;
  if (ok && sketchView && v.viewType !== "ThreeD" && sketchView !== v.id)
    useAppStore.getState().openView(sketchView);
  return ok;
}

export const deleteForm = (index: number) => apply(() => ipc.inPlaceDeleteForm(index));

/** Finish Model (✓). */
export const finishModel = () => apply(() => ipc.inPlaceFinish());

/** Cancel Model (✗). */
export const cancelModel = () => apply(() => ipc.inPlaceCancel());

/** Changes the form being sketched: its heights, or a sweep's profile. */
export const setForm = (kind: FormKind) => apply(() => ipc.sketchSetForm(kind));

/** The kind a form label names ("Void Extrusion 1" is an extrusion). */
export function formName(label: string): FormName {
  if (label.startsWith("Void")) return "VoidExtrusion";
  if (label.startsWith("Blend")) return "Blend";
  if (label.startsWith("Sweep")) return "Sweep";
  return "Extrusion";
}

/** A length as feet and inches to the nearest 1/8" (5' 6", 0' 4 1/2"). */
export function ftIn(mm: number): string {
  const sign = mm < 0 ? "-" : "";
  const eighths = Math.round((Math.abs(mm) / 25.4) * 8);
  const ft = Math.floor(eighths / 96);
  const rest = eighths - ft * 96;
  const whole = Math.floor(rest / 8);
  let frac = rest % 8;
  let den = 8;
  while (frac > 0 && frac % 2 === 0) {
    frac /= 2;
    den /= 2;
  }
  return `${sign}${ft}' ${whole}${frac ? ` ${frac}/${den}` : ""}"`;
}
