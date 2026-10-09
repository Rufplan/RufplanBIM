import { describe, expect, it } from "vitest";
import { gradeAt, hazeVisibility, hourAtAltitude, lookOf, LOOKS, mistOf } from "./looks";
import { NO_GRADE } from "./post";

// Looks (ADR-120): studio presets of atmosphere, light, lens and grade.
describe("looks", () => {
  it("maps the sliders to visibilities", () => {
    expect(hazeVisibility(0)).toBe(0);
    expect(hazeVisibility(1)).toBeCloseTo(300_000, -1);
    // Natural keeps the old 2.5 km aerial perspective (ADR-102's 2500 m to 63%… near it).
    expect(hazeVisibility(lookOf("natural").haze) / 1e6).toBeGreaterThan(2);
    expect(hazeVisibility(lookOf("natural").haze) / 1e6).toBeLessThan(3.5);
    expect(mistOf(0).visibility).toBe(0);
    expect(mistOf(1).visibility).toBeCloseTo(80_000, -1);
    expect(mistOf(1).height).toBe(26_000);
  });

  it("dials a grade back to neutral at strength 0", () => {
    for (const l of LOOKS) {
      const g = gradeAt(l, 0);
      expect(g.saturation).toBe(1);
      expect(g.contrast).toBe(1);
      expect(g.kelvin).toBe(6500);
      expect(g.grade).toEqual({ ...NO_GRADE, shadows: [0, 0, 0], highlights: [0, 0, 0] });
    }
    // At full strength, Nordic Mist mutes greens and lifts blacks.
    const n = gradeAt(lookOf("nordic"), 1);
    expect(n.grade.greens).toBeLessThan(0.8);
    expect(n.grade.fade).toBeGreaterThan(0.01);
  });

  it("finds the golden and blue hours from the sun's altitude", async () => {
    // A sun that sets at 7 PM, falling 12° an hour through the evening.
    const sunAt = async (h: number) => (19 - h) * 12;
    expect(await hourAtAltitude(sunAt, 7)).toBe(18.5);
    expect(await hourAtAltitude(sunAt, -4)).toBe(19.25);
    expect(await hourAtAltitude(async () => 60, 7)).toBeNull();
  });

  it("has a unique id per look", () => {
    expect(new Set(LOOKS.map((l) => l.id)).size).toBe(LOOKS.length);
  });
});
