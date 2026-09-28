// Enscape's Grass material type (ADR-064): real 3D blades grown on any surface finished in
// a material whose appearance has grass settings (height and height variation), plus
// fallen pine cones on pine straw. Like Enscape, the blades grow densely only near the
// camera: clumps are scattered over the surfaces once (a field), and the nearest fill an
// instanced mesh, shrinking to nothing at its edge, as the camera moves; the lawn's
// texture carries on beyond. Renders take the field around their camera, merged.
import * as THREE from "three";
import { mergeGeometries } from "three/examples/jsm/utils/BufferGeometryUtils.js";
import { wind } from "./plants";

export interface GrassSettings {
  height: number;
  variation: number;
}

/** A surface to grow on: its triangles (9 floats each, mm, z up) and its grass. */
export interface GrassSurface {
  positions: ArrayLike<number>;
  grass: GrassSettings | null;
  /** The material's colour (sRGB 0-255), for the blades' tint. */
  color: [number, number, number];
  /** Pine straw: pine cones instead of (or with) blades. */
  cones?: boolean;
}

/** A small deterministic random source. */
export function rng(seed: number) {
  let s = seed >>> 0 || 1;
  return () => {
    s ^= s << 13;
    s ^= s >>> 17;
    s ^= s << 5;
    return (s >>> 0) / 4294967296;
  };
}

const lin = (c: number) => {
  const v = c / 255;
  return v <= 0.04045 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4);
};

/** A clump of `blades` blades about 1 unit tall, spread over `radius` units (instances
 * scale it by the grass height), each a tapered, bending strip, darker at its root. */
export function clumpGeometry(blades = 12, radius = 1.1, seed = 1): THREE.BufferGeometry {
  const r = rng(seed);
  const pos: number[] = [];
  const col: number[] = [];
  const nor: number[] = [];
  const idx: number[] = [];
  const segs = 3;
  for (let b = 0; b < blades; b++) {
    const a = r() * Math.PI * 2;
    const d = Math.sqrt(r()) * radius;
    const base = [Math.cos(a) * d, Math.sin(a) * d];
    const lean = r() * 0.9 + 0.1;
    const dir = r() * Math.PI * 2;
    const h = 0.6 + r() * 0.4;
    const w = 0.03 + r() * 0.02;
    const tone = 0.8 + r() * 0.35;
    const start = pos.length / 3;
    for (let s = 0; s <= segs; s++) {
      const t = s / segs;
      // The blade bends over as it rises.
      const bend = lean * t * t * 0.45;
      const x = base[0]! + Math.cos(dir) * bend * h;
      const y = base[1]! + Math.sin(dir) * bend * h;
      const z = h * t;
      const half = w * (1 - t * 0.92);
      const sx = -Math.sin(dir) * half;
      const sy = Math.cos(dir) * half;
      pos.push(x - sx, y - sy, z, x + sx, y + sy, z);
      const shade = (0.35 + 0.65 * Math.pow(t, 0.6)) * tone;
      col.push(shade, shade, shade, shade, shade, shade);
      // Normals mostly up (soft, like a lawn), a little toward the blade's face.
      const nx = Math.cos(dir) * 0.35;
      const ny = Math.sin(dir) * 0.35;
      nor.push(nx, ny, 1, nx, ny, 1);
    }
    for (let s = 0; s < segs; s++) {
      const i = start + s * 2;
      idx.push(i, i + 1, i + 3, i, i + 3, i + 2);
    }
  }
  const g = new THREE.BufferGeometry();
  g.setAttribute("position", new THREE.Float32BufferAttribute(pos, 3));
  g.setAttribute("normal", new THREE.Float32BufferAttribute(nor, 3));
  g.setAttribute("color", new THREE.Float32BufferAttribute(col, 3));
  g.setIndex(idx);
  g.normalizeNormals();
  return g;
}

