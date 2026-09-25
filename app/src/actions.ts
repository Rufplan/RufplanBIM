// The commands behind Revit's shortcuts that aren't drawing tools (ADR-024).
import { apply, deleteSelection } from "./fileActions";
import { ipc } from "./ipc";
import { startSketch } from "./sketch";
import type { Action } from "./shortcuts";
import { activeViewInfo, useAppStore, type Tool, type ToolTypes } from "./store";

/** Which tool (and type slot) creates more of a category, for Create Similar. */
const SIMILAR: Record<string, [Tool, keyof ToolTypes | null]> = {
  Wall: ["wall", "wall"],
  Door: ["door", "door"],
  Window: ["window", "window"],
  Floor: ["floorAuto", "floor"],
  Ceiling: ["ceilingAuto", "ceiling"],
  Roof: ["roof", "roof"],
  Column: ["column", "column"],
  Beam: ["beam", "beam"],
  Railing: ["railing", "railing"],
  Room: ["room", null],
  Grid: ["grid", null],
  Level: ["level", null],
  Stair: ["stair", null],
  RoomSeparator: ["roomSeparator", null],
  Dimension: ["dimension", null],
  TextNote: ["text", null],
};

async function selectedCategories(view: string, ids: string[]): Promise<string[]> {
  const cats = await ipc.viewCategories(view);
  const set = new Set(ids);
  return [...new Set(cats.filter(([id]) => set.has(id)).map(([, c]) => c))];
}

export async function runAction(action: Action) {
  const s = useAppStore.getState();
  const view = activeViewInfo(s);
  const sel = s.selection;
  const need = (what: string) => {
    if (sel.length === 0) {
      s.setError(`Select elements first, then ${what}.`);
      return false;
    }
    return true;
  };
  switch (action) {
    case "repeat":
      if (s.lastTool) s.setTool(s.lastTool);
      return;
    case "createSimilar": {
      if (!need("Create Similar")) return;
      const sheet = await ipc.properties(sel[0]!);
      const hit = SIMILAR[sheet.category];
      if (!hit) {
        s.setError(`Create Similar doesn't apply to ${sheet.category}.`);
        return;
      }
      const [tool, slot] = hit;
      if (slot && sheet.typeId) s.setToolType(slot, sheet.typeId);
      s.setTool(tool);
      return;
    }
    case "selectAll":
      if (!need("Select All Instances")) return;
      s.select(await ipc.selectAllInstances(sel[0]!));
      return;
    case "pin":
    case "unpin":
      if (!need(action === "pin" ? "pin them" : "unpin them")) return;
      await apply(() => ipc.setPinned(sel, action === "pin"));
      return;
    case "delete":
      await deleteSelection();
      return;
    case "floor":
      await startSketch("Floor");
      return;
    case "hideElement":
    case "isolateElement":
    case "hideCategory":
    case "isolateCategory": {
      if (!view || !need("hide or isolate")) return;
      const byCat = action === "hideCategory" || action === "isolateCategory";
      const isolate = action.startsWith("isolate");
      const categories = byCat ? await selectedCategories(view.id, sel) : [];
      s.setTempHide(view.id, { isolate, ids: byCat ? [] : [...sel], categories });
      s.select([]);
      return;
    }
    case "resetTemporary":
      if (view) s.setTempHide(view.id, null);
      return;
    case "hideInView":
      if (!view || !need("hide them in the view")) return;
      await apply(() => ipc.hideElements(view.id, sel));
      s.select([]);
      return;
    case "hideCategoryInView": {
      if (!view || !need("hide their category in the view")) return;
      const cats = await selectedCategories(view.id, sel);
      await apply(() => ipc.setCategoryVisible(view.id, cats as never, false));
      s.select([]);
      return;
    }
    case "visibility":
      s.setUi({ viewDialog: "visibility" });
      return;
    case "keyboard":
      s.setUi({ viewDialog: "keyboard" });
      return;
    case "snapEndpoint":
      s.setSnapOverride("Endpoint");
      return;
    case "snapMidpoint":
      s.setSnapOverride("Midpoint");
      return;
    case "snapIntersection":
      s.setSnapOverride("Intersection");
      return;
    case "snapPerpendicular":
      s.setSnapOverride("Perpendicular");
      return;
    case "snapNearest":
      s.setSnapOverride("Nearest");
      return;
    case "snapOff":
      s.setSnapOverride("None");
      return;
    case "snapOverrideOff":
      s.setSnapOverride(null);
      return;
    case "zoomFit":
      window.dispatchEvent(new Event("view-fit"));
      return;
    case "zoomOut":
      window.dispatchEvent(new Event("view-zoom-out"));
      return;
    case "zoomRegion":
      window.dispatchEvent(new Event("view-zoom-region"));
      return;
    case "zoomPrevious":
      window.dispatchEvent(new Event("view-zoom-previous"));
      return;
    case "thinLines":
      s.setUi({ thinLines: !s.thinLines });
      return;
    case "wireframe":
    case "hiddenLine":
    case "shaded":
      s.setUi({ visualStyle: action });
      return;
    case "properties":
      s.setUi({ propsHidden: !s.propsHidden });
      return;
    case "viewProperties":
      s.select([]);
      s.setUi({ propsHidden: false });
      return;
  }
}

/** Status text for a pending snap override. */
export function snapOverrideLabel(k: string | null): string {
  if (!k) return "";
  return k === "None" ? "Snaps off for the next pick" : `Snap to ${k} for the next pick`;
}
