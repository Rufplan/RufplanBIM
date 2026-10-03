// Render backgrounds (ADR-028): real 360° panoramas from Poly Haven (CC0), bundled under
// public/backgrounds as a 3072 px JPEG to show behind the model and a 1k HDR to light it
// (V-Ray's Dome light), plus the physical sky and plain white.
import * as THREE from "three";
import { HDRLoader } from "three/examples/jsm/loaders/HDRLoader.js";
import type { SkyPreset } from "../bindings/SkyPreset";
import { ipc } from "../ipc";

/** A bundled background, or a Sky Library sky as `sky:<Poly Haven id>` (ADR-101). */
export type BackgroundId = string;

export interface Background {
  id: BackgroundId;
  label: string;
  /** Has a photo (and an HDR to light by). */
  photo: boolean;
  /** The ground plane's surface when the model has no topography. */
  ground: "grass" | "paving" | "neutral";
  /** The photo's own ground is projected onto the ground plane (V-Ray ground projection). */
  project?: boolean;
  source?: string;
  /** A Sky Library sky (ADR-101): downloaded on first use. */
  library?: SkyPreset;
}

/** A Sky Library sky as a background: photographed, lighting the scene by its HDR. */
export function skyBackground(s: SkyPreset): Background {
  return {
    id: `sky:${s.id}`,
    label: s.name,
    photo: true,
    ground: "grass",
    source: `${s.name} (${s.id}), Poly Haven, CC0`,
    library: s,
  };
}

let library: Promise<SkyPreset[]> | null = null;
/** The Sky Library's skies (ADR-101). */
export function skyLibrary(): Promise<SkyPreset[]> {
  if (!library) {
    library = ipc
      .skyLibrary()
      .then((s) => s ?? [])
      .catch(() => {
        library = null;
        return [];
      });
  }
  return library;
}

/** The library sky a background id names, if it is one. */
export function libraryId(id: BackgroundId): string | null {
  return id.startsWith("sky:") ? id.slice(4) : null;
}

export const BACKGROUNDS: Background[] = [
  // D5's default (ADR-065): the physical sky with soft clouds, matching the site's sun.
  { id: "physical", label: "D5 Sky (clouds, matches the sun)", photo: false, ground: "grass" },
  {
    id: "sky",
    label: "Sky",
    photo: true,
    ground: "grass",
    source: "Kloofendal 48d Partly Cloudy (Pure Sky), Poly Haven, CC0",
  },
  {
    id: "mountains",
    label: "Mountains",
    photo: true,
    project: true,
    ground: "grass",
    source: "Alps Field, Poly Haven, CC0",
  },
  {
    id: "grass",
    label: "Grass Plain",
    photo: true,
    project: true,
    ground: "grass",
    source: "JE Gray Park, Poly Haven, CC0",
  },
  {
    id: "city",
    label: "City",
    photo: true,
    project: true,
    ground: "paving",
    source: "Canary Wharf, Poly Haven, CC0",
  },
  { id: "white", label: "White", photo: false, ground: "neutral" },
];

const photos = new Map<BackgroundId, Promise<THREE.Texture>>();
const hdrs = new Map<BackgroundId, Promise<THREE.DataTexture>>();

