import { describe, expect, it } from "vitest";
import * as THREE from "three";
import { fillGround, hdrSun, matchSunRotation, skyBackground } from "./render/backgrounds";

/** A tiny equirect HDR: blue sky, a bright sun at pixel (si, sj), black below. */
function sky(w: number, h: number, si: number, sj: number) {
  const data = new Float32Array(w * h * 4);
  for (let j = 0; j < h; j++)
    for (let i = 0; i < w; i++) {
      const k = (j * w + i) * 4;
      const up = j < h / 2;
      data.set(up ? [0.3, 0.5, 1] : [0, 0, 0], k);
      data[k + 3] = 1;
    }
  data.set([5000, 4800, 4500], (sj * w + si) * 4);
  return new THREE.DataTexture(data, w, h, THREE.RGBAFormat, THREE.FloatType);
}

describe("Sky Library (ADR-101)", () => {
  it("finds the sky's sun and turns it to the site's", () => {
    const t = sky(64, 32, 40, 6);
    const s = hdrSun(t)!;
    expect((s.azimuth * 180) / Math.PI).toBeCloseTo((40.5 / 64 - 0.5) * 360, 3);
    expect(s.elevation).toBeGreaterThan(0.9);
    // Turned by the result, the sun's angle lands on the site sun's (renderer convention:
    // the dome turns the opposite way to the angle).
    const sun: [number, number, number] = [-0.6, -0.6, 0.5];
    const rot = matchSunRotation(s.azimuth, sun);
    const site = Math.atan2(0.6, -0.6);
    const turned = s.azimuth - rot;
    expect(Math.cos(turned - site)).toBeCloseTo(1, 6);
  });

  it("fills a pure sky's black ground with lit ground", () => {
    const t = sky(64, 32, 10, 10);
    fillGround(t);
    const d = t.image.data as Float32Array;
    const below = (28 * 64 + 5) * 4;
    expect(d[below + 1]!).toBeGreaterThan(0.01);
    expect(d[below + 1]!).toBeGreaterThan(d[below + 2]!);
    // The sky is untouched.
    expect(d[(2 * 64 + 5) * 4 + 2]).toBe(1);
  });

  it("makes a library sky a lit photo background", () => {
    const b = skyBackground({ id: "x_puresky", name: "X", mood: "Clear", note: "" });
    expect(b).toMatchObject({ id: "sky:x_puresky", photo: true, ground: "grass" });
  });
});
