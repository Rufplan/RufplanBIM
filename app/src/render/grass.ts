// 3D grass after D5 Render (ADR-064, ADR-065): real blades grown on every surface finished
// in a Grass-type material (Enscape's) and on grass painted with the Grass Brush (D5's), in
// D5's kinds — lawn, lush lawn, meadow with flowers, wild grass, dry grass, clover, tall
// grass — plus fallen pine cones on pine straw.
// - Each clump mixes blades of different heights, widths and hues (yellow-green to
//   blue-green, a few dry straw ones), darker at the root and lighter at the tip; meadows
//   carry wildflowers, clover its trefoil leaves and white heads, wild and tall grass their
//   seed heads.
// - The lawn has gentle patches across it (lighter, yellower, darker), as D5's do.
// - Grass grows densely near the camera: clumps are scattered over the surfaces once (a
//   field), and the nearest fill an instanced mesh, shrinking to nothing at its edge, as
//   the camera moves; the ground's texture carries on beyond. Renders take the field around
//   their camera, merged.
import * as THREE from "three";
import { mergeGeometries } from "three/examples/jsm/utils/BufferGeometryUtils.js";
import type { GrassKind } from "../bindings/GrassKind";
import { rgba, wind } from "./plants";

export interface GrassSettings {
  height: number;
  variation: number;
}