/** A pine cone, about 80 mm long, lying on its side: a scaled, ridged spindle. */
export function coneGeometry(): THREE.BufferGeometry {
  const pts: THREE.Vector2[] = [];
  for (let k = 0; k <= 10; k++) {
    const t = k / 10;
    const r = 26 * Math.sin(Math.PI * Math.min(1, t * 1.1)) * (1 - 0.35 * t) + 1;
    pts.push(new THREE.Vector2(r * (1 + 0.12 * ((k % 2) - 0.5)), t * 80));
  }
  const g = new THREE.LatheGeometry(pts, 10);
  g.rotateX(Math.PI / 2);
  g.translate(0, -40, 18);
  const n = g.getAttribute("position").count;
  const c = new Float32Array(n * 3).fill(1);
  g.setAttribute("color", new THREE.BufferAttribute(c, 3));
  return g;
}

/** Scatters instances over triangles, `perM2` per square metre, denser within `near`
 * mm of `center` (and none beyond `far`), at most `budget`. */
export function scatter(
  positions: ArrayLike<number>,
  perM2: number,
  budget: number,
  center: THREE.Vector3,
  near: number,
  far: number,
  seed: number,
  place: (p: THREE.Vector3, r: () => number) => THREE.Matrix4 | null,
): THREE.Matrix4[] {
  const r = rng(seed);
  const tris: { a: number; area: number }[] = [];
  let total = 0;
  const a = new THREE.Vector3();
  const b = new THREE.Vector3();
  const c = new THREE.Vector3();
  const e1 = new THREE.Vector3();
  const e2 = new THREE.Vector3();
  const mid = new THREE.Vector3();
  for (let i = 0; i + 8 < positions.length; i += 9) {
    a.set(positions[i]!, positions[i + 1]!, positions[i + 2]!);
    b.set(positions[i + 3]!, positions[i + 4]!, positions[i + 5]!);
    c.set(positions[i + 6]!, positions[i + 7]!, positions[i + 8]!);
    e1.subVectors(b, a);
    e2.subVectors(c, a);
    const n = e1.clone().cross(e2);
    // Only surfaces facing up (not walls).
    if (n.z <= 0.6 * n.length()) continue;
    mid
      .addVectors(a, b)
      .add(c)
      .multiplyScalar(1 / 3);
    const d = Math.hypot(mid.x - center.x, mid.y - center.y);
    if (d > far) continue;
    // Full density near the model, thinning out to a quarter at `far`.
    const falloff =
      d < near ? 1 : Math.max(0.25, 1 - (0.75 * (d - near)) / Math.max(far - near, 1));
    const area = (n.length() / 2) * falloff;
    total += area;
    tris.push({ a: i, area });
  }
  if (!tris.length) return [];
  const want = Math.min(budget, Math.round((total / 1e6) * perM2));
  const out: THREE.Matrix4[] = [];
  const p = new THREE.Vector3();
  // Systematic sampling over the triangles' cumulative area.
  const step = total / Math.max(want, 1);
  let acc = r() * step;
  let k = 0;
  let run = 0;
  for (const t of tris) {
    run += t.area;
    while (acc < run && k < want) {
      const i = t.a;
      let u = r();
      let v = r();
      if (u + v > 1) {
        u = 1 - u;
        v = 1 - v;
      }
      a.set(positions[i]!, positions[i + 1]!, positions[i + 2]!);
      b.set(positions[i + 3]!, positions[i + 4]!, positions[i + 5]!);
      c.set(positions[i + 6]!, positions[i + 7]!, positions[i + 8]!);
      p.copy(a).addScaledVector(b.sub(a), u).addScaledVector(c.sub(a), v);
      const m = place(p, r);
      if (m) out.push(m);
      acc += step;
      k++;
    }
  }
  return out;
}

/** Grass clumps per square metre at full density, and the most drawn at once. */
export const CLUMPS_PER_M2 = 90;
export const LIVE_BUDGET = 45_000;
export const RENDER_BUDGET = 70_000;
/** The most clumps a field holds (it thins out beyond, in proportion). */
const FIELD_MAX = 900_000;

const clump = { geo: null as THREE.BufferGeometry | null };

