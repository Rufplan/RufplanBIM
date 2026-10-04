import { describe, expect, it } from "vitest";
import { areaCursor, blockGhost, hoverCursor } from "./blocks";
import type { DragArea } from "./bindings/DragArea";

// A text block on a sheet stretches by its edges with Revit's push-pull cursor (ADR-112).
describe("text blocks on sheets", () => {
  const area = (key: string, x0: number, x1: number): DragArea => ({
    id: "g",
    key,
    min: { x: x0, y: 100 },
    max: { x: x1, y: 160 },
    at: { x: (x0 + x1) / 2, y: 130 },
  });
  const areas = [
    area("block_left", 98, 102),
    area("block_right", 178, 182),
    area("block_move", 100, 180),
  ];

  it("shows the block with the dragged edge, never under 25 mm wide", () => {
    expect(blockGhost(areas, areas[1]!, { x: 220, y: 0 })).toEqual([100, 100, 220, 160]);
    expect(blockGhost(areas, areas[0]!, { x: 60, y: 0 })).toEqual([60, 100, 180, 160]);
    expect(blockGhost(areas, areas[1]!, { x: 0, y: 0 })).toEqual([100, 100, 125, 160]);
    expect(blockGhost(areas, areas[2]!, { x: 0, y: 0 })).toBeNull();
    const top = area("block_top", 100, 180);
    const bottom = area("block_bottom", 100, 180);
    expect(blockGhost([...areas, top], top, { x: 0, y: 190 })).toEqual([100, 100, 180, 190]);
    expect(blockGhost([...areas, bottom], bottom, { x: 0, y: 80 })).toEqual([100, 80, 180, 160]);
  });

  it("uses the push-pull cursor on edges and move on the block", () => {
    expect(areaCursor("block_right")).toBe("ew-resize");
    expect(areaCursor("block_left")).toBe("ew-resize");
    expect(areaCursor("block_move")).toBe("move");
    expect(areaCursor("block_top")).toBe("ns-resize");
    expect(hoverCursor("block_bottom")).toBe("ns-resize");
    expect(hoverCursor("block_move")).toBe("");
    expect(areaCursor("title_move")).toBe("");
  });
});
