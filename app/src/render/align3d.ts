// Align in 3D (ADR-097), as Revit's: pick a face to align to, then a parallel face on the
// element to move; it moves along the first face's normal until the faces are coplanar.
// The faces are planar regions of the meshes (z-up mm), found from the hit triangle.
import * as THREE from "three";

/** A picked face: its element, a point on it and its unit normal (world, z-up mm), and its
 * triangles (9 floats each) for the highlight. */
export interface AlignFace {
  el: string;
  point: THREE.Vector3;
  normal: THREE.Vector3;
  tris: Float32Array;
}

/** Faces this close to parallel align (about 3°). */
const PARALLEL = Math.cos((3 * Math.PI) / 180);
/** Triangles within this of the face's plane (mm) belong to it. */
const ON_PLANE = 1.5;

/** The face of `mesh` hit at `point` with object-space `faceNormal`: every triangle of the
 * mesh lying in that plane, so a wall's whole exterior face lights, not one triangle. */
export function faceAt(
  mesh: THREE.Mesh,
  point: THREE.Vector3,
  faceNormal: THREE.Vector3,
): AlignFace | null {
  const el = mesh.userData.el as string | undefined;
  const pos = mesh.geometry.getAttribute("position");
  if (!el || !pos) return null;
  const normal = faceNormal.clone().transformDirection(mesh.matrixWorld).normalize();
  const index = mesh.geometry.getIndex();
  const count = index ? index.count : pos.count;
  const a = new THREE.Vector3();
  const b = new THREE.Vector3();
  const c = new THREE.Vector3();
  const n = new THREE.Vector3();
  const out: number[] = [];
  const at = (i: number, v: THREE.Vector3) =>
    v.fromBufferAttribute(pos, index ? index.getX(i) : i).applyMatrix4(mesh.matrixWorld);
  for (let i = 0; i + 2 < count; i += 3) {
    at(i, a);
    at(i + 1, b);
    at(i + 2, c);
    n.subVectors(c, b).cross(new THREE.Vector3().subVectors(a, b));
    if (n.lengthSq() < 1e-12) continue;
    n.normalize();
    if (Math.abs(n.dot(normal)) < 0.999) continue;
    const off = (v: THREE.Vector3) => Math.abs(v.clone().sub(point).dot(normal));
    if (off(a) > ON_PLANE || off(b) > ON_PLANE || off(c) > ON_PLANE) continue;
    out.push(a.x, a.y, a.z, b.x, b.y, b.z, c.x, c.y, c.z);
  }
  return { el, point: point.clone(), normal, tris: new Float32Array(out) };
}

/** Whether `target` can align to `reference`: on another element, and parallel. */
export function canAlign(reference: AlignFace, target: AlignFace): boolean {
  return target.el !== reference.el && Math.abs(reference.normal.dot(target.normal)) >= PARALLEL;
}

/** How far the target's element moves (x, y, z mm) so its face lies on the reference's
 * plane: along the reference's normal only, as Revit's Align. */
export function alignDelta(reference: AlignFace, target: AlignFace): [number, number, number] {
  const n = reference.normal;
  const d = reference.point.clone().sub(target.point).dot(n);
  return [n.x * d, n.y * d, n.z * d];
}

/** A highlight for a face: a tinted, slightly lifted fill and its outline. `strong` for the
 * picked reference, lighter for the face under the cursor. */
export function faceHighlight(face: AlignFace, strong: boolean): THREE.Group {
  const g = new THREE.Group();
  const lifted = new Float32Array(face.tris.length);
  // Lift it a hair off the surface toward the viewer's side of either face.
  for (let i = 0; i < face.tris.length; i += 3) {
    lifted[i] = face.tris[i]! + face.normal.x * 2;
    lifted[i + 1] = face.tris[i + 1]! + face.normal.y * 2;
    lifted[i + 2] = face.tris[i + 2]! + face.normal.z * 2;
  }
  const geo = new THREE.BufferGeometry();
  geo.setAttribute("position", new THREE.BufferAttribute(lifted, 3));
  const fill = new THREE.Mesh(
    geo,
    new THREE.MeshBasicMaterial({
      color: strong ? 0x1f7ae0 : 0x3ecff7,
      transparent: true,
      opacity: strong ? 0.45 : 0.3,
      side: THREE.DoubleSide,
      depthWrite: false,
      polygonOffset: true,
      polygonOffsetFactor: -4,
      polygonOffsetUnits: -4,
    }),
  );
  fill.renderOrder = 10;
  const edges = new THREE.LineSegments(
    new THREE.EdgesGeometry(geo, 30),
    new THREE.LineBasicMaterial({ color: strong ? 0x1f7ae0 : 0x3ecff7, depthTest: false }),
  );
  edges.renderOrder = 11;
  g.add(fill, edges);
  g.userData.align = true;
  return g;
}
