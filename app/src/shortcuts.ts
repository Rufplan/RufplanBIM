// Revit's keyboard shortcuts (ADR-024): two-letter keys for tools and commands, with the
// owner's overrides from Keyboard Shortcuts (KS) kept in this computer's local storage.
import type { Tool } from "./store";

export type Action =
  | "paint"
  | "repeat"
  | "createSimilar"
  | "selectAll"
  | "pin"
  | "unpin"
  | "delete"
  | "floor"
  | "hideElement"
  | "isolateElement"
  | "hideCategory"
  | "isolateCategory"
  | "resetTemporary"
  | "hideInView"
  | "hideCategoryInView"
  | "visibility"
  | "snapEndpoint"
  | "snapMidpoint"
  | "snapIntersection"
  | "snapPerpendicular"
  | "snapNearest"
  | "snapOff"
  | "snapOverrideOff"
  | "zoomFit"
  | "zoomOut"
  | "zoomRegion"
  | "zoomPrevious"
  | "thinLines"
  | "wireframe"
  | "hiddenLine"
  | "shaded"
  | "consistent"
  | "realistic"
  | "properties"
  | "viewProperties"
  | "keyboard"
  | "render";

export type Command = { tool: Tool } | { action: Action };

export interface ShortcutDef {
  /** Stable id for overrides. */
  id: string;
  label: string;
  group: string;
  keys: string[];
  command: Command;
}

const t = (tool: Tool): Command => ({ tool });
const a = (action: Action): Command => ({ action });

