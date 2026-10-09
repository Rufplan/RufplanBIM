import { describe, expect, it } from "vitest";
import { extinction, fogShare, mistDepth, type FogSettings } from "./fog";

// The atmosphere (ADR-120): Koschmieder haze and exponential height mist in closed form.
describe("fog", () => {
  const f: FogSettings = {
    visibility: 2_000_000,
    mistVisibility: 200_000,
    mistHeight: 15_000,
    ground: 0,
    color: [1, 1, 1],
    sunDir: null,
    glow: 0,
    sunColor: [1, 1, 1],
  };

  it("hides a dark object to 2% contrast at the visibility (Koschmieder)", () => {
    const clear = { ...f, mistVisibility: 0 };
    expect(fogShare(clear, 1600, 0, 2_000_000)).toBeCloseTo(0.98, 3);
    expect(extinction(0)).toBe(0);
  });

  it("integrates the mist along a ray exactly", () => {
    // From eye height, down toward the lawn 300 m off, and up and away.
    for (const [y0, dy, len] of [
      [1600, -0.005, 300_000],
      [1600, 0.08, 400_000],
      [40_000, -0.1, 380_000],
      [1600, 0, 250_000],
    ] as const) {
      const n = 20_000;
      let sum = 0;
      for (let i = 0; i < n; i++) {
        const y = y0 + dy * ((i + 0.5) / n) * len;
        sum += extinction(f.mistVisibility) * Math.exp(-(y - f.ground) / f.mistHeight) * (len / n);
      }
      expect(mistDepth(f, y0, dy, len) / sum).toBeCloseTo(1, 4);
    }
  });

  it("sinks the low sky into the mist but leaves it clear overhead", () => {
    const low = fogShare(f, 1600, 0.01, Infinity);
    const high = fogShare(f, 1600, 0.7, Infinity);
    expect(low).toBeGreaterThan(0.9);
    expect(high).toBeLessThan(0.35);
    // The building rises out of it: a wall 30 m up is less hidden than its base.
    expect(fogShare(f, 1600, 0.1, 300_000)).toBeLessThan(fogShare(f, 1600, 0, 300_000));
  });
});

describe("fog start", () => {
  it("leaves the near field clear and builds from where it starts", () => {
    const f: FogSettings = {
      visibility: 1_000_000,
      mistVisibility: 0,
      mistHeight: 0,
      start: 25_000,
      ground: 0,
      color: [1, 1, 1],
      sunDir: null,
      glow: 0,
      sunColor: [1, 1, 1],
    };
    expect(fogShare(f, 1600, 0, 20_000)).toBe(0);
    // 1 km of haze past the start: 98%.
    expect(fogShare(f, 1600, 0, 1_025_000)).toBeCloseTo(0.98, 3);
  });
});
