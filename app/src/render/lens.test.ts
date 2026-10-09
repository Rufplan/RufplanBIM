import { describe, expect, it } from "vitest";
import * as THREE from "three";
import { lensSample } from "./denoise";

// Depth of field's guide (ADR-120): a thin lens sampled by moving the camera across its
// aperture and shifting its frame, so the focal plane stays put and nearer and farther
// things blur.
describe("lens samples", () => {
  const setup = (shifted: boolean) => {
    const cam = new THREE.PerspectiveCamera(50, 16 / 9, 50, 1e7);
    cam.position.set(0, 1600, 20_000);
    cam.lookAt(0, 1600, 0);
    if (shifted) cam.setViewOffset(1000 * (16 / 9), 1000, 0, -120, 1000 * (16 / 9), 1000);
    cam.updateProjectionMatrix();
    cam.updateMatrixWorld();
    return cam;
  };
  const ndc = (cam: THREE.PerspectiveCamera, p: THREE.Vector3) => {
    cam.updateMatrixWorld();
    return p.clone().project(cam);
  };
  for (const shifted of [false, true]) {
    it(`keeps the focal plane still and moves the rest${shifted ? " (shifted lens)" : ""}`, () => {
      const cam = setup(shifted);
      const base = {
        position: cam.position.clone(),
        view: cam.view ? { ...cam.view } : null,
        w: 1280,
        h: 720,
      };
      // On the focal plane (20 m away), off axis; and a point 40 m beyond.
      const inFocus = new THREE.Vector3(3000, 2500, 0);
      const far = new THREE.Vector3(-4000, 900, -40_000);
      const a0 = ndc(cam, inFocus);
      const f0 = ndc(cam, far);
      lensSample(cam, base, 0, 0, 120, -80, 20_000);
      const a1 = ndc(cam, inFocus);
      const f1 = ndc(cam, far);
      expect(a1.x).toBeCloseTo(a0.x, 5);
      expect(a1.y).toBeCloseTo(a0.y, 5);
      expect(Math.hypot(f1.x - f0.x, f1.y - f0.y)).toBeGreaterThan(0.005);
    });
  }
});
