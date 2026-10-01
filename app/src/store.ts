import { toGroups, withMembers } from "./groups";
import { create } from "zustand";
import type { GrassKind } from "./bindings/GrassKind";
import type { TextAlign } from "./bindings/TextAlign";
import type { LeaderMode } from "./text";
import type { NavMode } from "./render/navigate";
import type { SunSettings } from "./bindings/SunSettings";
import type { AppState, CloudStatus, ElementId, Pt } from "./ipc";
import type { Discipline } from "./bindings/Discipline";
import type { SnapKind } from "./bindings/SnapKind";
import type { ImportReport } from "./bindings/ImportReport";
import type { VisualStyle } from "./render/visualStyle";
import type { Prefer } from "./bindings/Prefer";
import type { Standards } from "./bindings/Standards";
import type { DrawTool } from "./bindings/DrawTool";
import type { LineStyle } from "./bindings/LineStyle";

export type PickerCategory = "Door" | "Window" | "Light" | "Plant" | "Furniture" | "Equipment";

/** An applied Edit Model change: the prompt, what it did, and its undo step's name. */
export interface EditLogEntry {
  prompt: string;
  summary: string;
  label: string;
  undone: boolean;
}

/** What views draw as selected: the selection and any Edit Model highlight. */
export function litOf(s: {
  selection: ElementId[];
  highlight: ElementId[];
  app?: AppState | null;
}): ElementId[] {
  // A selected group lights up its members (ADR-087).
  const sel = withMembers(s.app, s.selection);
  return s.highlight.length ? [...sel, ...s.highlight] : sel;
}

// UI state only. The model lives in Rust; `app` mirrors the last snapshot it returned.

export type Tool =
  | "keynoteElement"
  | "keynoteMaterial"
  | "keynoteUser"
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
  | "light"
  | "furniture"
  | "equipment"
  | "plant"
  | "placeGroup"
  | "grassBrush"
  | "wallOpening"
  | "beam"
  | "railing"
  | "roomSeparator"
  | "callout"
  | "elevation"
  | "sketch"
  | "tag"
  | "matchType"
  | "mirrorPick"
  | "camera"
  | "dimension"
  | "dimensionLinear"
  | "dimensionAngular"
  | "text"
  | "spotElevation"
  | "detailLine"
  | "component"
  | "modelLine"
  | "spotSlope"
  | "northArrow"
  | "graphicScale"
  | "keyPlan"
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
  | "level"
  | "paint";

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
  light: "Lighting Fixture",
  furniture: "Furniture",
  equipment: "Equipment",
  plant: "Plant",
  placeGroup: "Place Group",
  grassBrush: "Grass Brush",
  wallOpening: "Wall Opening",
  beam: "Beam",
  railing: "Railing",
  roomSeparator: "Room Separator",
  callout: "Callout",
  elevation: "Elevation",
  sketch: "Boundary Sketch",
  tag: "Tag by Category",
  keynoteElement: "Element Keynote",
  keynoteMaterial: "Material Keynote",
  keynoteUser: "User Keynote",
  matchType: "Match Type Properties",
  mirrorPick: "Mirror - Pick Axis",
  camera: "Camera",
  select: "Select",
  room: "Room",
  move: "Move",
  dimension: "Aligned Dimension",
  dimensionLinear: "Linear Dimension",
  dimensionAngular: "Angular Dimension",
  text: "Text",
  spotElevation: "Spot Elevation",
  detailLine: "Detail Line",
  component: "Detail Component",
  modelLine: "Model Line",
  spotSlope: "Spot Slope",
  northArrow: "North Arrow",
  graphicScale: "Graphic Scale",
  keyPlan: "Key Plan",
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
  paint: "Paint",
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
  light: ElementId | null;
  /** Furniture and equipment (ADR-090). */
  furniture: ElementId | null;
  equipment: ElementId | null;
  plant: ElementId | null;
  /** Place Group's group type (ADR-087). */
  group: ElementId | null;
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
  /** Camera: eye height above the plan's level, as typed (Revit's Offset). */
  cameraHeight: string;
  /** Dimensions: the wall line picked from inside a wall (Revit's Prefer, ADR-040). */
  dimPrefer: Prefer;
  /** Align: the same, defaulting to wall faces (ADR-042). */
  alignPrefer: Prefer;
  /** Plant (Enscape's placement, ADR-064): turn each plant randomly, and vary its size by
   * up to this many percent. */
  plantRandomRotation: boolean;
  /** Furniture and equipment (ADR-090): the placed piece's rotation in degrees. */
  ffeRotation: number;
  plantSizeVariation: number;
  /** Grass Brush (D5's, ADR-065): the grass to paint, the brush's radius (mm), the density
   * (%), and erasing instead of painting. */
  grassKind: GrassKind;
  grassBrush: number;
  grassDensity: number;
  grassErase: boolean;
  /** Text (Revit's, ADR-070): its type (paper mm), leader and alignment. */
  textSize: number;
  textLeader: LeaderMode;
  textAlign: TextAlign;
  /** Detail Component (ADR-071): the type, a point-based one's rotation, and flipped. */
  componentKey: string;
  componentRotation: number;
  componentFlip: boolean;
  /** Section and Callout (ADR-076): Reference Other View, and the view ("" for a new
   * drafting view). */
  refOther: boolean;
  refTarget: string;
  /** Keynotes (ADR-081): the tag type, whether it has a leader, and the User keynote. */
  keynoteStyle: "Key" | "KeyAndText";
  keynoteLeader: boolean;
  keynoteUserKey: string;
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