/** The background photo, as an equirectangular texture. */
export function backgroundPhoto(id: BackgroundId): Promise<THREE.Texture> {
  let p = photos.get(id);
  const sky = libraryId(id);
  if (!p && sky) {
    // Poly Haven's full-size photo, at most 8K wide on the GPU (16K would take 512 MB).
    p = ipc.skyFile(sky, "photo").then(async (buf) => {
      const blob = new Blob([buf], { type: "image/jpeg" });
      const probe = await createImageBitmap(blob);
      const k = Math.min(1, 8192 / probe.width);
      const bitmap =
        k < 1
          ? await createImageBitmap(blob, {
              resizeWidth: Math.round(probe.width * k),
              resizeHeight: Math.round(probe.height * k),
              resizeQuality: "high",
              imageOrientation: "flipY",
            })
          : await createImageBitmap(blob, { imageOrientation: "flipY" });
      if (k < 1) probe.close();
      const t = new THREE.Texture(bitmap);
      t.flipY = false;
      t.mapping = THREE.EquirectangularReflectionMapping;
      t.colorSpace = THREE.SRGBColorSpace;
      t.generateMipmaps = true;
      t.minFilter = THREE.LinearMipmapLinearFilter;
      t.anisotropy = 8;
      t.needsUpdate = true;
      return t;
    });
    p.catch(() => photos.delete(id));
    photos.set(id, p);
  }
  if (!p) {
    p = new THREE.TextureLoader().loadAsync(`/backgrounds/${id}.jpg`).then((t) => {
      t.mapping = THREE.EquirectangularReflectionMapping;
      t.colorSpace = THREE.SRGBColorSpace;
      return t;
    });
    p.catch(() => photos.delete(id));
    photos.set(id, p);
  }
  return p;
}

/** The background's HDR, to light the scene by (Dome light). */
export function backgroundHdr(id: BackgroundId): Promise<THREE.DataTexture> {
  let p = hdrs.get(id);
  if (!p) {
    const loader = new HDRLoader();
    loader.setDataType(THREE.FloatType);
    const sky = libraryId(id);
    const url = sky
      ? ipc.skyFile(sky, "light").then((buf) => URL.createObjectURL(new Blob([buf])))
      : Promise.resolve(`/backgrounds/${id}.hdr`);
    p = url
      .then((u) => loader.loadAsync(u))
      .then((t) => {
        if (sky) fillGround(t);
        t.mapping = THREE.EquirectangularReflectionMapping;
        return t;
      });
    p.catch(() => hdrs.delete(id));
    hdrs.set(id, p);
  }
  return p;
}

/** Where the sun is in an HDR (its brightest region): its angle about the vertical in
 * three.js's equirect frame (atan2(z, x), y up) and its elevation, radians (ADR-101). */
export function hdrSun(t: THREE.DataTexture): { azimuth: number; elevation: number } | null {
  const img = t.image as { data: ArrayLike<number>; width: number; height: number };
  const { data, width: w, height: h } = img;
  if (!data || !w || !h) return null;
  let best = -1;
  let bi = 0;
  let bj = 0;
  // Only the upper half: the sun is above the horizon.
  for (let j = 0; j < h / 2; j++)
    for (let i = 0; i < w; i++) {
      const k = (j * w + i) * 4;
      const l = 0.2126 * data[k]! + 0.7152 * data[k + 1]! + 0.0722 * data[k + 2]!;
      if (l > best) {
        best = l;
        bi = i;
        bj = j;
      }
    }
  if (best <= 0) return null;
  // Row 0 is the top of the sky (v = 1).
  const u = (bi + 0.5) / w;
  const v = 1 - (bj + 0.5) / h;
  return { azimuth: (u - 0.5) * 2 * Math.PI, elevation: (v - 0.5) * Math.PI };
}

/** The background rotation (radians) that puts an HDR's sun at the site sun's compass
 * direction: `sunDir` z-up (x east, y north), as the renderer turns the dome. */
export function matchSunRotation(hdrAzimuth: number, sunDir: [number, number, number]): number {
  // y-up: (x, z, -y), so the site sun's angle is atan2(-y, x).
  const alpha = Math.atan2(-sunDir[1], sunDir[0]);
  // The path tracer turns the dome the opposite way to three.js's rotation matrix (checked
  // against the sun & sky render: the shadows fall the same way).
  const r = hdrAzimuth - alpha;
  return Math.atan2(Math.sin(r), Math.cos(r));
}

/** A copy of HDR `t` with its sun (the pixels far brighter than the sky) scaled by `k`
 * (ADR-102): below 1 the shadows open up against the sunlit faces, as a hazier day's do,
 * so whites needn't clip for the shade to read. */
