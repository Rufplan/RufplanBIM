// Physical materials for renderings (ADR-029): a material's V-Ray-style appearance as a
// three.js MeshPhysicalMaterial, its textures (Poly Haven photo sets or procedural ones)
// and real-world box mapping, as V-Ray's "real-world scale" box UVW map.
import * as THREE from "three";
import type { Appearance } from "../bindings/Appearance";
import type { TextureMap } from "../bindings/TextureMap";

/** Loads a photo texture map's JPEG bytes (in the app, from Rust's cache). */
export type TextureLoader = (set: string, map: TextureMap) => Promise<ArrayBuffer | Blob>;

export interface TextureSet {
  map: THREE.Texture | null;
  normalMap: THREE.Texture | null;
  roughnessMap: THREE.Texture | null;
  /** Tile height over width (1 for square tiles). */
  aspect?: number;
}

const photoSets = new Map<string, Promise<TextureSet>>();

async function imageTexture(bytes: ArrayBuffer | Blob, srgb: boolean): Promise<THREE.Texture> {
  const blob = bytes instanceof Blob ? bytes : new Blob([bytes], { type: "image/jpeg" });
  const url = URL.createObjectURL(blob);
  try {
    const img = new Image();
    img.src = url;
    await img.decode();
    const t = new THREE.Texture(img);
    t.wrapS = THREE.RepeatWrapping;
    t.wrapT = THREE.RepeatWrapping;
    t.colorSpace = srgb ? THREE.SRGBColorSpace : THREE.NoColorSpace;
    t.anisotropy = 8;
    t.needsUpdate = true;
    return t;
  } finally {
    // The decoded image stays with the texture.
    setTimeout(() => URL.revokeObjectURL(url), 1000);
  }
}

/** A photo texture set (colour, normal, roughness), loaded once per set. */
export function photoSet(set: string, load: TextureLoader): Promise<TextureSet> {
  let p = photoSets.get(set);
  if (!p) {
    p = Promise.all([
      load(set, "color").then((b) => imageTexture(b, true)),
      load(set, "normal").then((b) => imageTexture(b, false)),
      load(set, "roughness").then((b) => imageTexture(b, false)),
    ]).then(([map, normalMap, roughnessMap]) => ({ map, normalMap, roughnessMap }));
    p.catch(() => photoSets.delete(set));
    photoSets.set(set, p);
  }
  return p;
}

// ---------------------------------------------------------------- procedural textures

const SIZE = 1024;

function hash(x: number, y: number, seed = 0): number {
  const s = Math.sin(x * 127.1 + y * 311.7 + seed * 74.7) * 43758.5453;
  return s - Math.floor(s);
}

/** Tileable value noise at `cell` px. */
function noise(x: number, y: number, cell: number, seed = 0): number {
  const n = SIZE / cell;
  const gx = Math.floor(x / cell);
  const gy = Math.floor(y / cell);
  const fx = x / cell - gx;
  const fy = y / cell - gy;
  const v = (a: number, b: number) => hash(((a % n) + n) % n, ((b % n) + n) % n, seed);
  const sx = fx * fx * (3 - 2 * fx);
  const sy = fy * fy * (3 - 2 * fy);
  const top = v(gx, gy) * (1 - sx) + v(gx + 1, gy) * sx;
  const bot = v(gx, gy + 1) * (1 - sx) + v(gx + 1, gy + 1) * sx;
  return top * (1 - sy) + bot * sy;
}

function fbm(x: number, y: number, octaves: number, cell: number, seed = 0): number {
  let a = 0.5;
  let s = 0;
  let c = cell;
  for (let i = 0; i < octaves; i++) {
    s += a * noise(x, y, Math.max(c, 2), seed + i);
    a *= 0.5;
    c /= 2;
  }
  return s;
}

interface Field {
  /** Tile height (px) when not square (hexagons), width being SIZE. */
  height_px?: number;
  /** Albedo multiplier (0–1.2) per channel, applied to the material colour. */
  albedo: (x: number, y: number) => [number, number, number];
  /** Height 0–1 for the bump. */
  height: (x: number, y: number) => number;
  /** Roughness multiplier. */
  rough: (x: number, y: number) => number;
}

