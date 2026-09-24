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
  DI: "dimension",
  TX: "text",
  CO: "copy",
  RO: "rotate",
  MM: "mirror",
  AR: "array",
  AL: "align",
  TR: "trim",
  OF: "offset",
  SL: "split",
  RF: "roof",
  ST: "stair",
};

/** Tools that need a plan view (they place elements on the view's level). */
export const PLAN_TOOLS: Tool[] = [
  "roof",
  "stair",
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
  if (tool === "level") return view === "Elevation" || view === "Section";
  if (tool === "room" || tool === "section" || tool === "stair") return view === "Plan";
  if (tool === "roof") return view === "Plan";
  // Move works in plan coordinates, and on sheets for viewports.
  if (tool === "move") return view === "Plan" || view === "CeilingPlan" || view === "Sheet";
  if (tool === "text" && view === "Sheet") return true;
  if (tool === "dimension" || tool === "text") {
    return view === "Plan" || view === "CeilingPlan" || view === "Elevation" || view === "Section";
  }
  if (tool === "grid") return view === "Plan" || view === "CeilingPlan";
  return view === "Plan" || view === "CeilingPlan";
}

/** Status-bar prompt for a tool with `n` points placed so far. */
export function promptFor(tool: Tool, n: number, view: ViewType | undefined): string {
  if (!toolAllowed(tool, view)) {
    if (tool === "level") return "Open an elevation to place levels.";
    if (tool === "room") return "Open a floor plan to place rooms.";
    if (tool === "section") return "Open a floor plan to draw a section line.";
    if (tool === "dimension" || tool === "text")
      return "Open a plan, elevation or section to annotate.";
    return "Open a floor or ceiling plan to use this tool.";
  }
  switch (tool) {
    case "copy":
      return n === 0
        ? "Click a base point to copy the selection from (select elements first)."
        : "Click where the copy goes, or type a distance and press Enter.";
    case "array":
      return n === 0
        ? "Click the first point of the array spacing (select elements first)."
        : "Click the second point: each copy is placed this far from the last. Set the number in the options bar.";
    case "rotate":
      return n === 0
        ? "Click the center of rotation (select elements first)."
        : n === 1
          ? "Click a point on the line to rotate from."
          : "Click the point to rotate to, or type an angle in degrees and press Enter.";
    case "mirror":
      return n === 0
        ? "Draw the mirror axis: click its first point (select elements first)."
        : "Click the second point of the mirror axis.";
    case "align":
      return n === 0
        ? "Click the reference line to align to (a wall face, centerline or grid)."
        : "Click the line on the element that should move to it.";
    case "trim":
      return n === 0
        ? "Click the part of the first wall to keep."
        : "Click the part of the second wall to keep: both meet at the corner.";
    case "offset":
      return "Hover beside a wall or grid and click to place an offset copy at the options-bar distance.";
    case "split":
      return "Click a wall where it should be split in two.";
    case "roof":
      return "Click anywhere to put a hip roof over this level's walls (18\" overhang, 6/12). Edit its edges and slope in Properties.";
    case "stair":
      return n === 0
        ? "Click the center of the first riser."
        : "Click toward where the stair climbs to the level above.";
    case "select":
      return "Click to select. Double-click an elevation marker or level to open its view. Drag with the middle or right mouse button to pan; scroll to zoom.";
    case "wall":
      return n === 0
        ? "Click the wall start point."
        : "Click the next point, or type a length and press Enter. Right-click or Esc to finish the chain.";
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
    case "dimension":
      return n === 0
        ? "Click the first point to dimension from."
        : n === 1
          ? "Click the second point."
          : "Move to place the dimension line, then click.";
    case "text":
      return "Click where the text note goes.";
    case "section":
      return n === 0
        ? "Click the section line's start. The section looks to the left of the line."
        : "Click the end of the section line.";
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

/** Keys that start typing a length (or angle) while drawing: digits and ft-in marks. */
export function startsTypedValue(key: string): boolean {
  return /^[0-9.'"-]$/.test(key);
}

/** The point `length` mm from `from` toward `toward` (the snapped cursor). */
export function pointAtLength(from: Pt, toward: Pt, length: number): Pt | null {
  const dx = toward.x - from.x;
  const dy = toward.y - from.y;
  const d = Math.hypot(dx, dy);
  if (d < 1e-6 || !(length > 0)) return null;
  return { x: from.x + (dx / d) * length, y: from.y + (dy / d) * length };
}

/** Counter-clockwise angle (radians) from ray center→a to ray center→b, in (-π, π]. */
export function sweep(center: Pt, a: Pt, b: Pt): number {
  const a0 = Math.atan2(a.y - center.y, a.x - center.x);
  const a1 = Math.atan2(b.y - center.y, b.x - center.x);
  let d = a1 - a0;
  while (d <= -Math.PI) d += 2 * Math.PI;
  while (d > Math.PI) d -= 2 * Math.PI;
  return d;
}
