// Looks (ADR-120): what the leading visualization studios' images are made of, as presets
// of the renderer's atmosphere, light, lens and grade. Named for the look, not the studio;
// what each draws on is in the ADR.
// - Nordic Mist: grey overcast light, low mist, muted olive greens, lifted blacks, fine
//   grain: the Scandinavian painterly calm (MIR).
// - Golden Hour: a low sun through haze that glows toward it, teal shadows and warm
//   highlights, a touch of lens (The Boundary, Squint/Opera).
// - Blue Hour: just after sunset, the sky deep blue, the rooms lit warm behind the glass
//   (The Boundary's and Luxigon's dusk exteriors).
// - Editorial: crisp, bright and clean, as a magazine's architectural photograph
//   (Transparent House).
// - Cinematic: a 2.39:1 film frame, mist and haze in depth, strong contrast, shallow focus,
//   grain and fringing (Squint/Opera's film stills).
import type { Grade } from "./post";

export type LookId = "natural" | "plain" | "nordic" | "golden" | "blue" | "editorial" | "cinematic";

export interface Look {
  id: LookId;
  label: string;
  note: string;
  /** Sets the time to the golden hour (sun 7° up, before sunset) or the blue hour (4°
   * below, after it); none keeps the chosen time. */
  time?: "golden" | "blue";
  /** Interior and exterior lights on (the blue hour's lit rooms). */
  artificial?: boolean;
  /** Rooms behind the glass glowing, against the sky's light (ADR-101's interior glow). */
  interiorGlow?: number;
  /** The atmosphere: haze and mist (0–1 sliders, see [`hazeVisibility`], [`mistOf`]), the
   * sun's glow in the haze (0–1) and the cloud deck (0–1). */
  haze: number;
  mist: number;
  glow: number;
  overcast: number;
  /** Where the haze and mist begin (mm): the near ground stays clear. */
  fogStart?: number;
  /** Depth of field: how far the distance spreads (share of the image's width). */
  dof: number;
  tone: "neutral" | "filmic" | "contrast";
  /** Exposure, in stops from the scheme's. */
  ev: number;
  kelvin: number;
  /** White balance's green-magenta tint (above 0 greener). */
  tint?: number;
  saturation: number;
  contrast: number;
  bloom: number;
  vignette: number;
  grade: Grade;
  /** A film frame (2.39:1). */
  cinema?: boolean;
}

const none: Grade = {
  fade: 0,
  shadows: [0, 0, 0],
  highlights: [0, 0, 0],
  greens: 1,
  grain: 0,
  aberration: 0,
  sharpen: 0,
};

export const LOOKS: Look[] = [
  {
    id: "natural",
    label: "Natural",
    note: "D5's colour: true to the model, a touch richer",
    haze: 0.54,
    mist: 0,
    glow: 0.25,
    overcast: 0,
    dof: 0,
    tone: "neutral",
    ev: 0,
    kelvin: 6500,
    saturation: 1.14,
    contrast: 1.06,
    bloom: 0.025,
    vignette: 0.35,
    grade: none,
  },
  {
    id: "plain",
    label: "Plain",
    note: "No grade: the light as traced",
    haze: 0.54,
    mist: 0,
    glow: 0,
    overcast: 0,
    dof: 0,
    tone: "neutral",
    ev: 0,
    kelvin: 6500,
    saturation: 1,
    contrast: 1,
    bloom: 0.025,
    vignette: 0.35,
    grade: none,
  },
  {
    id: "nordic",
    label: "Nordic Mist",
    note: "Grey overcast light, low mist, muted greens, a matte finish",
    haze: 0.72,
    mist: 0.55,
    glow: 0.1,
    overcast: 0.85,
    fogStart: 12_000,
    dof: 0,
    tone: "filmic",
    ev: 0,
    kelvin: 6200,
    saturation: 0.86,
    contrast: 1.02,
    bloom: 0.04,
    vignette: 0.3,
    grade: {
      fade: 0.02,
      shadows: [-0.012, 0, 0.01],
      highlights: [0.012, 0.008, -0.006],
      greens: 0.68,
      grain: 0.022,
      aberration: 0,
      sharpen: 0,
    },
  },
  {
    id: "golden",
    label: "Golden Hour",
    note: "A low sun through glowing haze, teal shadows, warm light",
    time: "golden",
    haze: 0.64,
    mist: 0.3,
    glow: 0.6,
    overcast: 0,
    fogStart: 20_000,
    dof: 0.0025,
    tone: "neutral",
    ev: 0,
    kelvin: 7200,
    saturation: 1.06,
    contrast: 1.12,
    bloom: 0.06,
    vignette: 0.45,
    grade: {
      fade: 0.012,
      shadows: [-0.012, -0.004, 0.016],
      highlights: [0.02, 0.008, -0.015],
      greens: 0.92,
      grain: 0.015,
      aberration: 0.8,
      sharpen: 0.15,
    },
  },
  {
    id: "blue",
    label: "Blue Hour",
    note: "Just after sunset: a deep blue sky, the rooms lit warm",
    time: "blue",
    artificial: true,
    interiorGlow: 6,
    haze: 0.5,
    mist: 0,
    glow: 0,
    overcast: 0,
    dof: 0,
    tone: "neutral",
    ev: 1.5,
    kelvin: 5400,
    tint: 0.02,
    saturation: 1.08,
    contrast: 1.08,
    bloom: 0.07,
    vignette: 0.4,
    grade: {
      fade: 0.008,
      shadows: [-0.01, 0, 0.02],
      highlights: [0.02, 0.01, -0.01],
      greens: 0.9,
      grain: 0.012,
      aberration: 0,
      sharpen: 0.1,
    },
  },
  {
    id: "editorial",
    label: "Editorial",
    note: "Crisp, bright and clean, as a magazine's photograph",
    haze: 0.35,
    mist: 0,
    glow: 0.15,
    overcast: 0,
    dof: 0,
    tone: "neutral",
    ev: 0.17,
    kelvin: 6900,
    saturation: 1.06,
    contrast: 1.05,
    bloom: 0.02,
    vignette: 0.12,
    grade: { ...none, highlights: [0.004, 0.002, 0], sharpen: 0.2 },
  },
  {
    id: "cinematic",
    label: "Cinematic",
    note: "A 2.39:1 film frame, depth in mist, strong contrast, shallow focus",
    cinema: true,
    haze: 0.7,
    mist: 0.45,
    glow: 0.5,
    overcast: 0,
    fogStart: 25_000,
    dof: 0.006,
    tone: "contrast",
    ev: 0,
    kelvin: 6900,
    saturation: 1.04,
    contrast: 1.1,
    bloom: 0.07,
    vignette: 0.6,
    grade: {
      fade: 0.012,
      shadows: [-0.02, 0, 0.026],
      highlights: [0.022, 0.01, -0.018],
      greens: 0.85,
      grain: 0.03,
      aberration: 1.4,
      sharpen: 0,
    },
  },
];

