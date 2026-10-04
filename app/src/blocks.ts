import type { DragArea } from "./bindings/DragArea";
import type { Pt } from "./bindings/Pt";

// Blocks of sheet text (ADR-112): the detail groups the sets make of each box of notes drag
// whole by their `block_move` area and stretch by their `block_left` / `block_right` edges.

/** The narrowest a block stretches to (paper mm), as studio-sheets `blocks::MIN_WIDTH`. */
export const MIN_BLOCK_WIDTH = 25;

/** A block being stretched by its edge area `area` to `to`, as [x0, y0, x1, y1]; null for
 * any other area. */
export function blockGhost(
  areas: DragArea[],
  area: DragArea,
  to: Pt,
): [number, number, number, number] | null {
  if (area.key !== "block_left" && area.key !== "block_right") return null;
  const block = areas.find((a) => a.id === area.id && a.key === "block_move");
  if (!block) return null;
  const { min, max } = block;
  return area.key === "block_right"
    ? [min.x, min.y, Math.max(to.x, min.x + MIN_BLOCK_WIDTH), max.y]
    : [Math.min(to.x, max.x - MIN_BLOCK_WIDTH), min.y, max.x, max.y];
}

/** The cursor over or pressing a sheet's drag area: move for views and blocks, Revit's
 * push-pull arrows for a block's edge. */
export function areaCursor(key: string): string {
  if (key === "view_move" || key === "block_move") return "move";
  if (key === "block_left" || key === "block_right") return "ew-resize";
  return "";
}
