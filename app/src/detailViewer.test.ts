import { describe, expect, it } from "vitest";
import { FIT, fitSize, step, wheelFactor, zoomAt, ZOOM_MAX, ZOOM_MIN } from "./detailViewer";

// The Detail Viewer's zoom and pan (the enlarged detail before inserting).
describe("detail viewer", () => {
  it("zooms about a point, keeping that point still", () => {
    const v = zoomAt(FIT, 2, 100, -50);
    expect(v).toEqual({ zoom: 2, tx: -100, ty: 50 });
    // A drawing point under the cursor (p = t + s·zoom) stays under it.
    const s = (100 - FIT.tx) / FIT.zoom;
    expect(v.tx + s * v.zoom).toBeCloseTo(100);
    // About the centre, the pan scales with the zoom.
    expect(zoomAt({ zoom: 2, tx: 40, ty: 0 }, 4)).toEqual({ zoom: 4, tx: 80, ty: 0 });
  });

  it("keeps zoom between 50% and 800%", () => {
    expect(zoomAt(FIT, 100).zoom).toBe(ZOOM_MAX);
    expect(zoomAt(FIT, 0.01).zoom).toBe(ZOOM_MIN);
    expect(ZOOM_MIN).toBe(0.5);
    expect(ZOOM_MAX).toBe(8);
    // The wheel: up zooms in, down out.
    expect(wheelFactor(-100)).toBeGreaterThan(1);
    expect(wheelFactor(100)).toBeCloseTo(Math.exp(-0.15));
  });

  it("fits a drawing inside the canvas, keeping its proportions", () => {
    expect(fitSize(1400, 820, 700, 700)).toEqual([700, 410]);
    expect(fitSize(1000, 2000, 800, 400)).toEqual([200, 400]);
    expect(fitSize(0, 10, 100, 100)).toEqual([0, 0]);
  });

  it("browses the filtered list, wrapping at either end", () => {
    const ids = ["a", "b", "c"];
    expect(step(ids, "a", 1)).toBe("b");
    expect(step(ids, "c", 1)).toBe("a");
    expect(step(ids, "a", -1)).toBe("c");
    expect(step(ids, "z", 1)).toBe("a");
    expect(step([], "a", 1)).toBeNull();
  });
});