/** Grout lines of a running-bond or grid pattern: distance to the nearest joint (px). */
function joints(x: number, y: number, w: number, h: number, bond: boolean): number {
  const row = Math.floor(y / h);
  const ox = bond && row % 2 ? w / 2 : 0;
  const lx = (((x + ox) % w) + w) % w;
  const ly = ((y % h) + h) % h;
  return Math.min(lx, w - lx, ly, h - ly);
}

/** Hexagon radius (px) and the tile height that holds a whole number of hex rows. */
// 2" hexagons on a 300 mm tile: 12 across, 7 whole rows high.
const HEX_R = SIZE / 12;
const HEX_ROWS = 7;
export const HEX_H = Math.round(HEX_ROWS * Math.sqrt(3) * HEX_R);

/** Distance to the nearest edge of a flat-topped hexagon grid of radius `r` (px). */
function hexJoint(x: number, y: number, r: number): number {
  const sq3 = Math.sqrt(3);
  // Nearest centre: axial coordinates, rounded in cube space.
  const q = ((2 / 3) * x) / r;
  const rr = ((-1 / 3) * x + (sq3 / 3) * y) / r;
  let cx = Math.round(q);
  let cz = Math.round(rr);
  const cy = Math.round(-q - rr);
  const dx = Math.abs(cx - q);
  const dy = Math.abs(cy - (-q - rr));
  const dz = Math.abs(cz - rr);
  if (dx > dy && dx > dz) cx = -cy - cz;
  else if (dy <= dz) cz = -cx - cy;
  const px = r * 1.5 * cx;
  const py = r * sq3 * (cz + cx / 2);
  const vx = x - px;
  const vy = y - py;
  // Flat-topped hexagon: edge normals at 90°, 30° and -30°.
  const apothem = (r * sq3) / 2;
  const d = Math.max(
    Math.abs(vy),
    Math.abs(vx * (sq3 / 2) + vy * 0.5),
    Math.abs(vx * (sq3 / 2) - vy * 0.5),
  );
  return apothem - d;
}

function veins(x: number, y: number, amount: number, sharp: number, seed: number): number {
  const w = fbm(x, y, 5, 256, seed) * 6;
  const v = Math.abs(Math.sin((x + y * 0.6) / 90 + w * 3));
  return Math.pow(1 - v, sharp) * amount;
}

