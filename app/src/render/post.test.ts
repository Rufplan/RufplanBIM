import { describe, expect, it } from "vitest";
import { blackbody, whiteBalance } from "./post";

// The finishing pass's camera white balance (ADR-118), in kelvin as V-Ray's and Corona's.
describe("white balance", () => {
  const lum = (c: [number, number, number]) => 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];

  it("is neutral at 6500 K and keeps luminance", () => {
    const n = whiteBalance(6500);
    for (const v of n) expect(v).toBeCloseTo(1, 6);
    expect(lum(whiteBalance(4000))).toBeCloseTo(1, 6);
    expect(lum(whiteBalance(9000))).toBeCloseTo(1, 6);
  });

  it("warms the image when set above the light's temperature, as a camera does", () => {
    const warm = whiteBalance(8000);
    expect(warm[0]).toBeGreaterThan(1);
    expect(warm[2]).toBeLessThan(1);
    const cool = whiteBalance(5000);
    expect(cool[0]).toBeLessThan(1);
    expect(cool[2]).toBeGreaterThan(1);
  });

  it("follows the black body: tungsten is orange, daylight white, shade blue", () => {
    const t = blackbody(2700);
    expect(t[0]).toBeGreaterThan(t[1]);
    expect(t[1]).toBeGreaterThan(t[2]);
    const sky = blackbody(12000);
    expect(sky[2]).toBeGreaterThan(sky[0]);
  });
});
