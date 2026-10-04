import { describe, expect, it } from "vitest";
import { textFrameGhost } from "./canvas/render";

// The live preview while a text note's grips are dragged (ADR-108), as Revit shows it.
describe("text note grips' preview", () => {
  const box = [
    { x: 0, y: 0 },
    { x: 100, y: 0 },
    { x: 100, y: 20 },
    { x: 0, y: 20 },
  ];
  it("moves the box with the move grip", () => {
    const g = textFrameGhost(box, "text_move_grip", { x: -5, y: 25 }, { x: 5, y: 35 })!;
    expect(g[0]).toEqual({ x: 10, y: 10 });
  });
  it("widens only the dragged side", () => {
    const right = textFrameGhost(box, "text_width", { x: 100, y: 10 }, { x: 130, y: 50 })!;
    expect(right[1]).toEqual({ x: 130, y: 0 });
    expect(right[0]).toEqual({ x: 0, y: 0 });
    const left = textFrameGhost(box, "text_width_left", { x: 0, y: 10 }, { x: 20, y: 10 })!;
    expect(left[0]).toEqual({ x: 20, y: 0 });
    expect(left[1]).toEqual({ x: 100, y: 0 });
  });
  it("turns about the centre, snapping near 90°", () => {
    // From the top-right grip to straight above-left of centre: about 88°.
    const c = { x: 50, y: 10 };
    const from = { x: 105, y: 25 };
    const a = Math.atan2(from.y - c.y, from.x - c.x) + (88 * Math.PI) / 180;
    const to = { x: c.x + Math.cos(a) * 50, y: c.y + Math.sin(a) * 50 };
    const g = textFrameGhost(box, "text_rotate", from, to)!;
    // The bottom edge now runs straight up.
    expect(g[1]!.x - g[0]!.x).toBeCloseTo(0, 6);
    expect(g[1]!.y - g[0]!.y).toBeCloseTo(100, 6);
  });
});