/** Temporary Hide/Isolate: what's hidden (or, isolating, what alone is shown). */
export interface TempHide {
  isolate: boolean;
  ids: ElementId[];
  categories: string[];
}

/** Tools that act on the current selection. */
/** Tools that draw detail or model lines (ADR-054). */
export const LINE_TOOLS: Tool[] = ["detailLine", "modelLine"];

/** Revit's line styles, with their names. */
export const LINE_STYLES: [LineStyle, string][] = [
  ["Thin", "Thin Lines"],
  ["Medium", "Medium Lines"],
  ["Wide", "Wide Lines"],
  ["Hidden", "<Hidden>"],
  ["Centerline", "<Centerline>"],
  ["Overhead", "<Overhead>"],
  ["Demolished", "<Demolished>"],
  ["Beyond", "<Beyond>"],
];

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
  /** An unattended render to run (ADR-095). */
  autoRender: import("./ipc").AutoRender | null;
  activeView: ElementId | null;
  /** A viewport activated on the active sheet (ADR-039): its view is edited in place, at
   * the sheet's scale, the rest of the sheet shown faded. */
  activeViewport: ActiveViewport | null;
  activateViewport: (v: ActiveViewport) => void;
  deactivateViewport: () => void;
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
  /** The last tool used, for Repeat Last Command (RC / Enter). */
  lastTool: Tool | null;
  /** A one-pick snap override (SE, SM, SI, SP, SN; "None" = SO snaps off). */
  snapOverride: SnapKind | null;
  setSnapOverride: (k: SnapKind | null) => void;
  /** Temporary Hide/Isolate by view (HH, HI, HC, IC; HR resets). */
  tempHide: Record<ElementId, TempHide>;
  setTempHide: (view: ElementId, t: TempHide | null) => void;
  thinLines: boolean;
  /** Each 3D view's visual style (ADR-038); Shaded until chosen. */
  visualStyles: Record<ElementId, VisualStyle>;
  setVisualStyle: (view: ElementId, style: VisualStyle) => void;
  propsHidden: boolean;
  /** Keyboard Shortcuts (KS) or Visibility/Graphics (VV) dialog. */
  viewDialog:
    | "keyboard"
    | "visibility"
    | "render"
    | "materials"
    | "generate"
    | "plans"
    | "sheetSets"
    | "sunSettings"
    | "artificialLights"
    | "ground"
    | "filter"
    | "inPlace"
    | "details"
    | "draftingView"
    | "saveDetail"
    | "worksets"
    | "structure"
    | "keynotes"
    | "mep"
    | "createGroup"
    | "fascia"
    | null;
  /** The door or window type picker (ADR-033): which category, which tab, and the
   * selected doors or windows it changes. */
  picker: { category: PickerCategory; tab: "project" | "library"; change: ElementId[] } | null;
  /** The Asset Library's category and group to open on (the Vegetation tab's buttons). */
  assetFilter: { category: string; group: string | null } | null;
  setAssetFilter: (f: UiState["assetFilter"]) => void;
  /** What the last IFC import brought in (ADR-035). */
  ifcReport: ImportReport | null;
  setIfcReport: (r: ImportReport | null) => void;
  /** The material the Paint tool applies (ADR-034). */
  paintMaterial: ElementId | null;
  setPaintMaterial: (id: ElementId | null) => void;
  setPicker: (p: UiState["picker"]) => void;
  setUi: (patch: Partial<Pick<UiState, "thinLines" | "propsHidden" | "viewDialog">>) => void;
  /** Site tab dialog: Find Lot or API Keys (ADR-023). */
  siteDialog: "find" | "keys" | null;
  setSiteDialog: (d: "find" | "keys" | null) => void;
  /** 3D view: the ground plane's cyan grid is shown. */
  grid3d: boolean;
  /** Enscape-style navigation in 3D (ADR-063). */
  nav3d: NavMode;
  setNav3d: (nav3d: NavMode) => void;
  /** Sun settings being tried in the 3D view's sun panel, before they're saved. */
  sunPreview: SunSettings | null;
  setSunPreview: (sunPreview: SunSettings | null) => void;
  /** The Realistic 3D view's exposure (this session). */
  exposure3d: number;
  setExposure3d: (exposure3d: number) => void;
  setGrid3d: (on: boolean) => void;
  /** 3D terrain (ADR-045): the ground as a block of earth, its contours, their labels. */
  terrain: { solid: boolean; contours: boolean; labels: boolean };
  setTerrain: (patch: Partial<UiState["terrain"]>) => void;
  /** Site plans and 3D: Google satellite imagery over the topography (ADR-026). */
  satellite: boolean;
  setSatellite: (on: boolean) => void;
  /** 3D view: the level walls and columns are placed on. */
  level3d: ElementId | null;
  setLevel3d: (id: ElementId | null) => void;

  /** The ribbon tab shown; Standards replaces the workspace (ADR-047). */
  ribbonTab: string;
  setRibbonTab: (tab: string) => void;
  /** Collaborate > Gray Inactive Workset Graphics (ADR-079). */
  grayInactive: boolean;
  setGrayInactive: (on: boolean) => void;
  /** The structural overlay (ADR-080): shown over the greyed-out architecture, and how
   * opaque it is (0.2–1). */
  structuralOverlay: boolean;
  structuralAlpha: number;
  /** The MEPT overlays (ADR-082): which disciplines show, and the Suggest dialog's. */
  mepOverlay: Discipline[];
  mepDiscipline: Discipline;
  setMepOverlay: (d: Discipline, on: boolean) => void;
  setMepDiscipline: (d: Discipline) => void;
  setStructuralOverlay: (on: boolean) => void;
  setStructuralAlpha: (a: number) => void;
  /** The project's drawing-set standards (ADR-047). */
  standards: Standards | null;
  setStandards: (s: Standards | null) => void;
  standardsUi: { category: string; item: number; filter: "all" | "open" };
  setStandardsUi: (patch: Partial<UiState["standardsUi"]>) => void;
  /** Detail and model lines (ADR-054): draw mode, line style, chain, polygon sides. */
  lineUi: { mode: DrawTool; style: LineStyle; chain: boolean; sides: number };
  setLineUi: (patch: Partial<UiState["lineUi"]>) => void;
  /** Edit Model with Claude (ADR-050): the dialog, its history, and the elements a
   * pending edit would change (highlighted like a selection). */
  editModel: { open: boolean; log: EditLogEntry[] };
  setEditModel: (patch: Partial<UiState["editModel"]>) => void;
  highlight: ElementId[];
  setHighlight: (ids: ElementId[]) => void;
  /** The choice pop-up for the standard picked (ADR-048). */
  choicesOpen: boolean;
  setChoicesOpen: (open: boolean) => void;
  /** Values being typed, not saved yet, by "category:index". */
  standardsPending: Record<string, string>;
  setStandardsPending: (key: string, value: string | null) => void;

  /** `fresh` = a different project was just created or opened. */
  setApp: (app: AppState | null, fresh?: boolean) => void;
  setError: (error: string | null) => void;
  openView: (id: ElementId) => void;
  closeView: (id: ElementId) => void;
  select: (ids: ElementId[]) => void;
  setTool: (tool: Tool) => void;
  /** Spacebar presses while placing a door or window: the swing turns (Revit's). */
  openingTurns: number;
  turnOpening: () => void;
  setToolType: (kind: keyof ToolTypes, id: ElementId) => void;
  setPrompt: (prompt: string) => void;
  /** Revit's status bar name for the element under the cursor, or Tab's candidate
   * (ADR-056); shown in place of the prompt while there is one. */
  hoverLabel: string;
  setHoverLabel: (hoverLabel: string) => void;
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
  autoRender: null,
  activeView: null,
  activeViewport: null,
  activateViewport: (activeViewport) => set({ activeViewport, selection: [], tool: "select" }),
  deactivateViewport: () => set({ activeViewport: null, selection: [], tool: "select" }),
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
    light: null,
    furniture: null,
    equipment: null,
    plant: null,
    group: null,
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
    dimPrefer: "WallCenterlines",
    alignPrefer: "WallFaces",
    wallLocation: "Centerline",
    stairShape: "straight",
    cameraHeight: "5' 6\"",
    plantRandomRotation: true,
    ffeRotation: 0,
    plantSizeVariation: 15,
    grassKind: "Lawn",
    grassBrush: 1200,
    grassDensity: 100,
    grassErase: false,
    textSize: 2.4,
    textLeader: "None",
    textAlign: "Left",
    componentKey: "lum-2x6",
    componentRotation: 0,
    componentFlip: false,
    refOther: false,
    refTarget: "",
    keynoteStyle: "Key",
    keynoteLeader: true,
    keynoteUserKey: "",
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
  ribbonTab: "Architecture",
  setRibbonTab: (ribbonTab) => set({ ribbonTab }),
  grayInactive: false,
  setGrayInactive: (grayInactive) => set({ grayInactive }),
  structuralOverlay: false,
  structuralAlpha: 0.85,
  mepOverlay: [],
  mepDiscipline: "Mechanical",
  setMepOverlay: (d, on) =>
    set((s) => ({
      mepOverlay: on ? [...new Set([...s.mepOverlay, d])] : s.mepOverlay.filter((x) => x !== d),
    })),
  setMepDiscipline: (mepDiscipline) => set({ mepDiscipline }),
  setStructuralOverlay: (structuralOverlay) => set({ structuralOverlay }),
  setStructuralAlpha: (structuralAlpha) =>
    set({ structuralAlpha: Math.min(1, Math.max(0.2, structuralAlpha)) }),
  standards: null,
  setStandards: (standards) => set({ standards }),
  standardsUi: { category: "sheet", item: 0, filter: "all" },
  setStandardsUi: (patch) => set((s) => ({ standardsUi: { ...s.standardsUi, ...patch } })),
  standardsPending: {},
  lineUi: { mode: "Line", style: "Thin", chain: true, sides: 6 },
  setLineUi: (patch) => set((s) => ({ lineUi: { ...s.lineUi, ...patch } })),
  editModel: { open: false, log: [] },
  setEditModel: (patch) => set((s) => ({ editModel: { ...s.editModel, ...patch } })),
  highlight: [],
  setHighlight: (highlight) => set({ highlight }),
  choicesOpen: false,
  setChoicesOpen: (choicesOpen) => set({ choicesOpen }),
  setStandardsPending: (key, value) =>
    set((s) => {
      const next = { ...s.standardsPending };
      if (value === null) delete next[key];
      else next[key] = value;
      return { standardsPending: next };
    }),
  lastTool: null,
  snapOverride: null,
  setSnapOverride: (snapOverride) => set({ snapOverride }),
  tempHide: {},
  setTempHide: (view, th) =>
    set((s) => {
      const next = { ...s.tempHide };
      if (th) next[view] = th;
      else delete next[view];
      return { tempHide: next };
    }),
  thinLines: false,
  visualStyles: {},
  setVisualStyle: (view, style) =>
    set((s) => ({ visualStyles: { ...s.visualStyles, [view]: style } })),
  propsHidden: false,
  viewDialog: null,
  setUi: (patch) => set(patch),
  picker: null,
  setPicker: (picker) => set({ picker }),
  paintMaterial: null,
  ifcReport: null,
  setIfcReport: (ifcReport) => set({ ifcReport }),
  setPaintMaterial: (paintMaterial) => set({ paintMaterial }),
  siteDialog: null,
  setSiteDialog: (siteDialog) => set({ siteDialog }),
  grid3d: true,
  nav3d: "orbit",
  setNav3d: (nav3d) => set({ nav3d }),
  sunPreview: null,
  setSunPreview: (sunPreview) => set({ sunPreview }),
  exposure3d: 1,
  setExposure3d: (exposure3d) => set({ exposure3d }),
  setGrid3d: (grid3d) => set({ grid3d }),
  terrain: { solid: true, contours: true, labels: true },
  setTerrain: (patch) => set((s) => ({ terrain: { ...s.terrain, ...patch } })),
  satellite: false,
  setSatellite: (satellite) => set({ satellite }),
  level3d: null,
  setLevel3d: (level3d) => set({ level3d }),

  setApp: (app, fresh = false) => {
    const s = get();
    if (!app) {
      set({ app: null, openViews: [], activeView: null, activeViewport: null, selection: [] });
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
    const vp = s.activeViewport;
    set({
      app,
      error: null,
      openViews,
      activeView,
      activeViewport:
        vp && vp.sheet === activeView && exists.has(vp.view) && exists.has(vp.sheet) ? vp : null,
      tool,
      sketchUi: { ...s.sketchUi, sel: s.sketchUi.sel.filter((i) => i < curves) },
      selection: sameProject && !app.sketch ? s.selection : [],
      // A different project opens on Architecture with its own standards.
      ...(sameProject ? {} : { ribbonTab: "Architecture", standards: null, standardsPending: {} }),
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
        // A 3'-0" x 5'-0" double-hung is the everyday default (ADR-031).
        window: firstId(
          [...app.windowTypes].sort(
            (a, b) =>
              Number(!a.name.startsWith('Double Hung 36" x 60"')) -
              Number(!b.name.startsWith('Double Hung 36" x 60"')),
          ),
          s.toolTypes.window,
        ),
        roof: firstId(app.roofTypes, s.toolTypes.roof),
        light: firstId(app.lightingFixtureTypes, s.toolTypes.light),
        furniture: firstId(app.furnitureTypes, s.toolTypes.furniture),
        equipment: firstId(app.equipmentTypes, s.toolTypes.equipment),
        plant: firstId(app.plantingTypes, s.toolTypes.plant),
        group: app.groupTypes.some((t) => t.id === s.toolTypes.group) ? s.toolTypes.group : null,
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
      // Another view deactivates the sheet's activated viewport.
      activeViewport: s.activeViewport?.sheet === id ? s.activeViewport : null,
      selection: [],
      tool: "select",
    })),
  closeView: (id) =>
    set((s) => {
      const openViews = s.openViews.filter((v) => v !== id);
      const activeView =
        s.activeView === id ? (openViews[openViews.length - 1] ?? null) : s.activeView;
      const activeViewport = s.activeViewport?.sheet === activeView ? s.activeViewport : null;
      return { openViews, activeView, activeViewport };
    }),
  // Clicking a group's member selects the group, as in Revit (ADR-087).
  select: (selection) => set({ selection: toGroups(get().app, selection) }),
  // Move, Copy, Rotate, Mirror and Array act on the current selection; other tools start
  // with nothing selected.
  openingTurns: 0,
  turnOpening: () => set((s) => ({ openingTurns: (s.openingTurns + 1) % 4 })),
  setTool: (tool) =>
    set({
      tool,
      openingTurns: tool === get().tool ? get().openingTurns : 0,
      lastTool: tool === "select" || tool === "sketch" ? get().lastTool : tool,
      selection: tool === "select" || SELECTION_TOOLS.includes(tool) ? get().selection : [],
      // A tool (from a shortcut) needs the views, which the Standards tab hides.
      ...(tool !== "select" &&
      (get().ribbonTab === "Standards" ||
        get().ribbonTab === "Project Info" ||
        get().ribbonTab === "Specifications")
        ? { ribbonTab: "Architecture" }
        : {}),
    }),
  assetFilter: null,
  setAssetFilter: (assetFilter) => set({ assetFilter }),
  setToolType: (kind, id) => set((s) => ({ toolTypes: { ...s.toolTypes, [kind]: id } })),
  setPrompt: (prompt) => set({ prompt }),
  hoverLabel: "",
  setHoverLabel: (hoverLabel) => set({ hoverLabel }),
  setCursor: (cursor) => set({ cursor }),
  setConfirm: (confirm) => set({ confirm }),
  setRufplan: (rufplan) => set({ rufplan }),
  setCloud: (cloud) => set({ cloud }),
  setOption: (key, value) => set((s) => ({ options: { ...s.options, [key]: value } })),
  setParamsOpen: (paramsOpen) => set({ paramsOpen }),
}));

/** Info about the active view, if any. */
/** The view being worked in: the active tab's, or the view activated on its sheet. */
export function activeViewInfo(s: Pick<UiState, "app" | "activeView" | "activeViewport">) {
  const a = s.activeViewport;
  const id = a && a.sheet === s.activeView ? a.view : s.activeView;
  return s.app?.views.find((v) => v.id === id) ?? null;
}

export interface ActiveViewport {
  sheet: ElementId;
  viewport: ElementId;
  view: ElementId;
  /** Center on the sheet, paper mm. */
  center: Pt;
}

/** A 3D view's visual style (ADR-038). */
export function styleOf(
  s: Pick<UiState, "visualStyles">,
  view: ElementId | null | undefined,
): VisualStyle {
  return (view && s.visualStyles[view]) || "shaded";
}

/** The dimension tools (ADR-040). */
export const DIMENSION_TOOLS: Tool[] = ["dimension", "dimensionLinear", "dimensionAngular"];

/** Tools that pick references (faces, centerlines, grids) with Tab to cycle (ADR-040/042). */
export const REFERENCE_TOOLS: Tool[] = [...DIMENSION_TOOLS, "align"];
