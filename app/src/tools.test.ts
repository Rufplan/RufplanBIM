import { describe, expect, it } from "vitest";
import { closesSketch, promptFor, shortcut, toolAllowed } from "./tools";
import { fit, toModel, toScreen, zoomAt } from "./canvas/render";

describe("tools", () => {
  it("two-letter shortcuts", () => {
    let r = shortcut("", "w");
    expect(r).toEqual({ buffer: "W", tool: null });
    r = shortcut(r.buffer, "a");
    expect(r).toEqual({ buffer: "", tool: "wall" });
    expect(shortcut("G", "R").tool).toBe("grid");
    expect(shortcut("D", "R").tool).toBe("door");
    expect(shortcut("W", "N").tool).toBe("window");
    expect(shortcut("R", "M").tool).toBe("room");
    expect(shortcut("M", "V").tool).toBe("move");
    expect(shortcut("X", "1")).toEqual({ buffer: "", tool: null });
  });

  it("tools are limited to views where they make sense", () => {
    expect(toolAllowed("wall", "Plan")).toBe(true);
    expect(toolAllowed("wall", "Elevation")).toBe(false);
    expect(toolAllowed("level", "Elevation")).toBe(true);
    expect(toolAllowed("level", "Plan")).toBe(false);
    expect(toolAllowed("door", "Plan")).toBe(true);
    expect(toolAllowed("window", "Elevation")).toBe(false);
    expect(toolAllowed("room", "Plan")).toBe(true);
    expect(toolAllowed("room", "CeilingPlan")).toBe(false);
    expect(promptFor("move", 1, "Plan")).toMatch(/destination/i);
    expect(promptFor("level", 0, "Plan")).toMatch(/elevation/i);
  });

  it("sketch closes near the first point after three corners", () => {
    expect(closesSketch([100, 100], [104, 103], 3)).toBe(true);
    expect(closesSketch([100, 100], [104, 103], 2)).toBe(false);
    expect(closesSketch([100, 100], [130, 100], 4)).toBe(false);
  });
});

describe("camera", () => {
  it("screen and model transforms invert each other, y up", () => {
    const cam = { cx: 1000, cy: 500, zoom: 0.1 };
    const [sx, sy] = toScreen(cam, 800, 600, 2000, 1500);
    expect([sx, sy]).toEqual([500, 200]);
    const p = toModel(cam, 800, 600, sx, sy);
    expect(p.x).toBeCloseTo(2000);
    expect(p.y).toBeCloseTo(1500);
  });

  it("zoom keeps the point under the cursor fixed", () => {
    const cam = fit([0, 0, 10000, 8000], 800, 600);
    const before = toModel(cam, 800, 600, 200, 150);
    const after = toModel(zoomAt(cam, 800, 600, 200, 150, 2), 800, 600, 200, 150);
    expect(after.x).toBeCloseTo(before.x);
    expect(after.y).toBeCloseTo(before.y);
  });
});