function grassMaterial(color: [number, number, number], swaying: boolean) {
  const m = new THREE.MeshStandardMaterial({
    color: new THREE.Color(lin(color[0]) * 1.1, lin(color[1]) * 1.1, lin(color[2]) * 1.1),
    vertexColors: true,
    roughness: 0.7,
    side: THREE.DoubleSide,
  });
  if (swaying) {
    m.onBeforeCompile = (shader) => {
      shader.uniforms.uWind = wind.time;
      shader.vertexShader = shader.vertexShader
        .replace("#include <common>", "#include <common>\nuniform float uWind;")
        .replace(
          "#include <begin_vertex>",
          `#include <begin_vertex>
          {
            vec3 base = instanceMatrix[3].xyz;
            float s = sin(uWind * 1.7 + base.x * 0.0021 + base.y * 0.0013);
            transformed.xy += vec2(s, s * 0.5) * transformed.z * transformed.z * 0.12;
          }`,
        );
    };
    m.customProgramCacheKey = () => "grass-wind";
  }
  return m;
}

/** Where other surfaces cover the ground (paving, slabs, decks): a 250 mm grid of the
 * lowest upward-facing surface over each cell. A clump is covered when something lies
 * from just under it to 1.5 m over it. */
export function coverMask(blockers: ArrayLike<number>[], center: THREE.Vector3, far: number) {
  const cell = 250;
  const n = Math.ceil((2 * far) / cell);
  const x0 = center.x - far;
  const y0 = center.y - far;
  const low = new Float32Array(n * n).fill(Infinity);
  for (const p of blockers) {
    for (let i = 0; i + 8 < p.length; i += 9) {
      const [ax, ay, az, bx, by, bz, cx, cy, cz] = [0, 1, 2, 3, 4, 5, 6, 7, 8].map(
        (k) => p[i + k]!,
      );
      // Upward-facing only (floors and paving, not walls).
      const nz = (bx! - ax!) * (cy! - ay!) - (by! - ay!) * (cx! - ax!);
      const area = Math.abs(nz);
      if (area < 1) continue;
      const z = (az! + bz! + cz!) / 3;
      const i0 = Math.max(0, Math.floor((Math.min(ax!, bx!, cx!) - x0) / cell));
      const i1 = Math.min(n - 1, Math.floor((Math.max(ax!, bx!, cx!) - x0) / cell));
      const j0 = Math.max(0, Math.floor((Math.min(ay!, by!, cy!) - y0) / cell));
      const j1 = Math.min(n - 1, Math.floor((Math.max(ay!, by!, cy!) - y0) / cell));
      for (let ii = i0; ii <= i1; ii++)
        for (let jj = j0; jj <= j1; jj++) {
          const px = x0 + (ii + 0.5) * cell;
          const py = y0 + (jj + 0.5) * cell;
          // Barycentric inside test.
          const w0 = (bx! - px) * (cy! - py) - (by! - py) * (cx! - px);
          const w1 = (cx! - px) * (ay! - py) - (cy! - py) * (ax! - px);
          const w2 = (ax! - px) * (by! - py) - (ay! - py) * (bx! - px);
          const inside = (w0 >= 0 && w1 >= 0 && w2 >= 0) || (w0 <= 0 && w1 <= 0 && w2 <= 0);
          if (inside && z < low[jj * n + ii]!) low[jj * n + ii] = z;
        }
    }
  }
  return (x: number, y: number, z: number) => {
    const i = Math.floor((x - x0) / cell);
    const j = Math.floor((y - y0) / cell);
    if (i < 0 || j < 0 || i >= n || j >= n) return false;
    const b = low[j * n + i]!;
    return b > z - 100 && b < z + 1500;
  };
}

/** Candidate clumps over grass surfaces: (x, y, z, height, turn) per clump, bucketed in
 * 4 m cells for finding those near a point. */
export class GrassField {
  readonly data: Float32Array;
  readonly count: number;
  private cells = new Map<string, number[]>();
  static readonly CELL = 4000;

