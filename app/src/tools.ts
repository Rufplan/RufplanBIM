// Tool helpers that don't touch React or IPC, so they're easy to test.
import type { Pt } from "./bindings/Pt";
import type { ViewType } from "./bindings/ViewType";
import { DEFAULT_SHORTCUTS, keyMap, type Action } from "./shortcuts";
import { useAppStore, type SketchMode, type Tool } from "./store";

/** Status-bar prompt in sketch mode, as Revit words it. */
export function sketchPrompt(mode: SketchMode, n: number): string {
  switch (mode) {
    case "Modify":
      return "Click boundary lines to select; drag their ends; Delete removes; Space flips a picked wall line. Finish ✓ or Cancel ✗ on the ribbon.";
    case "Line":
      return n === 0
        ? "Click to enter line start point."
        : "Click to enter line end point, or type a length and press Enter.";
    case "Rectangle":
      return n === 0 ? "Click to enter rectangle start point." : "Click to enter other corner.";
    case "InscribedPolygon":
    case "CircumscribedPolygon":
      return n === 0 ? "Click to enter polygon center." : "Click to enter polygon radius.";
    case "Circle":
      return n === 0 ? "Click to enter circle center." : "Click to enter circle radius.";
    case "StartEndRadiusArc":
      return n === 0
        ? "Click to enter arc start point."
        : n === 1
          ? "Click to enter arc end point."
          : "Click to enter a point on the arc (sets its radius).";
    case "CenterEndsArc":
      return n === 0
        ? "Click to enter arc center point."
        : n === 1
          ? "Click to enter arc start point (sets the radius)."
          : "Click to enter arc end point.";
    case "FilletArc":
      return n === 0 ? "Select first line to fillet." : "Select second line to fillet.";
    case "PickLines":
      return "Select a line, wall face or grid to add a boundary line.";
    case "PickWalls":
      return "Select walls to add boundary lines. Press Tab to pick a chain of walls; the side you point at is the face used.";
    case "Trim":
      return n === 0
        ? "Click the first line to trim/extend (the part to keep)."
        : "Click the second line (the part to keep).";
  }
}

/** Two-letter keys → tools (Revit's defaults; see shortcuts.ts for commands too). */
export const SHORTCUTS: Record<string, Tool> = Object.fromEntries(
  DEFAULT_SHORTCUTS.flatMap((d) =>
    "tool" in d.command ? d.keys.map((k) => [k, (d.command as { tool: Tool }).tool]) : [],
  ),
);

/** Tools that need a plan view (they place elements on the view's level). */
export const PLAN_TOOLS: Tool[] = [
  "plant",
  "roof",
  "stair",
  "column",
  "light",
  "beam",
  "railing",
  "roomSeparator",
  "wall",
  "door",
  "window",
  "floor",
  "floorAuto",
  "ceiling",
  "ceilingAuto",
];

/** Tools that also work in the 3D view (ADR-022). */
export const TOOLS_3D: Tool[] = [
  "wall",
  "door",
  "window",
  "floorAuto",
  "ceilingAuto",
  "roof",
  "column",
  "light",
  "plant",
  "grassBrush",
  "wallOpening",
  "room",
  // Boundary sketches on the level's work plane, and moving or copying the selection (ADR-025).
  "sketch",
  "move",
  "copy",
  "paint",
];

/** Drawing tools whose clicks place points (ViewCanvas's placePoint). */
export const POINT_TOOLS: Tool[] = [
  "wall",
  "grid",
  "stair",
  "beam",
  "railing",
  "roomSeparator",
  "callout",
  "camera",
];

