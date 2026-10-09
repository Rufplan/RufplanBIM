import { describe, expect, it } from "vitest";
import { multipleScattering, skyRadiance, skyTable, sunTransmittance } from "./atmosphere";

// The physical sky (ADR-118): Rayleigh and Mie scattering and ozone in a curved atmosphere.
describe("physical sky", () => {
  const deg = Math.PI / 180;

  it("is blue overhead at midday, paler toward the horizon", () => {
    const zen = skyRadiance(89 * deg, Math.PI, 60 * deg);
    const hor = skyRadiance(2 * deg, Math.PI, 60 * deg);
    expect(zen[2]).toBeGreaterThan(zen[0] * 2);
    // The horizon is whiter: its blue to red ratio is lower.
    expect(hor[2] / hor[0]).toBeLessThan(zen[2] / zen[0]);
  });

  it("is brightest around the sun, from the haze's forward scattering", () => {
    const near = skyRadiance(32 * deg, 5 * deg, 30 * deg);
    const far = skyRadiance(32 * deg, 175 * deg, 30 * deg);
    expect(near[1]).toBeGreaterThan(far[1] * 3);
  });

  it("turns the sun orange as it sets, white overhead", () => {
    const high = sunTransmittance(70);
    const low = sunTransmittance(3);
    expect(high[2] / high[0]).toBeGreaterThan(0.7);
    expect(low[2] / low[0]).toBeLessThan(0.3);
    expect(low[0]).toBeLessThan(high[0]);
  });

  it("looks up the same values it traces", () => {
    const t = skyTable(40 * deg, 24, 32);
    const direct = skyRadiance(
      (Math.PI / 2) * (5 / 23) ** 2,
      0,
      40 * deg,
      32,
      multipleScattering(40 * deg),
    );
    const looked = t((Math.PI / 2) * (5 / 23) ** 2, 0);
    for (let c = 0; c < 3; c++) expect(looked[c]).toBeCloseTo(direct[c]!, 6);
  });

  it("keeps twilight blue overhead and lights the earth's shadow (multiple scattering)", () => {
    const sun = -4 * deg;
    const ms = multipleScattering(sun);
    const single = skyRadiance(89 * deg, 0, sun);
    const both = skyRadiance(89 * deg, 0, sun, 32, ms);
    // Single scattering alone leaves the zenith violet (red well up toward blue); the second
    // bounce, blue first, makes it blue: red no more than half of blue.
    expect(single[0] / single[2]).toBeGreaterThan(both[0] / both[2] + 0.1);
    expect(both[0] / both[2]).toBeLessThan(0.5);
    // Opposite the set sun, low down, is the earth's shadow: black with one bounce.
    const shadow = skyRadiance(10 * deg, Math.PI, -6 * deg);
    const lit = skyRadiance(10 * deg, Math.PI, -6 * deg, 32, multipleScattering(-6 * deg));
    expect(Math.max(...shadow)).toBe(0);
    expect(lit[2]).toBeGreaterThan(lit[0]);
    expect(lit[2]).toBeGreaterThan(0);
  });
});