  constructor(
    surfaces: GrassSurface[],
    center: THREE.Vector3,
    radius: number,
    blockers: ArrayLike<number>[] = [],
  ) {
    const near = Math.max(radius * 1.2, 15_000);
    const far = Math.max(radius * 3, 45_000);
    const out: number[] = [];
    const covered = coverMask(blockers, center, far);
    const grassy = surfaces.filter((s) => s.grass);
    grassy.forEach((s, k) => {
      const g = s.grass!;
      // Taller grass grows in bigger clumps, fewer to the metre.
      const density = CLUMPS_PER_M2 * Math.min(1, Math.max(0.05, (60 / g.height) ** 2));
      scatter(
        s.positions,
        density,
        FIELD_MAX / grassy.length,
        center,
        near,
        far,
        17 + k,
        (p, r) => {
          const h = g.height * (1 - g.variation * 0.5 + g.variation * r());
          if (!covered(p.x, p.y, p.z)) out.push(p.x, p.y, p.z, h, r() * Math.PI * 2);
          return null;
        },
      );
    });
    this.data = new Float32Array(out);
    this.count = out.length / 5;
    for (let i = 0; i < this.count; i++) {
      const key = this.key(this.data[i * 5]!, this.data[i * 5 + 1]!);
      const list = this.cells.get(key);
      if (list) list.push(i);
      else this.cells.set(key, [i]);
    }
  }
  private key(x: number, y: number) {
    return `${Math.floor(x / GrassField.CELL)},${Math.floor(y / GrassField.CELL)}`;
  }
  /** The clumps within `r` of (x, y), nearest first, at most `max`. */
  near(x: number, y: number, r: number, max: number): number[] {
    const c = GrassField.CELL;
    const [i0, j0] = [Math.floor((x - r) / c), Math.floor((y - r) / c)];
    const [i1, j1] = [Math.floor((x + r) / c), Math.floor((y + r) / c)];
    const found: [number, number][] = [];
    for (let i = i0; i <= i1; i++)
      for (let j = j0; j <= j1; j++)
        for (const k of this.cells.get(`${i},${j}`) ?? []) {
          const d = Math.hypot(this.data[k * 5]! - x, this.data[k * 5 + 1]! - y);
          if (d < r) found.push([d, k]);
        }
    found.sort((a, b) => a[0] - b[0]);
    return found.slice(0, max).map((f) => f[1]);
  }
  /** Fills `mesh` with the clumps nearest (x, y): full size close in, shrinking to nothing
   * at the edge of what the budget reaches. */
  fill(mesh: THREE.InstancedMesh, x: number, y: number, budget: number) {
    const reach = Math.sqrt(budget / (CLUMPS_PER_M2 * Math.PI)) * 1000 * 1.25;
    const ids = this.near(x, y, reach, budget);
    const edge = ids.length
      ? Math.hypot(
          this.data[ids[ids.length - 1]! * 5]! - x,
          this.data[ids[ids.length - 1]! * 5 + 1]! - y,
        )
      : reach;
    const m = new THREE.Matrix4();
    const s = new THREE.Vector3();
    ids.forEach((k, n) => {
      const [px, py, pz, h, turn] = [0, 1, 2, 3, 4].map((o) => this.data[k * 5 + o]!);
      const d = Math.hypot(px! - x, py! - y);
      const fade = Math.min(1, Math.max(0, (edge - d) / (edge * 0.3)));
      m.makeRotationZ(turn!);
      m.scale(s.setScalar(h! * fade));
      m.setPosition(px!, py!, pz!);
      mesh.setMatrixAt(n, m);
    });
    mesh.count = ids.length;
    mesh.instanceMatrix.needsUpdate = true;
    mesh.computeBoundingSphere();
  }
}

/** Pine cones on pine straw (a sparse scatter over the whole area). */
function conePlacements(surfaces: GrassSurface[], center: THREE.Vector3, radius: number) {
  const near = Math.max(radius * 1.2, 15_000);
  const far = Math.max(radius * 3, 45_000);
  const cones: THREE.Matrix4[] = [];
  surfaces.forEach((s, k) => {
    if (!s.cones) return;
    cones.push(
      ...scatter(s.positions, 1.2, 4000, center, near, far, 91 + k, (p, r) => {
        const m = new THREE.Matrix4().makeRotationZ(r() * Math.PI * 2);
        const sc = 0.7 + r() * 0.5;
        m.scale(new THREE.Vector3(sc, sc, sc));
        m.setPosition(p.x, p.y, p.z);
        return m;
      }),
    );
  });
  return cones;
}

function coneMesh(ms: THREE.Matrix4[]) {
  const mat = new THREE.MeshStandardMaterial({
    color: new THREE.Color(lin(118), lin(78), lin(46)),
    roughness: 0.85,
    vertexColors: true,
  });
  const mesh = new THREE.InstancedMesh(coneGeometry(), mat, ms.length);
  ms.forEach((m, i) => mesh.setMatrixAt(i, m));
  mesh.instanceMatrix.needsUpdate = true;
  mesh.castShadow = true;
  mesh.receiveShadow = true;
  return mesh;
}

