import { create } from "zustand";
import type { AppState, CloudStatus, ElementId } from "./ipc";

// UI state only. The model lives in Rust; `app` mirrors the last snapshot it returned.

export type Tool =
  | "copy"
  | "rotate"
  | "mirror"
  | "array"
  | "align"
  | "trim"
  | "offset"
  | "split"
  | "roof"
  | "stair"
  | "column"
  | "beam"
  | "railing"
  | "roomSeparator"
  | "callout"
  | "elevation"
  | "sketch"
  | "dimension"
  | "text"
  | "section"
  | "room"
  | "move"
  | "door"
  | "window"
  | "select"
  | "wall"
  | "grid"
  | "floor"
  | "floorAuto"
  | "ceiling"
  | "ceilingAuto"
  | "level";

export const TOOL_LABELS: Record<Tool, string> = {
  copy: "Copy",
  rotate: "Rotate",
  mirror: "Mirror",
  array: "Array",
  align: "Align",
  trim: "Trim/Extend",
  offset: "Offset",
  split: "Split",
  roof: "Roof",
  stair: "Stair",
  column: "Column",
  beam: "Beam",
  railing: "Railing",
  roomSeparator: "Room Separator",
  callout: "Callout",
  elevation: "Elevation",
  sketch: "Boundary Sketch",
  select: "Select",
  room: "Room",
  move: "Move",
  dimension: "Dimension",
  text: "Text",
  section: "Section",
  door: "Door",
  window: "Window",
  wall: "Wall",
  grid: "Grid",
  floor: "Floor: Sketch",
  floorAuto: "Floor: Pick Walls",
  ceiling: "Ceiling: Sketch",
  ceilingAuto: "Ceiling: Auto Room",
  level: "Level",
};

/** Which element types the active tools place, by category. */
export interface ToolTypes {
  wall: ElementId | null;
  floor: ElementId | null;
  ceiling: ElementId | null;
  door: ElementId | null;
  window: ElementId | null;
  roof: ElementId | null;
  column: ElementId | null;
  beam: ElementId | null;
  railing: ElementId | null;
}

/** Options-bar settings of the modify tools (like Revit's options bar). */
export interface ToolOptions {
  /** Copy: keep placing copies from the same base point. */
  copyMultiple: boolean;
  /** Rotate: rotate a copy instead of the selection. */
  rotateCopy: boolean;
  /** Mirror: mirror a copy (Revit's default) instead of the selection. */
  mirrorCopy: boolean;
  /** Array: number of items including the original. */
  arrayCount: number;
  /** Offset distance as typed (feet-inches). */
  offsetDistance: string;
  /** Wall: which line of the wall the drawn points follow (a LocationLine variant). */
  wallLocation: string;
  /** Stair: shape id (straight, l-left, l-right, u-left, u-right). */
  stairShape: string;
}

/** Revit's boundary line tools in sketch mode (ADR-021), plus Modify and Trim. */
export type SketchMode =
  | "Modify"
  | "Line"
  | "Rectangle"
  | "InscribedPolygon"
  | "CircumscribedPolygon"
  | "Circle"
  | "StartEndRadiusArc"
  | "CenterEndsArc"
  | "FilletArc"
  | "PickLines"
  | "PickWalls"
  | "Trim";

/** Sketch mode's UI settings (the sketch itself lives in Rust: `app.sketch`). */
export interface SketchUi {
  mode: SketchMode;
  /** Selected sketch curves (indices). */
  sel: number[];
  /** Line: keep drawing from the last point. */
  chain: boolean;
  offset: string;
  radiusOn: boolean;
  radius: string;
  sides: number;
  /** Pick Walls: Extend into wall (to core). */
  core: boolean;
  /** Pick Lines: lock to the wall. */
  lock: boolean;
  /** Pick Walls: Tab picks the whole chain of walls under the cursor. */
  tab: boolean;
}

