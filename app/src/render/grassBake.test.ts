import { describe, expect, it } from "vitest";
import * as THREE from "three";
import { mergeGeometries } from "three/examples/jsm/utils/BufferGeometryUtils.js";
import { bakeClumps } from "./grass";

// Render grass baked in place (crash fix, 2026-10-09): the same geometry as cloning each
// clump and merging, without 120,000 copies at once; capped in triangles.
describe("grass baking", () => {
  const clump = () => {
    const g = new THREE.BufferGeometry();
    g.setAttribute(
      "position",
      new THREE.BufferAttribute(new Float32Array([0, 0, 0, 10, 0, 0, 0, 0, 50, 10, 0, 50]), 3),
    );
    g.setAttribute(
      "normal",
      new THREE.BufferAttribute(new Float32Array([0, -1, 0, 0, -1, 0, 0, -1, 0, 0, -1, 0]), 3),
    );
    g.setAttribute("color", new THREE.BufferAttribute(new Float32Array(12).fill(0.5), 3));
    g.setIndex([0, 1, 2, 1, 3, 2]);
    return g;
  };
  const place = (n: number) =>
    Array.from({ length: n }, (_, i) =>
      new THREE.Matrix4().makeRotationZ(i * 0.7).setPosition(i * 1000, i * 500, 0),
    );

  it("matches cloning and merging", () => {
    const geo = clump();
    const ms = place(5);
    const tints = ms.map((_, i) => new THREE.Color(1, 0.5 + i * 0.1, 0.2));
    const pre = new THREE.Matrix4().makeRotationX(-Math.PI / 2);
    const baked = bakeClumps(
      geo,
      ms,
      ms.map((_, i) => i * 1000),
      tints,
      null,
      pre,
    )!;
    const pieces = ms.map((m, i) => {
      const p = geo.clone().applyMatrix4(pre.clone().multiply(m));
      const c = p.getAttribute("color");
      for (let v = 0; v < c.count; v++)
        c.setXYZ(v, c.getX(v) * tints[i]!.r, c.getY(v) * tints[i]!.g, c.getZ(v) * tints[i]!.b);
      return p;
    });
    const merged = mergeGeometries(pieces, false)!;
    for (const n of ["position", "normal", "color"]) {
      const a = baked.getAttribute(n).array;
      const b = merged.getAttribute(n).array;
      expect(a.length).toBe(b.length);
      for (let i = 0; i < a.length; i++) expect(a[i]).toBeCloseTo(b[i]!, 3);
    }
    expect(Array.from(baked.index!.array)).toEqual(Array.from(merged.index!.array));
  });

  it("keeps to its triangle budget: the light clump nearer, then the farthest go", () => {
    const geo = clump(); // 2 triangles
    const light = new THREE.BufferGeometry().copy(clump());
    light.setIndex([0, 1, 2]); // 1 triangle
    const ms = place(100);
    const dist = ms.map((_, i) => i * 300);
    // 100 full clumps would be 200; the light one beyond 16 m gives 154; a budget of 120
    // brings it nearer (to 4 m at the closest), then drops the farthest.
    const baked = bakeClumps(geo, ms, dist, null, { geo: light, near: 16_000 }, undefined, 120)!;
    expect(baked.index!.count / 3).toBeLessThanOrEqual(120);
    expect(baked.index!.count / 3).toBeGreaterThan(100);
    const unlimited = bakeClumps(geo, ms, dist, null, null, undefined, 1e9)!;
    expect(unlimited.index!.count / 3).toBe(200);
  });
});