/** Groups surfaces by their grass's colour (one material each). */
function byColor(surfaces: GrassSurface[]) {
  const out = new Map<string, GrassSurface[]>();
  for (const s of surfaces) {
    const k = s.color.join(",");
    out.set(k, [...(out.get(k) ?? []), s]);
  }
  return [...out.values()];
}

/** Live-view grass: a field per grass colour that follows the camera (call `follow` as it
 * moves; it refills only after a few metres), and pine cones. */
export function grassGroup(
  surfaces: GrassSurface[],
  center: THREE.Vector3,
  radius: number,
  blockers: ArrayLike<number>[] = [],
): THREE.Group & { follow: (x: number, y: number) => void } {
  const g = new THREE.Group() as THREE.Group & { follow: (x: number, y: number) => void };
  g.name = "grass";
  clump.geo ??= clumpGeometry(12, 1.1, 7);
  const fields: { field: GrassField; mesh: THREE.InstancedMesh }[] = [];
  const groups = byColor(surfaces);
  for (const list of groups) {
    if (list.some((s) => s.grass)) {
      const field = new GrassField(list, center, radius, blockers);
      const budget = Math.floor(LIVE_BUDGET / groups.length);
      const mesh = new THREE.InstancedMesh(clump.geo, grassMaterial(list[0]!.color, true), budget);
      mesh.count = 0;
      mesh.receiveShadow = true;
      mesh.frustumCulled = false;
      g.add(mesh);
      fields.push({ field, mesh });
    }
    const cones = conePlacements(list, center, radius);
    if (cones.length) g.add(coneMesh(cones));
  }
  let last: [number, number] | null = null;
  g.follow = (x, y) => {
    if (last && Math.hypot(last[0] - x, last[1] - y) < 2500) return;
    last = [x, y];
    for (const f of fields) f.field.fill(f.mesh, x, y, f.mesh.instanceMatrix.count);
  };
  g.follow(center.x, center.y);
  return g;
}

/** Grass for the path tracer around the render's camera (x, y): merged, turned y-up. */
export function grassMeshesYUp(
  surfaces: GrassSurface[],
  center: THREE.Vector3,
  radius: number,
  camera: { x: number; y: number },
  blockers: ArrayLike<number>[] = [],
): THREE.Mesh[] {
  const toYUp = new THREE.Matrix4().makeRotationX(-Math.PI / 2);
  clump.geo ??= clumpGeometry(12, 1.1, 7);
  const out: THREE.Mesh[] = [];
  const bake = (geo: THREE.BufferGeometry, ms: THREE.Matrix4[]) => {
    if (!ms.length) return null;
    const pieces = ms.map((m) => geo.clone().applyMatrix4(toYUp.clone().multiply(m)));
    const merged = mergeGeometries(pieces, false);
    for (const p of pieces) p.dispose();
    return merged;
  };
  const groups = byColor(surfaces);
  for (const list of groups) {
    if (list.some((s) => s.grass)) {
      const field = new GrassField(list, center, radius, blockers);
      const budget = Math.floor(RENDER_BUDGET / groups.length);
      const tmp = new THREE.InstancedMesh(clump.geo, undefined, budget);
      field.fill(tmp, camera.x, camera.y, budget);
      const ms: THREE.Matrix4[] = [];
      for (let i = 0; i < tmp.count; i++) {
        const m = new THREE.Matrix4();
        tmp.getMatrixAt(i, m);
        ms.push(m);
      }
      tmp.dispose();
      const g = bake(clump.geo, ms);
      if (g) out.push(new THREE.Mesh(g, grassMaterial(list[0]!.color, false)));
    }
    const c = bake(coneGeometry(), conePlacements(list, center, radius));
    if (c)
      out.push(
        new THREE.Mesh(
          c,
          new THREE.MeshStandardMaterial({
            color: new THREE.Color(lin(118), lin(78), lin(46)),
            roughness: 0.85,
            vertexColors: true,
          }),
        ),
      );
  }
  return out;
}
