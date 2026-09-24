// Tool helpers that don't touch React or IPC, so they're easy to test.
import type { Pt } from "./bindings/Pt";
import type { ViewType } from "./bindings/ViewType";
import type { Tool } from "./store";

/** Revit-style two-letter keyboard shortcuts. */
export const SHORTCUTS: Record<string, Tool> = {
  MD: "select",
  WA: "wall",
  GR: "grid",
  LL: "level",
  SB: "floor",
  FP: "floorAuto",
  CL: "ceilingAuto",
  CS: "ceiling",
  DR: "door",
  WN: "window",
  RM: "room",
  MV: "move",
};

/** Tools that need a plan view (they place elements on the view's level). */
export const PLAN_TOOLS: Tool[] = [
  "wall",
  "door",
  "window",
  "floor",
  "floorAuto",
  "ceiling",
  "ceilingAuto",
];

export function toolAllowed(tool: Tool, view: ViewType | undefined): boolean {
  if (tool === "select") return true;
  if (tool === "level") return view === "Elevation";
  if (tool === "room") return view === "Plan";
  if (tool === "grid") return view === "Plan" || view === "CeilingPlan";
  return view === "Plan" || view === "CeilingPlan";
}

/** Status-bar prompt for a tool with `n` points placed so far. */
export function promptFor(tool: Tool, n: number, view: ViewType | undefined): string {
  if (!toolAllowed(tool, view)) {
    if (tool === "level") return "Open an elevation to place levels.";
    if (tool === "room") return "Open a floor plan to place rooms.";
    return "Open a floor or ceiling plan to use this tool.";
  }
  switch (tool) {
    case "select":
      return "Click to select. Double-click an elevation marker or level to open its view. Drag with the middle or right mouse button to pan; scroll to zoom.";
    case "wall":
      return n === 0
        ? "Click the wall start point."
        : "Click the next point. Right-click or Esc to finish the chain.";
    case "grid":
      return n === 0 ? "Click the grid start point." : "Click the grid end point.";
    case "floor":
    case "ceiling":
      return n < 3
        ? "Click boundary corners."
        : "Click the first corner or press Enter to close the boundary. Esc cancels.";
    case "floorAuto":
      return "Click anywhere to create a floor at the outer faces of this level's walls.";
    case "ceilingAuto":
      return "Click inside a room enclosed by walls to place a ceiling.";
    case "level":
      return "Click at the height for the new level.";
    case "door":
    case "window":
      return `Hover over a wall and click to place the ${tool}. The side of the wall you point at sets which way it faces.`;
    case "room":
      return "Hover inside an area enclosed by walls and click to place a room.";
    case "move":
      return n === 0
        ? "Click a base point to move the selection from (select elements first)."
        : "Click the destination. Joined walls stretch to follow.";
  }
}

/** True when a click at `p` (screen px) should close a sketch that starts at `first`. */
export function closesSketch(
  firstScreen: [number, number],
  pScreen: [number, number],
  n: number,
): boolean {
  if (n < 3) return false;
  return Math.hypot(firstScreen[0] - pScreen[0], firstScreen[1] - pScreen[1]) < 10;
}

/** Feeds a key into the shortcut buffer; returns the tool when a shortcut completes. */
export function shortcut(buffer: string, key: string): { buffer: string; tool: Tool | null } {
  if (!/^[a-z]$/i.test(key)) return { buffer: "", tool: null };
  const next = (buffer + key.toUpperCase()).slice(-2);
  const tool = SHORTCUTS[next] ?? null;
  return { buffer: tool ? "" : next, tool };
}

export function samePt(a: Pt, b: Pt): boolean {
  return Math.abs(a.x - b.x) < 0.5 && Math.abs(a.y - b.y) < 0.5;
}
