import type { DrawOptions } from "./bindings/DrawOptions";
import type { DrawTool } from "./bindings/DrawTool";

/** Draw options for detail and model lines (ADR-054): no offset or fillet, polygon sides. */
export function lineOptions(ui: { mode: DrawTool; sides: number }): DrawOptions {
  return { offset: 0, radius: null, sides: Math.max(3, ui.sides || 6) };
}

/** The draw modes lines use (Revit's Draw panel for Detail Line and Model Line). */
export const DRAW_TOOLS: string[] = [
  "Line",
  "Rectangle",
  "InscribedPolygon",
  "CircumscribedPolygon",
  "Circle",
  "StartEndRadiusArc",
  "CenterEndsArc",
];