/** A surface to grow on: its triangles (9 floats each, mm, z up) and its grass. */
export interface GrassSurface {
  positions: ArrayLike<number>;
  grass: GrassSettings | null;
  /** The grass's base colour (sRGB 0-255). */
  color: [number, number, number];
  /** D5's kind (a Grass-type material is a lawn, or a meadow when tall). */
  kind?: GrassKind;
  /** Relative to the kind's own density. */
  density?: number;
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

/** How a kind's clumps grow (sizes in blade heights). */
interface Look {
  blades: number;
  width: number;
  lean: number;
  spread: number;
  /** Share of dry straw blades. */
  dry: number;
  /** Flowers per clump, and their colours (linear). */
  flowers: number;
  flowerColors: [number, number, number][];
  clover: number;
  seedHeads: number;
  /** Clumps per square metre at full density. */
  perM2: number;
  /** Lawns (ADR-101): wide leaves arching over, as D5's and Enscape's turf, rather than
   * upright spikes. */
  arch?: boolean;
}

const WHITE: [number, number, number] = [0.9, 0.9, 0.85];
const look = (o: Partial<Look> & Pick<Look, "blades" | "width" | "perM2">): Look => ({
  lean: 0.35,
  spread: 1,
  dry: 0.05,
  flowers: 0,
  flowerColors: [],
  clover: 0,
  seedHeads: 0,
  ...o,
});
const LOOKS: Record<GrassKind, Look> = {
  Lawn: look({ blades: 24, width: 0.07, lean: 0.8, spread: 0.9, perM2: 80, arch: true }),
  LushLawn: look({
    blades: 26,
    width: 0.085,
    lean: 1,
    spread: 0.95,
    // A few dry, yellowed blades among the green (ADR-102).
    dry: 0.045,
    perM2: 85,
    arch: true,
  }),
  Meadow: look({
    blades: 26,
    width: 0.036,
    lean: 0.5,
    spread: 0.55,
    dry: 0.1,
    flowers: 3.4,
    flowerColors: [WHITE, [0.95, 0.75, 0.1], [0.45, 0.28, 0.7], [0.85, 0.3, 0.45]],
    seedHeads: 1,
    perM2: 40,
  }),
  WildGrass: look({
    blades: 24,
    width: 0.03,
    lean: 0.6,
    spread: 0.5,
    dry: 0.2,
    flowers: 0.3,
    flowerColors: [WHITE, [0.95, 0.8, 0.15]],
    seedHeads: 3,
    perM2: 30,
  }),
  DryGrass: look({
    blades: 22,
    width: 0.034,
    lean: 0.55,
    spread: 0.5,
    dry: 0.65,
    seedHeads: 2,
    perM2: 34,
  }),
  Clover: look({
    blades: 10,
    width: 0.04,
    spread: 0.9,
    dry: 0.03,
    flowers: 0.6,
    flowerColors: [WHITE],
    clover: 8,
    perM2: 70,
  }),
  TallGrass: look({
    blades: 20,
    width: 0.022,
    lean: 0.4,
    spread: 0.4,
    dry: 0.15,
    seedHeads: 5,
    perM2: 18,
  }),
};

/** The kind a Grass-type material grows: a lawn, or a meadow when tall. */
export const kindFor = (s: GrassSurface): GrassKind =>
  s.kind ??
  ((s.grass?.height ?? 60) > 200 ? "Meadow" : (s.grass?.height ?? 60) >= 90 ? "LushLawn" : "Lawn");

/** A clump of one kind's grass in `color` (sRGB), about 1 unit tall (instances scale it
 * by the grass height). Its vertex colours are linear: each blade its own hue of the base
 * colour, darker at the root; flowers and seed heads their own colours. */
export function clumpGeometry(
  kind: GrassKind = "Lawn",
  color: [number, number, number] = [86, 124, 54],
  seed = 7,
  /** Far from the camera (ADR-101): fewer, wider leaves in fewer segments, covering the
   * same ground at about a fifth of the triangles. */
  lod = false,
): THREE.BufferGeometry {
  const baseLin = color.map(lin);
  const L = LOOKS[kind];
  const r = rng(seed + kind.length * 101);
  const pos: number[] = [];
  const col: number[] = [];
  const nor: number[] = [];
  const idx: number[] = [];
  const segs = 3;
  // Grass parts are tints of the base colour; flowers and seed heads their own.
  const push = (p: number[], n: number[], c: number[], own = false) => {
    pos.push(...p);
    nor.push(...n);
    col.push(...(own ? c : c.map((v, k) => v * baseLin[k]!)));
    return pos.length / 3 - 1;
  };
  // Straw relative to green: the base colour times this reads as dry.
  const straw = [2.6, 1.7, 1.9];
  if (L.arch)
    archedBlades(
      lod ? { ...L, blades: Math.round(L.blades * 0.38), width: L.width * 1.55 } : L,
      r,
      push,
      idx,
      pos,
      lod ? 2 : 4,
    );
  for (let b = 0; b < (L.arch ? 0 : L.blades); b++) {
    const a = r() * Math.PI * 2;
    const d = Math.sqrt(r()) * L.spread;
    const base = [Math.cos(a) * d, Math.sin(a) * d];
    const lean = r() * L.lean * 2 + 0.05;
    const dir = r() * Math.PI * 2;
    const h = 0.55 + r() * 0.45;
    const w = L.width * (0.7 + r() * 0.6);
    const twist = (r() - 0.5) * 1.2;
    // Hue: yellow-green to blue-green, and brightness.
    const hue = r();
    const bright = 0.88 + r() * 0.22;
    const tint = [
      (1.12 - 0.22 * hue) * bright,
      (1.02 + 0.02 * hue) * bright,
      (0.62 + 0.3 * hue) * bright,
    ];
    const dry = r() < L.dry;
    const start = pos.length / 3;
    for (let s = 0; s <= segs; s++) {
      const t = s / segs;
      // The blade bends over as it rises, and twists a little.
      const bend = lean * t * t * 0.5;
      const x = base[0]! + Math.cos(dir) * bend * h;
      const y = base[1]! + Math.sin(dir) * bend * h;
      const z = h * t * (1 - 0.15 * lean * t);
      const half = w * (1 - t * 0.9);
      const ang = dir + twist * t;
      const sx = -Math.sin(ang) * half;
      const sy = Math.cos(ang) * half;
      // Dark at the root (the thatch), lighter and yellower toward the tip.
      const shade = 0.8 + 0.25 * Math.pow(t, 0.55);
      const tip = [1 + 0.12 * t, 1 + 0.05 * t, 1 - 0.1 * t];
      const c = [0, 1, 2].map((k) => {
        const v = tint[k]! * tip[k]! * shade;
        return dry ? v * straw[k]! * 0.75 : v;
      });
      const nx = Math.cos(dir) * 0.35;
      const ny = Math.sin(dir) * 0.35;
      push([x - sx, y - sy, z], [nx, ny, 1], c);
      push([x + sx, y + sy, z], [nx, ny, 1], c);
    }
    for (let s = 0; s < segs; s++) {
      const i = start + s * 2;
      // Wound so the face points the way its normals do (up): a path tracer turns the
      // normals to the face, and blades lit from above went black (ADR-101).
      idx.push(i, i + 3, i + 1, i, i + 2, i + 3);
    }
  }
  // A little flat disc (flower head, clover leaflet), coloured `c` relative to the base
  // colour of the grass (flowers are divided by it so they show their own colour).
  const disc = (
    cx: number,
    cy: number,
    cz: number,
    rad: number,
    c: number[],
    tilt = 0,
    own = false,
  ) => {
    const centre = push([cx, cy, cz], [0, 0, 1], c, own);
    const n = 6;
    const ring: number[] = [];
    for (let k = 0; k < n; k++) {
      const t = (k / n) * Math.PI * 2;
      ring.push(
        push(
          [cx + Math.cos(t) * rad, cy + Math.sin(t) * rad, cz + Math.sin(t) * rad * tilt],
          [0, 0, 1],
          c,
          own,
        ),
      );
    }
    for (let k = 0; k < n; k++) idx.push(centre, ring[k]!, ring[(k + 1) % n]!);
  };
  const stalk = (x: number, y: number, top: number, c: number[]) => {
    const w = 0.008;
    const i = push([x - w, y, 0], [0, -1, 0.3], c);
    push([x + w, y, 0], [0, -1, 0.3], c);
    push([x + w * 0.5, y, top], [0, -1, 0.3], c);
    push([x - w * 0.5, y, top], [0, -1, 0.3], c);
    idx.push(i, i + 1, i + 2, i, i + 2, i + 3);
  };
  // Clover: trefoil leaves low in the sward.
  for (let k = 0; k < L.clover; k++) {
    const a = r() * Math.PI * 2;
    const d = Math.sqrt(r()) * L.spread;
    const [x, y] = [Math.cos(a) * d, Math.sin(a) * d];
    const z = 0.25 + r() * 0.3;
    const g = [0.75, 0.95, 0.8];
    for (let l = 0; l < 3; l++) {
      const t = a + (l * Math.PI * 2) / 3;
      disc(x + Math.cos(t) * 0.07, y + Math.sin(t) * 0.07, z, 0.075, g, 0.2);
    }
  }
  // Flowers on stalks a little above the grass, in their own colours.
  const flowers = Math.floor(L.flowers) + (r() < L.flowers % 1 ? 1 : 0);
  for (let k = 0; k < flowers; k++) {
    const a = r() * Math.PI * 2;
    const d = Math.sqrt(r()) * L.spread * 0.8;
    const [x, y] = [Math.cos(a) * d, Math.sin(a) * d];
    const top = kind === "Clover" ? 0.55 : 0.9 + r() * 0.35;
    stalk(x, y, top, [0.8, 0.95, 0.8]);
    // Each clump carries a mix of the colours, not one.
    const fc = L.flowerColors[k % L.flowerColors.length]!;
    disc(x, y, top, kind === "Clover" ? 0.08 : 0.07 + r() * 0.05, fc, 0, true);
  }
  // Seed heads: tan spikes at some blade tips.
  for (let k = 0; k < L.seedHeads; k++) {
    const a = r() * Math.PI * 2;
    const d = Math.sqrt(r()) * L.spread * 0.6;
    const [x, y] = [Math.cos(a) * d, Math.sin(a) * d];
    const top = 1.05 + r() * 0.3;
    stalk(x, y, top, [1.4, 1.2, 1.1]);
    const c = [0.42, 0.32, 0.17];
    const i = push([x - 0.02, y, top - 0.14], [0, -1, 0.4], c, true);
    push([x + 0.02, y, top - 0.14], [0, -1, 0.4], c, true);
    push([x, y, top + 0.02], [0, -1, 0.4], c, true);
    idx.push(i, i + 1, i + 2);
  }
  const g = new THREE.BufferGeometry();
  g.setAttribute("position", new THREE.Float32BufferAttribute(pos, 3));
  g.setAttribute("normal", new THREE.Float32BufferAttribute(nor, 3));
  g.setAttribute("color", rgba(col));
  g.setIndex(idx);
  g.normalizeNormals();
  return g;
}

/** A lawn's blades (ADR-101): flat leaves that rise and arch over (some nearly lying
 * down), tapering to a point, so a clump covers the ground as real turf does. Each leaf's
 * normals are its own surface's, fanned across its width as if folded along its rib: the
 * upper face catches the sun, the far half falls into shade. */
function archedBlades(
  L: Look,
  r: () => number,
  push: (p: number[], n: number[], c: number[], own?: boolean) => number,
  idx: number[],
  pos: number[],
  segs = 4,
) {
  const straw = [2.6, 1.7, 1.9];
  for (let b = 0; b < L.blades; b++) {
    const a = r() * Math.PI * 2;
    const d = Math.sqrt(r()) * L.spread;
    const base = [Math.cos(a) * d, Math.sin(a) * d];
    // Any way: leaves fanning out from every clump's middle read as rosettes.
    const dir = r() * Math.PI * 2;
    const len = 0.6 + r() * 0.55;
    const w = L.width * (0.75 + r() * 0.5);
    // Rises steeply, then arches: the elevation falls along the leaf.
    const rise = (62 + r() * 24) * (Math.PI / 180);
    const droop = (25 + r() * 75) * L.lean * (Math.PI / 180);
    const twist = (r() - 0.5) * 0.9;
    const hue = r();
    const bright = 0.86 + r() * 0.24;
    const tint = [
      (1.04 - 0.14 * hue) * bright,
      (1.02 + 0.03 * hue) * bright,
      (0.8 + 0.3 * hue) * bright,
    ];
    const dry = r() < L.dry;
    const [ux, uy] = [Math.cos(dir), Math.sin(dir)];
    let [x, y, z] = [base[0]!, base[1]!, 0];
    const start = pos.length / 3;
    for (let k = 0; k <= segs; k++) {
      const t = k / segs;
      const el = rise - droop * t * t;
      // Tangent, and the leaf's width across it (horizontal, turning with the twist).
      const tx = ux * Math.cos(el);
      const ty = uy * Math.cos(el);
      const tz = Math.sin(el);
      const ang = dir + Math.PI / 2 + twist * t;
      const [sx, sy] = [Math.cos(ang), Math.sin(ang)];
      // Normal: tangent × side, turned to face up.
      let nx = ty * 0 - tz * sy;
      let ny = tz * sx - tx * 0;
      let nz = tx * sy - ty * sx;
      if (nz < 0) [nx, ny, nz] = [-nx, -ny, -nz];
      const half = w * (1 - Math.pow(t, 1.6) * 0.92) * (t < 0.12 ? 0.6 + t * 3.3 : 1);
      // Thatch-dark only right at the root; light passes through real leaves.
      const shade = 0.9 + 0.18 * Math.pow(t, 0.6);
      const tip = [1 + 0.04 * t, 1 + 0.04 * t, 1 - 0.04 * t];
      const c = [0, 1, 2].map((q) => {
        const v = tint[q]! * tip[q]! * shade;
        return dry ? v * straw[q]! * 0.75 : v;
      });
      const fan = 0.55;
      push([x - sx * half, y - sy * half, z], [nx - sx * fan, ny - sy * fan, nz], c);
      push([x + sx * half, y + sy * half, z], [nx + sx * fan, ny + sy * fan, nz], c);
      const step = len / segs;
      x += tx * step;
      y += ty * step;
      z = Math.max(0.02, z + tz * step);
    }
    for (let k = 0; k < segs; k++) {
      const i = start + k * 2;
      // Wound so the face points the way its normals do (up): a path tracer turns the
      // normals to the face, and blades lit from above went black (ADR-101).
      idx.push(i, i + 3, i + 1, i, i + 2, i + 3);
    }
  }
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
  g.setAttribute("color", new THREE.BufferAttribute(new Float32Array(n * 4).fill(1), 4));
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
    // Skip a triangle only when all of it is beyond `far` (a big ground triangle's middle
    // can be far away while its corner is under the camera).
    const span = Math.max(mid.distanceTo(a), mid.distanceTo(b), mid.distanceTo(c));
    const d = Math.max(0, Math.hypot(mid.x - center.x, mid.y - center.y) - span);
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

/** Lawn clumps per square metre at full density (other kinds have their own), and the
 * most drawn at once. */
export const CLUMPS_PER_M2 = 70;
export const LIVE_BUDGET = 45_000;
// Renders merge every clump into the path tracer's scene: about 150 vertices each, four
// float attributes a vertex. More than this crowds out the rest of the scene on the GPU.
export const RENDER_BUDGET = 40_000;
/** The most clumps a field holds (it thins out beyond, in proportion). */
const FIELD_MAX = 900_000;

const clumps = new Map<string, THREE.BufferGeometry>();
const clumpOf = (k: GrassKind, color: [number, number, number]) => {
  const key = `${k}|${color.join(",")}`;
  let g = clumps.get(key);
  if (!g) {
    g = clumpGeometry(k, color);
    clumps.set(key, g);
  }
  return g;
};

function grassMaterial(swaying: boolean) {
  const m = new THREE.MeshStandardMaterial({
    vertexColors: true,
    roughness: 0.5,
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
            float s = sin(uWind * 1.7 + base.x * 0.0021 + base.y * 0.0013)
                    + 0.4 * sin(uWind * 3.1 + base.y * 0.004);
            transformed.xy += vec2(s, s * 0.5) * transformed.z * transformed.z * 0.1;
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
  const at = (i: number, j: number, z: number) => {
    if (i < 0 || j < 0 || i >= n || j >= n) return false;
    const b = low[j * n + i]!;
    return b > z - 100 && b < z + 1500;
  };
  return (x: number, y: number, z: number) => {
    // Sampled at a jittered point, so a bed's edge is ragged as turf grows into it, not
    // the grid's stair-steps (ADR-101).
    const jx = (patchNoise(x, y, 420, 31) - 0.5) * cell * 1.4;
    const jy = (patchNoise(x, y, 420, 37) - 0.5) * cell * 1.4;
    // Eroded (ADR-102): a clump is held back only well inside paving, so turf grows up to
    // and leans over the edges of stepping stones and the patio, as it does.
    const e = 25;
    const ok = (dx: number, dy: number) =>
      at(Math.floor((x + jx + dx - x0) / cell), Math.floor((y + jy + dy - y0) / cell), z);
    return ok(0, 0) && ok(e, 0) && ok(-e, 0) && ok(0, e) && ok(0, -e);
  };
}

/** Smooth value noise over the ground (mm), 0-1, for the lawn's patches. */
function patchNoise(x: number, y: number, cell: number, seed: number) {
  const h = (i: number, j: number) => {
    let n = (i * 374761393 + j * 668265263 + seed * 144269) | 0;
    n = (n ^ (n >>> 13)) * 1274126177;
    return ((n ^ (n >>> 16)) >>> 0) / 4294967296;
  };
  const fx = x / cell;
  const fy = y / cell;
  const i = Math.floor(fx);
  const j = Math.floor(fy);
  const [tx, ty] = [fx - i, fy - j];
  const s = (t: number) => t * t * (3 - 2 * t);
  const a = h(i, j) * (1 - s(tx)) + h(i + 1, j) * s(tx);
  const b = h(i, j + 1) * (1 - s(tx)) + h(i + 1, j + 1) * s(tx);
  return a * (1 - s(ty)) + b * s(ty);
}

/** A clump's colour for where it grows: gentle patches of lighter, yellower and darker
 * grass, as D5's lawns have. */
export function patchTint(x: number, y: number): [number, number, number] {
  const big = patchNoise(x, y, 9000, 1);
  const small = patchNoise(x, y, 2600, 2);
  // Gentle, as a kept lawn (ADR-101): the reference turf is even, not blotchy.
  const v = 0.93 + 0.1 * big + 0.05 * (small - 0.5);
  const yellow = Math.max(0, small - 0.62) * 0.9;
  return [v * (1 + yellow * 0.6), v * (1 + yellow * 0.2), v * (1 - yellow * 0.5)];
}

/** The parts of the triangles within `far` (plan) of `center`: big triangles are split
 * until their pieces are small, and the pieces wholly beyond `far` dropped, so a render's
 * field about its camera isn't spread over the whole site. */
export function clipNear(
  positions: ArrayLike<number>,
  center: THREE.Vector3,
  far: number,
): number[] {
  const out: number[] = [];
  const small = far / 6;
  const stack: number[][] = [];
  for (let i = 0; i + 8 < positions.length; i += 9)
    stack.push(Array.from({ length: 9 }, (_, k) => positions[i + k]!));
  while (stack.length) {
    const t = stack.pop()!;
    const mx = (t[0]! + t[3]! + t[6]!) / 3;
    const my = (t[1]! + t[4]! + t[7]!) / 3;
    let span = 0;
    for (let k = 0; k < 9; k += 3) span = Math.max(span, Math.hypot(t[k]! - mx, t[k + 1]! - my));
    if (Math.hypot(mx - center.x, my - center.y) - span > far) continue;
    if (span <= small || out.length > 400_000) {
      out.push(...t);
      continue;
    }
    // Split into four at the edge midpoints.
    const m = (a: number, b: number) => [0, 1, 2].map((k) => (t[a + k]! + t[b + k]!) / 2);
    const [a, b, c] = [t.slice(0, 3), t.slice(3, 6), t.slice(6, 9)];
    const [ab, bc, ca] = [m(0, 3), m(3, 6), m(6, 0)];
    stack.push(
      [...a, ...ab, ...ca],
      [...ab, ...b, ...bc],
      [...ca, ...bc, ...c],
      [...ab, ...bc, ...ca],
    );
  }
  return out;
}

/** Candidate clumps over grass surfaces: (x, y, z, height, turn) per clump, bucketed in
 * 4 m cells for finding those near a point. */
/** Clumps per m² a surface grows (ADR-102): its kind's, by its density and height. */
export function densityOf(s: GrassSurface): number {
  const g = s.grass ?? { height: 70, variation: 0 };
  const look = LOOKS[kindFor(s)];
  const typical = look.arch ? 100 : 300;
  return (
    look.perM2 *
    (s.density ?? 1) *
    // Lawns cover the ground: a clump's footprint goes as its height squared.
    (look.arch
      ? Math.min(2.2, Math.max(0.5, (typical / g.height) ** 2))
      : Math.min(1.4, Math.max(0.3, (typical / g.height) ** 0.5)))
  );
}

export class GrassField {
  readonly data: Float32Array;
  readonly count: number;
  /** Clumps per square metre at full density. */
  readonly perM2: number;
  private cells = new Map<string, number[]>();
  static readonly CELL = 4000;

  constructor(
    surfaces: GrassSurface[],
    center: THREE.Vector3,
    radius: number,
    blockers: ArrayLike<number>[] = [],
    /** A render's own reach about its camera (mm): full density out to it. */
    reach: number | null = null,
  ) {
    const near = reach ?? Math.max(radius * 1.2, 15_000);
    const far = reach ? reach * 1.05 : Math.max(radius * 3, 45_000);
    const out: number[] = [];
    const covered = coverMask(blockers, center, far);
    const grassy = surfaces.filter((s) => s.grass);
    let per = 0;
    grassy.forEach((s, k) => {
      const g = s.grass!;
      const density = densityOf(s);
      per = Math.max(per, density);
      scatter(
        reach ? clipNear(s.positions, center, far) : s.positions,
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
    this.perM2 = per || CLUMPS_PER_M2;
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
  near(
    x: number,
    y: number,
    r: number,
    max: number,
    dir: { x: number; y: number } | null = null,
  ): number[] {
    const c = GrassField.CELL;
    const [i0, j0] = [Math.floor((x - r) / c), Math.floor((y - r) / c)];
    const [i1, j1] = [Math.floor((x + r) / c), Math.floor((y + r) / c)];
    const found: [number, number][] = [];
    for (let i = i0; i <= i1; i++)
      for (let j = j0; j <= j1; j++)
        for (const k of this.cells.get(`${i},${j}`) ?? []) {
          const dx = this.data[k * 5]! - x;
          const dy = this.data[k * 5 + 1]! - y;
          const d = Math.hypot(dx, dy);
          // A render sees only what's in front of it (within about 65° of the view).
          if (d < r && (!dir || d < 1500 || dx * dir.x + dy * dir.y > 0.42 * d)) found.push([d, k]);
        }
    found.sort((a, b) => a[0] - b[0]);
    return found.slice(0, max).map((f) => f[1]);
  }
  /** Fills `mesh` with the clumps nearest (x, y): full size close in, shrinking to nothing
   * at the edge of what the budget reaches; each tinted for its patch of lawn. */
  fill(
    mesh: THREE.InstancedMesh,
    x: number,
    y: number,
    budget: number,
    dir: { x: number; y: number } | null = null,
  ) {
    // In a view cone the same budget reaches about twice as far.
    const reach =
      Math.sqrt(budget / (Math.max(this.perM2, 1) * Math.PI)) * 1000 * 1.4 * (dir ? 2.1 : 1);
    const ids = this.near(x, y, Math.max(reach, 5000), budget, dir);
    const edge = ids.length
      ? Math.hypot(
          this.data[ids[ids.length - 1]! * 5]! - x,
          this.data[ids[ids.length - 1]! * 5 + 1]! - y,
        )
      : reach;
    const m = new THREE.Matrix4();
    const s = new THREE.Vector3();
    const c = new THREE.Color();
    ids.forEach((k, n) => {
      const [px, py, pz, h, turn] = [0, 1, 2, 3, 4].map((o) => this.data[k * 5 + o]!);
      const d = Math.hypot(px! - x, py! - y);
      const fade = Math.min(1, Math.max(0, (edge - d) / (edge * 0.3)));
      m.makeRotationZ(turn!);
      m.scale(s.setScalar(h! * fade));
      m.setPosition(px!, py!, pz!);
      mesh.setMatrixAt(n, m);
      const t = patchTint(px!, py!);
      mesh.setColorAt(n, c.setRGB(t[0], t[1], t[2]));
    });
    mesh.count = ids.length;
    mesh.instanceMatrix.needsUpdate = true;
    if (mesh.instanceColor) mesh.instanceColor.needsUpdate = true;
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

/** Groups surfaces by their grass's kind and colour (one geometry and material each). */
function byLook(surfaces: GrassSurface[]) {
  const out = new Map<string, GrassSurface[]>();
  for (const s of surfaces) {
    const k = `${s.grass ? kindFor(s) : "none"}|${s.color.join(",")}`;
    out.set(k, [...(out.get(k) ?? []), s]);
  }
  return [...out.values()];
}

/** Live-view grass: a field per kind and colour that follows the camera (call `follow` as
 * it moves; it refills only after a few metres), and pine cones. */
export function grassGroup(
  surfaces: GrassSurface[],
  center: THREE.Vector3,
  radius: number,
  blockers: ArrayLike<number>[] = [],
): THREE.Group & { follow: (x: number, y: number) => void } {
  const g = new THREE.Group() as THREE.Group & { follow: (x: number, y: number) => void };
  g.name = "grass";
  const fields: { field: GrassField; mesh: THREE.InstancedMesh }[] = [];
  const groups = byLook(surfaces);
  const grassy = groups.filter((l) => l.some((s) => s.grass)).length || 1;
  for (const list of groups) {
    if (list.some((s) => s.grass)) {
      const field = new GrassField(list, center, radius, blockers);
      const budget = Math.floor(LIVE_BUDGET / grassy);
      const mesh = new THREE.InstancedMesh(
        clumpOf(kindFor(list[0]!), list[0]!.color),
        grassMaterial(true),
        budget,
      );
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

/** Grass for the path tracer around the render's camera (x, y): merged (each clump's
 * patch tint baked into its colours), turned y-up. */
export function grassMeshesYUp(
  surfaces: GrassSurface[],
  center: THREE.Vector3,
  radius: number,
  camera: { x: number; y: number },
  blockers: ArrayLike<number>[] = [],
  /** Where the render camera looks (plan, unit), so the grass goes in front of it. */
  look: { x: number; y: number } | null = null,
  total = RENDER_BUDGET,
): THREE.Mesh[] {
  const toYUp = new THREE.Matrix4().makeRotationX(-Math.PI / 2);
  const out: THREE.Mesh[] = [];
  const bake = (
    geo: THREE.BufferGeometry,
    ms: THREE.Matrix4[],
    tints: THREE.Color[] | null = null,
    /** Clumps beyond \`near\` mm of the camera take this lighter one (ADR-101). */
    far: { geo: THREE.BufferGeometry; near: number } | null = null,
  ) => {
    if (!ms.length) return null;
    const pieces = ms.map((m, i) => {
      const e = m.elements;
      const d = Math.hypot(e[12]! - camera.x, e[13]! - camera.y);
      const g = far && d > far.near ? far.geo : geo;
      const p = g.clone().applyMatrix4(toYUp.clone().multiply(m));
      const t = tints?.[i];
      if (t) {
        const c = p.getAttribute("color");
        for (let v = 0; v < c.count; v++)
          c.setXYZ(v, c.getX(v) * t.r, c.getY(v) * t.g, c.getZ(v) * t.b);
      }
      return p;
    });
    const merged = mergeGeometries(pieces, false);
    for (const p of pieces) p.dispose();
    return merged;
  };
  const groups = byLook(surfaces);
  const grassy = groups.filter((l) => l.some((s) => s.grass)).length || 1;
  for (const list of groups) {
    if (list.some((s) => s.grass)) {
      const budget = Math.floor(total / grassy);
      // A render seeds its field about the camera, out to what the budget reaches in
      // its view cone, so a large site doesn't thin the lawn in front of it.
      // At the lawn's own density, or the field thins out to fit (ADR-102).
      const per = Math.max(...list.filter((x) => x.grass).map(densityOf), CLUMPS_PER_M2);
      const reach = look ? Math.sqrt(budget / (per * Math.PI * 0.3)) * 1000 : null;
      const field = look
        ? new GrassField(list, new THREE.Vector3(camera.x, camera.y, 0), radius, blockers, reach)
        : new GrassField(list, center, radius, blockers);
      const geo = clumpOf(kindFor(list[0]!), list[0]!.color);
      const tmp = new THREE.InstancedMesh(geo, undefined, budget);
      field.fill(tmp, camera.x, camera.y, budget, look);
      const ms: THREE.Matrix4[] = [];
      const tints: THREE.Color[] = [];
      for (let i = 0; i < tmp.count; i++) {
        const m = new THREE.Matrix4();
        tmp.getMatrixAt(i, m);
        ms.push(m);
        const c = new THREE.Color();
        tmp.getColorAt(i, c);
        tints.push(c);
      }
      tmp.dispose();
      // Lawns: full leaves near the camera, the lighter clump beyond 16 m.
      const kind = kindFor(list[0]!);
      const far = LOOKS[kind].arch
        ? { geo: clumpGeometry(kind, list[0]!.color, 7, true), near: 16_000 }
        : null;
      const g = bake(geo, ms, tints, far);
      far?.geo.dispose();
      if (g) out.push(new THREE.Mesh(g, grassMaterial(false)));
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
