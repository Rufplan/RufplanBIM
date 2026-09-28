import { describe, expect, it } from "vitest";
import { closesSketch, promptFor, shortcut, toolAllowed } from "./tools";
import { fit, toModel, toScreen, zoomAt } from "./canvas/render";

describe("tools", () => {
  it("two-letter shortcuts", () => {
    let r = shortcut("", "w");
    expect(r).toEqual({ buffer: "W", tool: null, action: null });
    r = shortcut(r.buffer, "a");
    expect(r).toEqual({ buffer: "", tool: "wall", action: null });
    expect(shortcut("G", "R").tool).toBe("grid");
    expect(shortcut("D", "R").tool).toBe("door");
    expect(shortcut("W", "N").tool).toBe("window");
    expect(shortcut("R", "M").tool).toBe("room");
    expect(shortcut("M", "V").tool).toBe("move");
    expect(shortcut("X", "1")).toEqual({ buffer: "", tool: null, action: null });
  });

  it("symbols go where Revit puts them (ADR-048)", () => {
    for (const v of ["Plan", "CeilingPlan", "Elevation", "Section"] as const) {
      expect(toolAllowed("spotElevation", v)).toBe(true);
      expect(toolAllowed("graphicScale", v)).toBe(true);
      expect(toolAllowed("keyPlan", v)).toBe(false);
    }
    expect(toolAllowed("northArrow", "Plan")).toBe(true);
    expect(toolAllowed("northArrow", "Sheet")).toBe(true);
    expect(toolAllowed("northArrow", "Elevation")).toBe(false);
    expect(toolAllowed("keyPlan", "Sheet")).toBe(true);
    expect(toolAllowed("spotElevation", "Sheet")).toBe(false);
    expect(toolAllowed("spotElevation", "ThreeD")).toBe(false);
    expect(promptFor("spotElevation", 1, "Plan")).toContain("same point again for no leader");
    expect(promptFor("keyPlan", 0, "Plan")).toBe("Open a sheet to place a key plan.");
    // Lines (ADR-054): detail lines in 2D views and sheets, model lines in plans.
    for (const v of ["Plan", "CeilingPlan", "Elevation", "Section", "Sheet"] as const)
      expect(toolAllowed("detailLine", v)).toBe(true);
    expect(toolAllowed("detailLine", "ThreeD")).toBe(false);
    expect(toolAllowed("modelLine", "Plan")).toBe(true);
    expect(toolAllowed("modelLine", "Elevation")).toBe(false);
    // Spot slopes (ADR-049): plans, elevations and sections.
    for (const v of ["Plan", "Elevation", "Section"] as const)
      expect(toolAllowed("spotSlope", v)).toBe(true);
    expect(toolAllowed("spotSlope", "CeilingPlan")).toBe(false);
    expect(toolAllowed("spotSlope", "Sheet")).toBe(false);
    expect(promptFor("spotSlope", 0, "Plan")).toContain("points downhill");
  });

  it("tools are limited to views where they make sense", () => {
    expect(toolAllowed("wall", "Plan")).toBe(true);
    expect(toolAllowed("wall", "Elevation")).toBe(false);
    expect(toolAllowed("level", "Elevation")).toBe(true);
    expect(toolAllowed("level", "Plan")).toBe(false);
    expect(toolAllowed("door", "Plan")).toBe(true);
    // Doors, windows and fixtures go on wall faces in elevations and sections (ADR-059).
    for (const tool of ["door", "window", "light", "dimensionAngular"] as const)
      for (const v of ["Elevation", "Section"] as const) expect(toolAllowed(tool, v)).toBe(true);
    expect(toolAllowed("window", "Sheet")).toBe(false);
    expect(promptFor("window", 0, "Elevation")).toMatch(/faces you/);
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
