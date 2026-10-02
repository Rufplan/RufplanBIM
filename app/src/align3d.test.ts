import { describe, expect, it } from "vitest";
import * as THREE from "three";
import { alignDelta, canAlign, faceAt, faceHighlight } from "./render/align3d";
import { toolAllowed } from "./tools";

/** A box mesh (mm, z-up) for element `el`, from `min` to `max`. */
function box(el: string, min: number[], max: number[]): THREE.Mesh {
  const g = new THREE.BoxGeometry(max[0]! - min[0]!, max[1]! - min[1]!, max[2]! - min[2]!);
  g.translate((min[0]! + max[0]!) / 2, (min[1]! + max[1]!) / 2, (min[2]! + max[2]!) / 2);
  const m = new THREE.Mesh(g.toNonIndexed(), new THREE.MeshBasicMaterial());
  m.userData.el = el;
  m.updateMatrixWorld();
  return m;
}

describe("Align in 3D (ADR-097)", () => {
  // A wall face at x = 0, and a column whose west face is at x = -700.
  const wall = box("wall", [0, 0, 0], [200, 4000, 3000]);
  const column = box("column", [-700, 1000, 0], [-400, 1300, 3000]);

  it("lights the whole planar face under the cursor", () => {
    const f = faceAt(wall, new THREE.Vector3(0, 2000, 1500), new THREE.Vector3(-1, 0, 0))!;
    expect(f.el).toBe("wall");
    expect(f.tris.length / 9).toBe(2);
    expect(f.normal.x).toBeCloseTo(-1);
    expect(faceHighlight(f, true).children.length).toBe(2);
  });

  it("moves the target along the reference's normal until the faces meet", () => {
    const ref = faceAt(wall, new THREE.Vector3(0, 2000, 1500), new THREE.Vector3(-1, 0, 0))!;
    const west = faceAt(column, new THREE.Vector3(-700, 1100, 500), new THREE.Vector3(-1, 0, 0))!;
    expect(canAlign(ref, west)).toBe(true);
    // Only across, not up or along the wall, however far apart the picks are.
    const d = alignDelta(ref, west);
    expect(d[0]).toBeCloseTo(700);
    expect(d[1]).toBeCloseTo(0);
    expect(d[2]).toBeCloseTo(0);
    // A floor top onto a beam's underside: straight up.
    const top = faceAt(column, new THREE.Vector3(-500, 1100, 3000), new THREE.Vector3(0, 0, 1))!;
    const beam = box("beam", [0, 0, 3400], [2000, 300, 3800]);
    const under = faceAt(beam, new THREE.Vector3(500, 100, 3400), new THREE.Vector3(0, 0, -1))!;
    expect(alignDelta(under, top)[2]).toBeCloseTo(400);
  });

  it("refuses faces that aren't parallel or are on the same element", () => {
    const ref = faceAt(wall, new THREE.Vector3(0, 2000, 1500), new THREE.Vector3(-1, 0, 0))!;
    const south = faceAt(column, new THREE.Vector3(-500, 1000, 500), new THREE.Vector3(0, -1, 0))!;
    expect(canAlign(ref, south)).toBe(false);
    const back = faceAt(wall, new THREE.Vector3(200, 2000, 1500), new THREE.Vector3(1, 0, 0))!;
    expect(canAlign(ref, back)).toBe(false);
  });

  it("is a 3D tool as well as a 2D one", () => {
    expect(toolAllowed("align", "ThreeD")).toBe(true);
    expect(toolAllowed("align", "Plan")).toBe(true);
  });
});