const FIELDS: Record<string, () => Field> = {
  subway: () => ({
    // 3" x 6" tile: the 610 mm tile shows 8 x 4 bricks per repeat.
    albedo: (x, y) => {
      const d = joints(x, y, SIZE / 4, SIZE / 8, true);
      const g = d < 4 ? 0.7 : 1 - 0.03 * noise(x, y, 64);
      return [g, g, g];
    },
    height: (x, y) => Math.min(1, joints(x, y, SIZE / 4, SIZE / 8, true) / 10),
    rough: (x, y) => (joints(x, y, SIZE / 4, SIZE / 8, true) < 5 ? 8 : 1),
  }),
  hex: () => ({
    height_px: HEX_H,
    albedo: (x, y) => {
      const g = hexJoint(x, y, HEX_R) < 3 ? 0.78 : 1 - 0.04 * noise(x, y, 32);
      return [g, g, g];
    },
    height: (x, y) => Math.min(1, hexJoint(x, y, HEX_R) / 6),
    rough: (x, y) => (hexJoint(x, y, HEX_R) < 3 ? 1.3 : 1),
  }),
  cmu: () => ({
    albedo: (x, y) => {
      const d = joints(x, y, SIZE / 3, SIZE / 6, true);
      const g =
        (d < 6 ? 0.86 : 1) *
        (0.9 + 0.2 * fbm(x, y, 4, 64, 3)) *
        (0.95 + 0.1 * hash(Math.floor(x / 4), Math.floor(y / 4)));
      return [g, g, g];
    },
    height: (x, y) =>
      Math.min(1, joints(x, y, SIZE / 3, SIZE / 6, true) / 8) * (0.8 + 0.2 * hash(x, y)),
    rough: () => 1,
  }),
  act: () => ({
    albedo: (x, y) => {
      const d = joints(x, y, SIZE, SIZE, false);
      const g = d < 8 ? 1.02 : 0.93 + 0.08 * hash(Math.floor(x / 3), Math.floor(y / 3));
      return [g, g, g];
    },
    height: (x, y) => {
      const d = joints(x, y, SIZE, SIZE, false);
      return d < 8 ? 1 : 0.4 + 0.2 * hash(Math.floor(x / 3), Math.floor(y / 3), 9);
    },
    rough: (x, y) => (joints(x, y, SIZE, SIZE, false) < 8 ? 0.4 : 1),
  }),
  seam: () => ({
    // 18" standing seam: one rib at the tile's edge, a faint pencil rib between.
    albedo: (x) => {
      const g = 0.98 + 0.03 * noise(x, 0, 256);
      return [g, g, g];
    },
    height: (x) => {
      const d = Math.min(x, SIZE - x);
      return d < 14 ? 1 : Math.abs(x - SIZE / 2) < 4 ? 0.55 : 0.45;
    },
    rough: () => 1,
  }),
  carpet: () => ({
    albedo: (x, y) => {
      const g = 0.75 + 0.35 * hash(x, y) * 0.6 + 0.2 * noise(x, y, 8);
      return [g, g, g];
    },
    height: (x, y) =>
      0.5 +
      0.5 * Math.sin(x / 2.2) * Math.sin(y / 2.2) * hash(Math.floor(x / 3), Math.floor(y / 3)),
    rough: () => 1,
  }),
  "carpet-pattern": () => ({
    albedo: (x, y) => {
      // Two-tone lattice with a medallion: hotel broadloom.
      const cx = (x % 256) - 128;
      const cy = (y % 256) - 128;
      const lattice = Math.abs(Math.abs(cx) - Math.abs(cy)) < 6 ? 1 : 0;
      const ring = Math.abs(Math.hypot(cx, cy) - 70) < 8 ? 1 : 0;
      const pile = 0.85 + 0.25 * hash(x, y) * 0.5;
      if (lattice) return [1.4 * pile, 1.25 * pile, 0.9 * pile];
      if (ring) return [0.55 * pile, 0.62 * pile, 0.7 * pile];
      return [pile, pile, pile];
    },
    height: (x, y) => 0.5 + 0.4 * hash(x, y),
    rough: () => 1,
  }),
  carrara: () => ({
    albedo: (x, y) => {
      const v = veins(x, y, 0.35, 8, 1) + veins(x * 1.7, y * 1.3, 0.15, 14, 7);
      const cloud = 0.04 * fbm(x, y, 4, 128, 5);
      const g = 1 - v - cloud;
      return [g, g, g * 1.01];
    },
    height: () => 0.5,
    rough: (x, y) => 1 + veins(x, y, 0.3, 8, 1),
  }),
  calacatta: () => ({
    albedo: (x, y) => {
      const v = veins(x, y, 0.6, 5, 11);
      const thin = veins(x * 1.9, y * 1.4, 0.2, 16, 3);
      const g = 1 - thin - 0.02 * fbm(x, y, 4, 128, 2);
      // Warm gold-grey in the bold veins.
      return [g - v * 0.55, g - v * 0.62, g - v * 0.78];
    },
    height: () => 0.5,
    rough: () => 1,
  }),
  granite: () => ({
    albedo: (x, y) => {
      const s = hash(Math.floor(x / 2), Math.floor(y / 2), 4);
      const g = s > 0.93 ? 3.2 : s > 0.8 ? 1.8 : 0.8 + 0.3 * noise(x, y, 16);
      return [g, g, g];
    },
    height: () => 0.5,
    rough: (x, y) => (hash(Math.floor(x / 2), Math.floor(y / 2), 4) > 0.93 ? 0.6 : 1),
  }),
  quartz: () => ({
    albedo: (x, y) => {
      const s = hash(Math.floor(x / 2), Math.floor(y / 2), 8);
      const g = s > 0.985 ? 0.55 : s > 0.95 ? 0.85 : 1 - 0.02 * noise(x, y, 64);
      return [g, g, g];
    },
    height: () => 0.5,
    rough: () => 1,
  }),
  brushed: () => ({
    albedo: (x, y) => {
      const g = 0.93 + 0.07 * noise(x * 0.02, y, 1.5, 6) + 0.02 * hash(0, y);
      return [g, g, g];
    },
    height: (x, y) => 0.5 + 0.08 * noise(x * 0.02, y, 1.5, 6),
    rough: (x, y) => 0.8 + 0.4 * noise(x * 0.02, y, 1.5, 9),
  }),
  grass: () => ({
    albedo: (x, y) => {
      const b = 0.7 + 0.5 * hash(x, y) * 0.8 + 0.2 * fbm(x, y, 3, 128, 2);
      const dry = fbm(x, y, 3, 256, 5);
      return [b * (1 + dry * 0.35), b, b * 0.9];
    },
    height: (x, y) => hash(x, y),
    rough: () => 1,
  }),
};