/** Tools that act on the current selection. */
export const SELECTION_TOOLS: Tool[] = ["move", "copy", "rotate", "mirror", "array"];

/** A pending Save / Don't Save / Cancel question and what to do after it. */
export interface Confirm {
  message: string;
  resolve: (choice: "save" | "discard" | "cancel") => void;
}

/** Which Rufplan dialog is open. */
export type RufplanDialogMode = "account" | "link" | "publish";

interface UiState {
  app: AppState | null;
  error: string | null;
  openViews: ElementId[];
  activeView: ElementId | null;
  /** When nothing is selected, the properties panel shows the active view. */
  selection: ElementId[];
  tool: Tool;
  toolTypes: ToolTypes;
  /** Prompt shown in the status bar by the active tool. */
  prompt: string;
  cursor: string;
  confirm: Confirm | null;
  rufplan: RufplanDialogMode | null;
  cloud: CloudStatus | null;
  options: ToolOptions;
  /** The Project Parameters dialog is open. */
  paramsOpen: boolean;
  sketchUi: SketchUi;
  setSketchUi: (patch: Partial<SketchUi>) => void;
  /** Elevation tool: the mark type to place (its family type decides interior/building). */
  elevationType: ElementId | null;
  setElevationType: (id: ElementId | null) => void;
  /** 3D view: the ground plane's cyan grid is shown. */
  grid3d: boolean;
  setGrid3d: (on: boolean) => void;
  /** 3D view: the level walls and columns are placed on. */
  level3d: ElementId | null;
  setLevel3d: (id: ElementId | null) => void;

  /** `fresh` = a different project was just created or opened. */
  setApp: (app: AppState | null, fresh?: boolean) => void;
  setError: (error: string | null) => void;
  openView: (id: ElementId) => void;
  closeView: (id: ElementId) => void;
  select: (ids: ElementId[]) => void;
  setTool: (tool: Tool) => void;
  setToolType: (kind: keyof ToolTypes, id: ElementId) => void;
  setPrompt: (prompt: string) => void;
  setCursor: (cursor: string) => void;
  setConfirm: (confirm: Confirm | null) => void;
  setRufplan: (mode: RufplanDialogMode | null) => void;
  setCloud: (cloud: CloudStatus | null) => void;
  setOption: <K extends keyof ToolOptions>(key: K, value: ToolOptions[K]) => void;
  setParamsOpen: (open: boolean) => void;
}

const firstId = (items: { id: ElementId }[] | undefined, current: ElementId | null) =>
  current && items?.some((i) => i.id === current) ? current : (items?.[0]?.id ?? null);