export function toolAllowed(tool: Tool, view: ViewType | undefined): boolean {
  if (tool === "select") return true;
  if (tool === "paint") return view !== "Sheet" && view !== "Schedule";
  // Drafting views hold only 2D detailing (ADR-069).
  if (view === "Drafting")
    return [
      "detailLine",
      "text",
      "sketch",
      "move",
      "copy",
      "rotate",
      "mirror",
      "array",
      "offset",
    ].includes(tool);
  // A filled region is sketched in any 2D view.
  if (tool === "sketch" && useAppStore.getState().app?.sketch?.region)
    return view !== "ThreeD" && view !== "Schedule";
  if (view === "ThreeD") return TOOLS_3D.includes(tool);
  if (tool === "level") return view === "Elevation" || view === "Section";
  if (tool === "room" || tool === "section" || tool === "stair" || tool === "camera")
    return view === "Plan";
  if (
    tool === "roof" ||
    tool === "column" ||
    tool === "beam" ||
    tool === "railing" ||
    tool === "roomSeparator"
  )
    return view === "Plan";
  // Wall openings are sketched on a wall's face, in an elevation or section (ADR-058).
  if (tool === "wallOpening") return view === "Elevation" || view === "Section";
  if (tool === "sketch" && useAppStore.getState().app?.sketch?.wall)
    return view === "Elevation" || view === "Section";
  // Doors, windows and lighting fixtures also go in from elevations and sections, on the
  // wall face clicked (ADR-059).
  if (tool === "door" || tool === "window" || tool === "light")
    return view === "Plan" || view === "CeilingPlan" || view === "Elevation" || view === "Section";
  if (tool === "sketch" || tool === "elevation") return view === "Plan" || view === "CeilingPlan";
  // D5's Grass Brush paints in 3D (ADR-065).
  if (tool === "grassBrush") return false;
  // Plants go in plans (site plans too) and 3D (ADR-064).
  if (tool === "plant") return view === "Plan";
  // Rooms are tagged in sections and elevations too (ADR-060).
  if (tool === "tag") return view === "Plan" || view === "Elevation" || view === "Section";
  if (tool === "matchType" || tool === "mirrorPick")
    return view === "Plan" || view === "CeilingPlan";
  if (tool === "callout")
    return view === "Plan" || view === "CeilingPlan" || view === "Elevation" || view === "Section";
  // Move works in plan coordinates, and on sheets for viewports.
  if (tool === "move") return view === "Plan" || view === "CeilingPlan" || view === "Sheet";
  if (tool === "text" && view === "Sheet") return true;
  // Symbols (ADR-048): key plans on sheets, north arrows in plans or on sheets.
  if (tool === "keyPlan") return view === "Sheet";
  // Lines (ADR-054): detail lines in any 2D view or sheet, model lines in plans.
  if (tool === "detailLine")
    return (
      view === "Plan" ||
      view === "CeilingPlan" ||
      view === "Elevation" ||
      view === "Section" ||
      view === "Sheet"
    );
  if (tool === "modelLine") return view === "Plan" || view === "CeilingPlan";
  if (tool === "northArrow") return view === "Plan" || view === "CeilingPlan" || view === "Sheet";
  if (tool === "spotElevation" || tool === "graphicScale")
    return view === "Plan" || view === "CeilingPlan" || view === "Elevation" || view === "Section";
  if (tool === "spotSlope") return view === "Plan" || view === "Elevation" || view === "Section";
  if (tool === "dimension" || tool === "dimensionLinear" || tool === "text") {
    return view === "Plan" || view === "CeilingPlan" || view === "Elevation" || view === "Section";
  }
  // Angular dimensions measure between walls or grids (ADR-040).
  if (tool === "dimensionAngular")
    return view === "Plan" || view === "CeilingPlan" || view === "Elevation" || view === "Section";
  if (tool === "grid") return view === "Plan" || view === "CeilingPlan";
  return view === "Plan" || view === "CeilingPlan";
}