export function scaleSun(t: THREE.DataTexture, k: number): THREE.DataTexture {
  const img = t.image as { data: Float32Array; width: number; height: number };
  const data = new Float32Array(img.data);
  // The sun: within a few percent of the brightest pixel (the sky itself is thousands of
  // times dimmer).
  let peak = 0;
  for (let i = 0; i < data.length; i += 4)
    peak = Math.max(peak, 0.2126 * data[i]! + 0.7152 * data[i + 1]! + 0.0722 * data[i + 2]!);
  const sky = peak / 30 / 20;
  for (let i = 0; i < data.length; i += 4) {
    const l = 0.2126 * data[i]! + 0.7152 * data[i + 1]! + 0.0722 * data[i + 2]!;
    if (l > sky * 30) {
      data[i] = data[i]! * k;
      data[i + 1] = data[i + 1]! * k;
      data[i + 2] = data[i + 2]! * k;
    }
  }
  const out = new THREE.DataTexture(data, img.width, img.height, THREE.RGBAFormat, t.type);
  out.mapping = t.mapping;
  // The loader flips HDRs on upload; a copy that didn't would put the sun underground.
  out.flipY = t.flipY;
  out.colorSpace = t.colorSpace;
  out.needsUpdate = true;
  return out;
}

/** The sky's mean radiance just above the horizon (linear), what aerial haze fades to. */
export function horizonColor(t: THREE.DataTexture): [number, number, number] {
  const img = t.image as { data: Float32Array; width: number; height: number };
  const { data, width: w, height: h } = img;
  const out: [number, number, number] = [0, 0, 0];
  let n = 0;
  // Rows a few degrees above the horizon (row 0 is the zenith).
  for (let j = Math.floor(h * 0.44); j < Math.floor(h * 0.49); j++)
    for (let i = 0; i < w; i += 4) {
      const k = (j * w + i) * 4;
      const l = 0.2126 * data[k]! + 0.7152 * data[k + 1]! + 0.0722 * data[k + 2]!;
      // Leave out the sun if it's low.
      if (l > 50) continue;
      out[0] += data[k]!;
      out[1] += data[k + 1]!;
      out[2] += data[k + 2]!;
      n++;
    }
  return out.map((v) => v / Math.max(n, 1)) as [number, number, number];
}

/** A pure sky is black below the horizon: nothing would bounce up from the ground, and
 * shade (the inside of a lawn, under eaves) goes black. Fill it as the physical sky does
 * (ADR-101): a grassy ground lit by this sky, its radiance albedo × irradiance / π. */
export function fillGround(
  t: THREE.DataTexture,
  albedo: [number, number, number] = [0.11, 0.15, 0.07],
) {
  const img = t.image as { data: Float32Array; width: number; height: number };
  const { data, width: w, height: h } = img;
  if (!data || !w || !h) return;
  // Horizontal irradiance from the upper half (row 0 is the zenith).
  let e = 0;
  const dTheta = Math.PI / h;
  const dPhi = (2 * Math.PI) / w;
  for (let j = 0; j < h / 2; j++) {
    const elev = Math.PI / 2 - (j + 0.5) * dTheta;
    const wgt = Math.sin(elev) * Math.cos(elev) * dTheta * dPhi;
    for (let i = 0; i < w; i++) {
      const k = (j * w + i) * 4;
      e += (0.2126 * data[k]! + 0.7152 * data[k + 1]! + 0.0722 * data[k + 2]!) * wgt;
    }
  }
  const g = albedo.map((a) => (a * e) / Math.PI);
  for (let j = Math.ceil(h / 2); j < h; j++)
    for (let i = 0; i < w; i++) {
      const k = (j * w + i) * 4;
      data[k] = g[0]!;
      data[k + 1] = g[1]!;
      data[k + 2] = g[2]!;
    }
  t.needsUpdate = true;
}