export const useAppStore = create<UiState>((set, get) => ({
  app: null,
  error: null,
  openViews: [],
  activeView: null,
  selection: [],
  tool: "select",
  toolTypes: {
    wall: null,
    floor: null,
    ceiling: null,
    door: null,
    window: null,
    roof: null,
    column: null,
    beam: null,
    railing: null,
  },
  prompt: "",
  cursor: "",
  confirm: null,
  rufplan: null,
  cloud: null,
  options: {
    copyMultiple: false,
    rotateCopy: false,
    mirrorCopy: true,
    arrayCount: 3,
    offsetDistance: "2'-0\"",
    wallLocation: "Centerline",
    stairShape: "straight",
  },
  paramsOpen: false,
  sketchUi: {
    mode: "PickWalls",
    sel: [],
    chain: true,
    offset: '0"',
    radiusOn: false,
    radius: "1'-0\"",
    sides: 6,
    core: false,
    lock: true,
    tab: false,
  },
  setSketchUi: (patch) => set((s) => ({ sketchUi: { ...s.sketchUi, ...patch } })),
  elevationType: null,
  setElevationType: (elevationType) => set({ elevationType }),
  grid3d: true,
  setGrid3d: (grid3d) => set({ grid3d }),
  level3d: null,
  setLevel3d: (level3d) => set({ level3d }),

  setApp: (app, fresh = false) => {
    const s = get();
    if (!app) {
      set({ app: null, openViews: [], activeView: null, selection: [] });
      return;
    }
    const exists = new Set(app.views.map((v) => v.id));
    let openViews = s.openViews.filter((v) => exists.has(v));
    // A different project (or first load): open its first floor plan.
    const sameProject = !fresh && s.app !== null;
    if (!sameProject || openViews.length === 0) {
      const first = app.views[0]?.id;
      openViews = first ? [first] : [];
    }
    const activeView =
      s.activeView && openViews.includes(s.activeView) ? s.activeView : (openViews[0] ?? null);
    // Sketch mode follows the model's sketch session (ADR-021).
    const tool: Tool = app.sketch ? "sketch" : s.tool === "sketch" ? "select" : s.tool;
    const curves = app.sketch?.curves.length ?? 0;
    set({
      app,
      error: null,
      openViews,
      activeView,
      tool,
      sketchUi: { ...s.sketchUi, sel: s.sketchUi.sel.filter((i) => i < curves) },
      selection: sameProject && !app.sketch ? s.selection : [],
      toolTypes: {
        wall: firstId(app.wallTypes, s.toolTypes.wall),
        floor: firstId(app.floorTypes, s.toolTypes.floor),
        ceiling: firstId(app.ceilingTypes, s.toolTypes.ceiling),
        // Single doors are the everyday default, even though "Double" sorts first.
        door: firstId(
          [...app.doorTypes].sort(
            (a, b) => Number(!a.name.startsWith("Single")) - Number(!b.name.startsWith("Single")),
          ),
          s.toolTypes.door,
        ),
        window: firstId(app.windowTypes, s.toolTypes.window),
        roof: firstId(app.roofTypes, s.toolTypes.roof),
        // Structural columns and steel beams are the everyday defaults.
        column: firstId(
          [...app.columnTypes].sort(
            (a, b) => Number(!a.name.startsWith("Steel")) - Number(!b.name.startsWith("Steel")),
          ),
          s.toolTypes.column,
        ),
        beam: firstId(
          [...app.beamTypes].sort(
            (a, b) => Number(!a.name.startsWith("Steel")) - Number(!b.name.startsWith("Steel")),
          ),
          s.toolTypes.beam,
        ),
        railing: firstId(app.railingTypes, s.toolTypes.railing),
      },
    });
  },
  setError: (error) => set({ error }),
  openView: (id) =>
    set((s) => ({
      openViews: s.openViews.includes(id) ? s.openViews : [...s.openViews, id],
      activeView: id,
      selection: [],
      tool: "select",
    })),
  closeView: (id) =>
    set((s) => {
      const openViews = s.openViews.filter((v) => v !== id);
      const activeView =
        s.activeView === id ? (openViews[openViews.length - 1] ?? null) : s.activeView;
      return { openViews, activeView };
    }),
  select: (selection) => set({ selection }),
  // Move, Copy, Rotate, Mirror and Array act on the current selection; other tools start
  // with nothing selected.
  setTool: (tool) =>
    set({
      tool,
      selection: tool === "select" || SELECTION_TOOLS.includes(tool) ? get().selection : [],
    }),
  setToolType: (kind, id) => set((s) => ({ toolTypes: { ...s.toolTypes, [kind]: id } })),
  setPrompt: (prompt) => set({ prompt }),
  setCursor: (cursor) => set({ cursor }),
  setConfirm: (confirm) => set({ confirm }),
  setRufplan: (rufplan) => set({ rufplan }),
  setCloud: (cloud) => set({ cloud }),
  setOption: (key, value) => set((s) => ({ options: { ...s.options, [key]: value } })),
  setParamsOpen: (paramsOpen) => set({ paramsOpen }),
}));

/** Info about the active view, if any. */
export function activeViewInfo(s: UiState) {
  return s.app?.views.find((v) => v.id === s.activeView) ?? null;
}