export const lookOf = (id: LookId | undefined): Look => LOOKS.find((l) => l.id === id) ?? LOOKS[0]!;

/** Haze slider (0–1) to visibility (mm): none at 0, then 30 km down to 300 m. */
export function hazeVisibility(h: number): number {
  return h <= 0 ? 0 : 30_000_000 * Math.pow(0.01, Math.min(1, h));
}

/** Mist slider (0–1) to its visibility at the ground and its scale height (mm): none at 0,
 * then 3 km down to 80 m, 6 m to 26 m deep. */
export function mistOf(m: number): { visibility: number; height: number } {
  if (m <= 0) return { visibility: 0, height: 0 };
  const k = Math.min(1, m);
  return { visibility: 3_000_000 * Math.pow(0.08 / 3, k), height: 6000 + 20_000 * k };
}

/** "2.5 km", "800 m". */
export function distanceLabel(mm: number): string {
  const m = mm / 1000;
  return m >= 1000 ? `${(m / 1000).toFixed(m >= 10_000 ? 0 : 1)} km` : `${Math.round(m)} m`;
}

/** The look's grade at `strength` (0 none, 1 as designed): every departure from neutral
 * scaled, so a look can be dialled back. */
export function gradeAt(look: Look, strength: number) {
  const k = Math.max(0, Math.min(1, strength));
  const g = look.grade;
  const lerp = (a: number, b: number) => a + (b - a) * k;
  return {
    saturation: lerp(1, look.saturation),
    contrast: lerp(1, look.contrast),
    kelvin: lerp(6500, look.kelvin),
    tint: (look.tint ?? 0) * k,
    grade: {
      fade: g.fade * k,
      shadows: g.shadows.map((v) => v * k || 0) as Grade["shadows"],
      highlights: g.highlights.map((v) => v * k || 0) as Grade["highlights"],
      greens: lerp(1, g.greens),
      grain: g.grain * k,
      aberration: g.aberration * k,
      sharpen: g.sharpen * k,
    } satisfies Grade,
  };
}

/** The hour (to the quarter) of the afternoon when the sun stands at `altitude` degrees on
 * its way down: 7° for the golden hour, -4° for the blue. `sunAt` gives the altitude at an
 * hour; null if it never gets there between noon and 10 PM. */
export async function hourAtAltitude(
  sunAt: (hour: number) => Promise<number>,
  altitude: number,
): Promise<number | null> {
  let prevH = 12;
  let prev = await sunAt(12);
  for (let h = 12.5; h <= 22; h += 0.5) {
    const a = await sunAt(h);
    if (prev >= altitude && a < altitude) {
      const t = prevH + ((prev - altitude) / (prev - a)) * (h - prevH);
      return Math.round(t * 4) / 4;
    }
    prevH = h;
    prev = a;
  }
  return null;
}