/** Revit's defaults (and ours for tools Revit has no default for, marked in the label). */
export const DEFAULT_SHORTCUTS: ShortcutDef[] = [
  // Modify
  { id: "modify", label: "Modify (Select)", group: "Modify", keys: ["MD"], command: t("select") },
  { id: "move", label: "Move", group: "Modify", keys: ["MV"], command: t("move") },
  { id: "copy", label: "Copy", group: "Modify", keys: ["CO", "CC"], command: t("copy") },
  { id: "rotate", label: "Rotate", group: "Modify", keys: ["RO"], command: t("rotate") },
  {
    id: "mirrorPick",
    label: "Mirror - Pick Axis",
    group: "Modify",
    keys: ["MM"],
    command: t("mirrorPick"),
  },
  {
    id: "mirrorDraw",
    label: "Mirror - Draw Axis",
    group: "Modify",
    keys: ["DM"],
    command: t("mirror"),
  },
  { id: "array", label: "Array", group: "Modify", keys: ["AR"], command: t("array") },
  { id: "align", label: "Align", group: "Modify", keys: ["AL"], command: t("align") },
  { id: "trim", label: "Trim/Extend to Corner", group: "Modify", keys: ["TR"], command: t("trim") },
  { id: "offset", label: "Offset", group: "Modify", keys: ["OF"], command: t("offset") },
  { id: "split", label: "Split Element", group: "Modify", keys: ["SL"], command: t("split") },
  { id: "delete", label: "Delete", group: "Modify", keys: ["DE"], command: a("delete") },
  { id: "pin", label: "Pin", group: "Modify", keys: ["PN"], command: a("pin") },
  { id: "unpin", label: "Unpin", group: "Modify", keys: ["UP"], command: a("unpin") },
  {
    id: "matchType",
    label: "Match Type Properties",
    group: "Modify",
    keys: ["MA"],
    command: t("matchType"),
  },
  {
    id: "createSimilar",
    label: "Create Similar",
    group: "Modify",
    keys: ["CS"],
    command: a("createSimilar"),
  },
  {
    id: "selectAll",
    label: "Select All Instances",
    group: "Modify",
    keys: ["SA"],
    command: a("selectAll"),
  },
  {
    id: "repeat",
    label: "Repeat Last Command",
    group: "Modify",
    keys: ["RC"],
    command: a("repeat"),
  },
  // Architecture and structure
  { id: "wall", label: "Wall", group: "Architecture", keys: ["WA"], command: t("wall") },
  { id: "door", label: "Door", group: "Architecture", keys: ["DR"], command: t("door") },
  { id: "window", label: "Window", group: "Architecture", keys: ["WN"], command: t("window") },
  { id: "paint", label: "Paint", group: "Modify", keys: ["PT"], command: a("paint") },
  { id: "floor", label: "Floor", group: "Architecture", keys: ["SB"], command: a("floor") },
  { id: "roof", label: "Roof (ours)", group: "Architecture", keys: ["RF"], command: t("roof") },
  { id: "stair", label: "Stair (ours)", group: "Architecture", keys: ["ST"], command: t("stair") },
  {
    id: "railing",
    label: "Railing (ours)",
    group: "Architecture",
    keys: ["RA"],
    command: t("railing"),
  },
  { id: "room", label: "Room", group: "Architecture", keys: ["RM"], command: t("room") },
  {
    id: "roomSeparator",
    label: "Room Separator (ours)",
    group: "Architecture",
    keys: ["RS"],
    command: t("roomSeparator"),
  },
  {
    id: "column",
    label: "Structural Column",
    group: "Structure",
    keys: ["CL"],
    command: t("column"),
  },
  { id: "beam", label: "Beam", group: "Structure", keys: ["BM"], command: t("beam") },
  { id: "grid", label: "Grid", group: "Datum", keys: ["GR"], command: t("grid") },
  { id: "level", label: "Level", group: "Datum", keys: ["LL"], command: t("level") },
  // Annotate
  {
    id: "dimension",
    label: "Aligned Dimension",
    group: "Annotate",
    keys: ["DI"],
    command: t("dimension"),
  },
  { id: "text", label: "Text", group: "Annotate", keys: ["TX"], command: t("text") },
  { id: "tag", label: "Tag by Category", group: "Annotate", keys: ["TG"], command: t("tag") },
  { id: "roomTag", label: "Room Tag", group: "Annotate", keys: ["RT"], command: t("tag") },
  // View
  {
    id: "elevation",
    label: "Elevation (ours)",
    group: "View",
    keys: ["EL"],
    command: t("elevation"),
  },
  { id: "callout", label: "Callout (ours)", group: "View", keys: ["CA"], command: t("callout") },
  {
    id: "visibility",
    label: "Visibility/Graphics",
    group: "View",
    keys: ["VV", "VG"],
    command: a("visibility"),
  },
  {
    id: "hideElement",
    label: "Temporary Hide Element",
    group: "View",
    keys: ["HH"],
    command: a("hideElement"),
  },
  {
    id: "isolateElement",
    label: "Temporary Isolate Element",
    group: "View",
    keys: ["HI"],
    command: a("isolateElement"),
  },
  {
    id: "hideCategory",
    label: "Temporary Hide Category",
    group: "View",
    keys: ["HC"],
    command: a("hideCategory"),
  },
  {
    id: "isolateCategory",
    label: "Temporary Isolate Category",
    group: "View",
    keys: ["IC"],
    command: a("isolateCategory"),
  },
  {
    id: "resetTemporary",
    label: "Reset Temporary Hide/Isolate",
    group: "View",
    keys: ["HR"],
    command: a("resetTemporary"),
  },
  {
    id: "hideInView",
    label: "Hide in View: Elements",
    group: "View",
    keys: ["EH"],
    command: a("hideInView"),
  },
  {
    id: "hideCategoryInView",
    label: "Hide in View: Category",
    group: "View",
    keys: ["VH"],
    command: a("hideCategoryInView"),
  },
  { id: "thinLines", label: "Thin Lines", group: "View", keys: ["TL"], command: a("thinLines") },
  {
    id: "wireframe",
    label: "Wireframe (3D)",
    group: "View",
    keys: ["WF"],
    command: a("wireframe"),
  },
  {
    id: "hiddenLine",
    label: "Hidden Line (3D)",
    group: "View",
    keys: ["HL"],
    command: a("hiddenLine"),
  },
  { id: "shaded", label: "Shaded (3D)", group: "View", keys: ["SD"], command: a("shaded") },
  {
    id: "consistent",
    label: "Consistent Colors (3D)",
    group: "View",
    keys: [],
    command: a("consistent"),
  },
  { id: "realistic", label: "Realistic (3D)", group: "View", keys: [], command: a("realistic") },
  {
    id: "zoomFit",
    label: "Zoom to Fit",
    group: "View",
    keys: ["ZF", "ZE", "ZX", "ZA"],
    command: a("zoomFit"),
  },
  { id: "zoomOut", label: "Zoom Out (2x)", group: "View", keys: ["ZO"], command: a("zoomOut") },
  {
    id: "zoomRegion",
    label: "Zoom in Region",
    group: "View",
    keys: ["ZR"],
    command: a("zoomRegion"),
  },
  {
    id: "zoomPrevious",
    label: "Previous Pan/Zoom",
    group: "View",
    keys: ["ZP"],
    command: a("zoomPrevious"),
  },
  { id: "properties", label: "Properties", group: "View", keys: ["PP"], command: a("properties") },
  {
    id: "viewProperties",
    label: "View Properties",
    group: "View",
    keys: ["VP"],
    command: a("viewProperties"),
  },
  {
    id: "keyboard",
    label: "Keyboard Shortcuts",
    group: "View",
    keys: ["KS"],
    command: a("keyboard"),
  },
  // Rendering (ADR-027)
  { id: "camera", label: "Camera", group: "Rendering", keys: [], command: t("camera") },
  { id: "render", label: "Render", group: "Rendering", keys: ["RR"], command: a("render") },
  // Snaps (for the next pick)
  {
    id: "snapEndpoint",
    label: "Snap to Endpoints",
    group: "Snaps",
    keys: ["SE"],
    command: a("snapEndpoint"),
  },
  {
    id: "snapMidpoint",
    label: "Snap to Midpoints",
    group: "Snaps",
    keys: ["SM"],
    command: a("snapMidpoint"),
  },
  {
    id: "snapIntersection",
    label: "Snap to Intersections",
    group: "Snaps",
    keys: ["SI"],
    command: a("snapIntersection"),
  },
  {
    id: "snapPerpendicular",
    label: "Snap to Perpendicular",
    group: "Snaps",
    keys: ["SP"],
    command: a("snapPerpendicular"),
  },
  {
    id: "snapNearest",
    label: "Snap to Nearest",
    group: "Snaps",
    keys: ["SN"],
    command: a("snapNearest"),
  },
  { id: "snapOff", label: "Snaps Off", group: "Snaps", keys: ["SO"], command: a("snapOff") },
  {
    id: "snapOverrideOff",
    label: "Turn Override Off",
    group: "Snaps",
    keys: ["SS"],
    command: a("snapOverrideOff"),
  },
];