/** Status-bar prompt for a tool with `n` points placed so far. */
export function promptFor(tool: Tool, n: number, view: ViewType | undefined): string {
  if (!toolAllowed(tool, view)) {
    if (tool === "level") return "Open an elevation to place levels.";
    if (tool === "room") return "Open a floor plan to place rooms.";
    if (tool === "section") return "Open a floor plan to draw a section line.";
    if (tool === "callout") return "Open a plan, elevation or section to draw a callout.";
    if (tool === "dimension" || tool === "dimensionLinear" || tool === "text")
      return "Open a plan, elevation or section to annotate.";
    if (tool === "dimensionAngular")
      return "Open a plan, elevation or section to dimension an angle.";
    if (tool === "keyPlan") return "Open a sheet to place a key plan.";
    if (tool === "detailLine")
      return "Open a plan, elevation, section or sheet to draw detail lines.";
    if (tool === "modelLine")
      return "Open a floor or ceiling plan to draw model lines on its level.";
    if (tool === "northArrow") return "Open a plan or a sheet to place a north arrow.";
    if (tool === "spotElevation" || tool === "graphicScale" || tool === "spotSlope")
      return "Open a plan, elevation or section to place it.";
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
        ? "Pick the reference to align to: a wall face, centerline or grid (Tab for the next)."
        : "Pick the line on the element that should move onto it (Tab for the next).";
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
        ? "Click the center of the first riser. Pick the shape (straight, L or U) in the options bar."
        : "Click toward where the stair climbs to the level above.";
    case "column":
      return "Click to place a column; it snaps to grid intersections. Columns rise to the level above.";
    case "wallOpening":
      return "Click the wall to cut the opening in. You'll sketch its shape on the wall's face: rectangles, circles, polygons, arcs or lines.";
    case "grassBrush":
      return "Drag over the ground to paint grass (Erase on the options bar takes it away). Esc finishes.";
    case "plant":
      return "Click to place the plant; keep clicking to place more (Esc to stop). Random rotation and size are on the options bar.";
    case "light":
      if (view === "Elevation" || view === "Section")
        return "Click a wall's face to place the fixture there (wall fixtures at the height clicked); in a section, click anywhere on the cut.";
      return "Click to place the lighting fixture: ceiling fixtures go at the ceiling, wall fixtures on the nearest wall's face, lamps and site lights on the floor or ground.";
    case "beam":
      return n === 0
        ? "Click the beam's start. It frames the floor above this plan."
        : "Click the beam's end, or type a length and press Enter.";
    case "sketch":
      return sketchPrompt(useAppStore.getState().sketchUi.mode, n);
    case "tag":
      return view === "Elevation" || view === "Section"
        ? "Click inside a room, at its level's height, to tag it there. Select a tag and drag it to move it."
        : "Click a door, window, room, column or beam to tag it. Select a tag and drag it to move it.";
    case "camera":
      return n === 0
        ? "Click in the plan to place the camera's eye point."
        : "Click the target point: where the camera looks.";
    case "matchType":
      return n === 0
        ? "Click the element whose type to copy."
        : "Click elements to give them that type. Esc finishes.";
    case "mirrorPick":
      return "Pick a line (wall face, centerline or grid) to mirror the selection across.";
    case "elevation":
      return "Click to place an elevation marker; it looks at the nearest wall. Check more views on the marker in Properties.";
    case "roomSeparator":
      return n === 0
        ? "Click the start of a room separation line (for open plans)."
        : "Click the next point, or type a length and press Enter. Esc finishes.";
    case "callout":
      return n === 0
        ? 'Click one corner of the area to call out at 1 1/2" = 1\'-0".'
        : "Click the opposite corner.";
    case "railing":
      return n < 2
        ? "Click the railing path's points."
        : "Click the next point, or press Enter to finish the railing. Esc cancels.";
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
      return view === "Elevation" || view === "Section"
        ? `Hover over a wall's face and click to place the ${tool}; it faces you, at its type's sill.`
        : `Hover over a wall and click to place the ${tool}. The side of the wall you point at sets which way it faces.`;
    case "room":
      return "Hover inside an area enclosed by walls and click to place a room.";
    case "move":
      return n === 0
        ? "Click a base point to move the selection from (select elements first)."
        : "Click the destination. Joined walls stretch to follow.";
    case "dimension":
      return n === 0
        ? "Pick a wall face, centerline, grid or point (Tab for the next reference)."
        : n === 1
          ? "Pick the next reference."
          : "Pick more references, or click in empty space to place the dimension.";
    case "dimensionLinear":
      return n < 2
        ? "Pick points or references; the dimension is horizontal or vertical."
        : "Pick more, or click above/below (horizontal) or beside (vertical) to place it.";
    case "dimensionAngular":
      return n === 0
        ? "Pick the first wall or grid line."
        : n === 1
          ? "Pick the second line."
          : "Click inside the angle to place the arc.";
    case "text":
      return "Click where the text note goes.";
    case "spotElevation":
      return n === 0
        ? "Click the point to measure (a floor, the ground, or any height in an elevation or section)."
        : "Click where the elevation goes (click the same point again for no leader).";
    case "detailLine":
    case "modelLine":
      return n === 0
        ? `Click to start the ${tool === "detailLine" ? "detail" : "model"} line. Pick a draw mode and line style in the options bar.`
        : "Click the next point. Esc finishes.";
    case "spotSlope":
      return view === "Plan"
        ? "Click a sloped roof, ramp, sidewalk or the ground; the arrow points downhill."
        : "Click on a sloped edge (a roof, ramp or the ground in a section).";
    case "northArrow":
      return "Click where the north arrow goes. Its look is set on the Standards tab.";
    case "graphicScale":
      return "Click the graphic scale's left end. Its divisions follow the view's scale.";
    case "keyPlan":
      return "Click where the key plan goes; the area this sheet's plans show is shaded.";
    case "paint":
      return "Click walls, floors, ceilings, roofs, columns or beams to paint them. Shift-click paints every element of that type. Esc finishes.";
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

/** Feeds a key into the shortcut buffer; returns the tool or command when a shortcut
 * completes (with the owner's Keyboard Shortcuts overrides). */
export function shortcut(
  buffer: string,
  key: string,
): { buffer: string; tool: Tool | null; action: Action | null } {
  if (!/^[a-z]$/i.test(key)) return { buffer: "", tool: null, action: null };
  const next = (buffer + key.toUpperCase()).slice(-2);
  const c = keyMap().get(next);
  if (!c) return { buffer: next, tool: null, action: null };
  return { buffer: "", tool: "tool" in c ? c.tool : null, action: "action" in c ? c.action : null };
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
