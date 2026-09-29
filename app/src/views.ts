// Views and sheets as Revit's project browser handles them (ADR-074): new views, Duplicate
// View, Duplicate Sheet, Rename and Delete. The copying is Rust's.
import type { AppState } from "./bindings/AppState";
import type { ElementId } from "./bindings/ElementId";
import type { SheetCopy } from "./bindings/SheetCopy";
import { apply } from "./fileActions";
import { errorMessage, ipc } from "./ipc";
import { useAppStore } from "./store";

/** Runs a command that makes a view or sheet, then opens it. */
async function make(run: () => Promise<[ElementId, AppState | null]>) {
  const s = useAppStore.getState();
  try {
    const [id, state] = await run();
    if (state) s.setApp(state);
    useAppStore.getState().openView(id);
    return true;
  } catch (e) {
    s.setError(errorMessage(e));
    return false;
  }
}

export const newPlanView = (level: ElementId, ceiling: boolean) =>
  make(() => ipc.createPlanView(level, ceiling));

export const new3dView = () => make(() => ipc.create3dView());

export const duplicateView = (view: ElementId, detailing: boolean) =>
  make(() => ipc.duplicateView(view, detailing));

export const duplicateSheet = (sheet: ElementId, how: SheetCopy) =>
  make(() => ipc.duplicateSheet(sheet, how));

/** Deletes a view or sheet (undoable); its tab closes. */
export async function deleteView(id: ElementId) {
  const s = useAppStore.getState();
  if (s.openViews.includes(id)) s.closeView(id);
  await apply(() => ipc.deleteElements([id]));
}

/** Properties: the view's (or sheet's) parameters in the Properties palette. */
export function viewProperties(id: ElementId) {
  const s = useAppStore.getState();
  s.select([id]);
  s.setUi({ propsHidden: false });
}