const STORE_KEY = "rufplan.shortcuts";

/** The owner's key overrides by command id (empty list: no shortcut). */
export function loadOverrides(): Record<string, string[]> {
  try {
    const raw = localStorage.getItem(STORE_KEY);
    return raw ? (JSON.parse(raw) as Record<string, string[]>) : {};
  } catch {
    return {};
  }
}

export function saveOverrides(o: Record<string, string[]>) {
  try {
    localStorage.setItem(STORE_KEY, JSON.stringify(o));
  } catch {
    // Local storage unavailable: shortcuts stay as they are for this session.
  }
}

/** The shortcut table in effect. */
export function activeShortcuts(overrides = loadOverrides()): ShortcutDef[] {
  return DEFAULT_SHORTCUTS.map((d) => (overrides[d.id] ? { ...d, keys: overrides[d.id]! } : d));
}

/** Keys → command, first definition winning. */
export function keyMap(defs = activeShortcuts()): Map<string, Command> {
  const m = new Map<string, Command>();
  for (const d of defs)
    for (const k of d.keys) if (!m.has(k.toUpperCase())) m.set(k.toUpperCase(), d.command);
  return m;
}

/** Keys used by more than one command. */
export function conflicts(defs: ShortcutDef[]): string[] {
  const seen = new Map<string, number>();
  for (const d of defs)
    for (const k of d.keys) seen.set(k.toUpperCase(), (seen.get(k.toUpperCase()) ?? 0) + 1);
  return [...seen].filter(([, n]) => n > 1).map(([k]) => k);
}
