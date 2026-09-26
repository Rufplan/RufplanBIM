// Render backgrounds (ADR-028): real 360° panoramas from Poly Haven (CC0), bundled under
// public/backgrounds as a 3072 px JPEG to show behind the model and a 1k HDR to light it
// (V-Ray's Dome light), plus the physical sky and plain white.
import * as THREE from "three";
import { HDRLoader } from "three/examples/jsm/loaders/HDRLoader.js";

export type BackgroundId = "sky" | "mountains" | "grass" | "city" | "physical" | "white";

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
}

export const BACKGROUNDS: Background[] = [
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
  { id: "physical", label: "Physical Sky (matches the sun)", photo: false, ground: "grass" },
  { id: "white", label: "White", photo: false, ground: "neutral" },
];

const photos = new Map<BackgroundId, Promise<THREE.Texture>>();
const hdrs = new Map<BackgroundId, Promise<THREE.DataTexture>>();

/** The background photo, as an equirectangular texture. */
export function backgroundPhoto(id: BackgroundId): Promise<THREE.Texture> {
  let p = photos.get(id);
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
    p = loader.loadAsync(`/backgrounds/${id}.hdr`).then((t) => {
      t.mapping = THREE.EquirectangularReflectionMapping;
      return t;
    });
    p.catch(() => hdrs.delete(id));
    hdrs.set(id, p);
  }
  return p;
}
