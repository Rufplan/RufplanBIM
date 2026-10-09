import { describe, expect, it } from "vitest";
import * as THREE from "three";
import { splitByUv } from "./plants";

// Flower cards render with their own material (ADR-119): split from the leaves by the
// atlas cell their texture coordinates are in.
describe("splitting leaf and flower cards", () => {
  it("puts each triangle with the cell its corners are in, keeping every attribute", () => {
    const g = new THREE.BufferGeometry();
    g.setAttribute("position", new THREE.Float32BufferAttribute(new Array(18).fill(0), 3));
    // A leaf triangle (v < 0.5), then a flower triangle (v ≥ 0.5).
    g.setAttribute(
      "uv",
      new THREE.Float32BufferAttribute([0, 0.1, 0.2, 0.2, 0.1, 0.4, 0, 0.6, 0.2, 0.7, 0.1, 0.9], 2),
    );
    g.setAttribute("color", new THREE.Float32BufferAttribute(new Array(24).fill(0.5), 4));
    const [leaves, flowers] = splitByUv(g, 0.5);
    expect(leaves!.getAttribute("position").count).toBe(3);
    expect(flowers!.getAttribute("position").count).toBe(3);
    expect(flowers!.getAttribute("color").itemSize).toBe(4);
    expect(flowers!.getAttribute("uv").getY(0)).toBeCloseTo(0.6, 6);
    const [onlyLeaves, none] = splitByUv(leaves!, 0.5);
    expect(onlyLeaves!.getAttribute("position").count).toBe(3);
    expect(none).toBeNull();
  });
});