const procSets = new Map<string, TextureSet>();

function canvasTexture(data: Uint8ClampedArray, srgb: boolean, h = SIZE): THREE.Texture {
  const c = document.createElement("canvas");
  c.width = SIZE;
  c.height = h;
  c.getContext("2d")!.putImageData(new ImageData(new Uint8ClampedArray(data), SIZE, h), 0, 0);
  const t = new THREE.CanvasTexture(c);
  t.wrapS = THREE.RepeatWrapping;
  t.wrapT = THREE.RepeatWrapping;
  t.colorSpace = srgb ? THREE.SRGBColorSpace : THREE.NoColorSpace;
  t.anisotropy = 8;
  return t;
}

/** A procedural texture set, coloured by the material's colour. */
export function proceduralSet(kind: string, color: [number, number, number]): TextureSet {
  const key = `${kind}:${color.join(",")}`;
  const hit = procSets.get(key);
  if (hit) return hit;
  const f = (FIELDS[kind] ?? FIELDS.quartz!)();
  const H = f.height_px ?? SIZE;
  const alb = new Uint8ClampedArray(SIZE * H * 4);
  const hgt = new Float32Array(SIZE * H);
  const rgh = new Uint8ClampedArray(SIZE * H * 4);
  // Albedo in linear space: modulate the colour's linear value, then back to sRGB.
  const lin = color.map((c) => Math.pow(c / 255, 2.2)) as [number, number, number];
  for (let y = 0; y < H; y++) {
    for (let x = 0; x < SIZE; x++) {
      const i = y * SIZE + x;
      const [a, b, c] = f.albedo(x, y);
      alb[i * 4] = 255 * Math.pow(Math.min(1, lin[0] * a), 1 / 2.2);
      alb[i * 4 + 1] = 255 * Math.pow(Math.min(1, lin[1] * b), 1 / 2.2);
      alb[i * 4 + 2] = 255 * Math.pow(Math.min(1, lin[2] * c), 1 / 2.2);
      alb[i * 4 + 3] = 255;
      hgt[i] = f.height(x, y);
      // Roughness map: a multiplier around mid-grey (the material's roughness scales it).
      const r = Math.max(0, Math.min(255, 128 * f.rough(x, y)));
      rgh[i * 4] = r;
      rgh[i * 4 + 1] = r;
      rgh[i * 4 + 2] = r;
      rgh[i * 4 + 3] = 255;
    }
  }
  // Normals from the height field (wrapping, so the tile repeats cleanly).
  const nrm = new Uint8ClampedArray(SIZE * H * 4);
  const h = (x: number, y: number) => hgt[((y + H) % H) * SIZE + ((x + SIZE) % SIZE)]!;
  const k = 3;
  for (let y = 0; y < H; y++) {
    for (let x = 0; x < SIZE; x++) {
      const dx = (h(x + 1, y) - h(x - 1, y)) * k;
      const dy = (h(x, y + 1) - h(x, y - 1)) * k;
      const l = Math.hypot(dx, dy, 1);
      const i = (y * SIZE + x) * 4;
      nrm[i] = 128 + (127 * -dx) / l;
      nrm[i + 1] = 128 + (127 * dy) / l;
      nrm[i + 2] = 128 + 127 / l;
      nrm[i + 3] = 255;
    }
  }
  const set = {
    map: canvasTexture(alb, true, H),
    normalMap: canvasTexture(nrm, false, H),
    roughnessMap: canvasTexture(rgh, false, H),
    aspect: H / SIZE,
  };
  procSets.set(key, set);
  return set;
}

