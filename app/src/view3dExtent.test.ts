import { describe, expect, it, vi } from "vitest";
import * as THREE from "three";
import { groundGrid, modelBox } from "./components/View3D";

// The 3D view's extent (crash fix, 2026-10-09): one stray or non-finite element mustn't
// size the ground grid to millions of lines and run the page out of memory.
describe("the 3D view's extent", () => {
  const box = (x: number, size: number) => {
    const m = new THREE.Mesh(new THREE.BoxGeometry(size, size, size));
    m.position.set(x, 0, 0);
    return m;
  };

  it("leaves out strays and keeps the model", () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    const g = new THREE.Group();
    g.add(box(0, 10_000), box(20_000, 4000));
    const stray = box(5e12, 1000);
    stray.userData.category = "Wall";
    g.add(stray);
    const bad = new THREE.Mesh(new THREE.BufferGeometry());
    bad.geometry.setAttribute(
      "position",
      new THREE.Float32BufferAttribute([0, 0, 0, Infinity, 0, 0, 0, 1, 0], 3),
    );
    g.add(bad);
    const b = modelBox(g);
    expect(b.min.x).toBeCloseTo(-5000);
    expect(b.max.x).toBeCloseTo(22_000);
    expect(warn).toHaveBeenCalledOnce();
    expect(String(warn.mock.calls[0]![0])).toContain("Wall");
    warn.mockRestore();
  });

  it("keeps the ground grid to a few hundred lines whatever the extent", () => {
    for (const half of [50_000, 1e12, Infinity, NaN]) {
      const grid = groundGrid(new THREE.Vector3(), half, 0);
      let points = 0;
      grid.traverse((o) => {
        const g = (o as THREE.LineSegments).geometry;
        if (g) points += g.getAttribute("position").count;
      });
      expect(points).toBeGreaterThan(0);
      expect(points).toBeLessThanOrEqual(2 * 2 * 501 * 2);
    }
  });
});