// ---------------------------------------------------------------- materials

/** Textures for an appearance: its photo set, procedural pattern, or none. */
export async function texturesFor(
  a: Appearance,
  color: [number, number, number],
  load: TextureLoader,
): Promise<TextureSet | null> {
  const t = a.texture;
  if (!t) return null;
  if (t.startsWith("proc:")) return proceduralSet(t.slice(5), color);
  return photoSet(t, load);
}

const srgb = (c: [number, number, number]) =>
  new THREE.Color().setRGB(c[0] / 255, c[1] / 255, c[2] / 255, THREE.SRGBColorSpace);

/** The physical material for an appearance, in the manner of V-Ray's VRayMtl. */
export function physicalMaterial(
  a: Appearance,
  color: [number, number, number],
  tex: TextureSet | null,
): THREE.MeshPhysicalMaterial {
  const photo = !!a.texture && !a.texture.startsWith("proc:");
  const m = new THREE.MeshPhysicalMaterial({
    color: photo && a.textureColor ? srgb(a.tint) : srgb(color),
    roughness: a.roughness,
    metalness: a.metalness,
    specularIntensity: a.reflection,
    ior: a.ior,
    transmission: a.refraction,
    // Thin architectural glass: no refraction offset, nothing trapped in the pane.
    thickness: 0,
    clearcoat: a.coat,
    clearcoatRoughness: 0.04,
    sheen: a.sheen,
    sheenColor: srgb(color),
    sheenRoughness: 0.6,
    side: THREE.DoubleSide,
  });
  if (tex) {
    if (tex.map && (a.textureColor || !photo)) m.map = tex.map;
    if (tex.normalMap) {
      m.normalMap = tex.normalMap;
      m.normalScale.set(a.bump, a.bump);
    }
    if (tex.roughnessMap) {
      m.roughnessMap = tex.roughnessMap;
      // Maps average around mid-grey: scale so the material's glossiness still reads.
      m.roughness = Math.min(1, a.roughness * (photo ? 1.4 : 2));
    }
  }
  if (a.refraction > 0) Object.assign(m, { castShadow: false });
  return m;
}

/** Real-world box mapping: each triangle projects along its dominant axis, tiling every
 * `scale` mm, with tangents along the projected u axis (for the normal map). Positions
 * are y-up. */
export function boxUv(geo: THREE.BufferGeometry, scale: number, aspect = 1) {
  const pos = geo.getAttribute("position");
  const n = pos.count;
  const uv = new Float32Array(n * 2);
  const tan = new Float32Array(n * 4);
  const a = new THREE.Vector3();
  const b = new THREE.Vector3();
  const c = new THREE.Vector3();
  const nrm = new THREE.Vector3();
  for (let i = 0; i + 2 < n; i += 3) {
    a.fromBufferAttribute(pos, i);
    b.fromBufferAttribute(pos, i + 1);
    c.fromBufferAttribute(pos, i + 2);
    nrm.subVectors(b, a).cross(c.clone().sub(a)).normalize();
    const ax = Math.abs(nrm.x);
    const ay = Math.abs(nrm.y);
    const az = Math.abs(nrm.z);
    for (let k = 0; k < 3; k++) {
      const p = [a, b, c][k]!;
      let u: number;
      let v: number;
      let t: [number, number, number];
      if (ay >= ax && ay >= az) {
        // Floors and ceilings: plan x across, north up.
        u = p.x;
        v = -p.z;
        t = [1, 0, 0];
      } else if (ax >= az) {
        // Walls facing east or west: along z, height up.
        u = -p.z * Math.sign(nrm.x || 1);
        v = p.y;
        t = [0, 0, -Math.sign(nrm.x || 1)];
      } else {
        u = p.x * Math.sign(nrm.z || 1);
        v = p.y;
        t = [Math.sign(nrm.z || 1), 0, 0];
      }
      uv[(i + k) * 2] = u / scale;
      uv[(i + k) * 2 + 1] = v / (scale * aspect);
      tan.set([t[0], t[1], t[2], 1], (i + k) * 4);
    }
  }
  geo.setAttribute("uv", new THREE.BufferAttribute(uv, 2));
  geo.setAttribute("tangent", new THREE.BufferAttribute(tan, 4));
}
