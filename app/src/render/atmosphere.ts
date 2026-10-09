// A physically based clear sky (ADR-118): sunlight scattered by the air (Rayleigh), haze
// (Mie) and absorbed by ozone, traced through a curved atmosphere (Nishita et al. 1993, with
// Hillaire 2020's constants, as Blender's sky and Unreal's Sky Atmosphere use). The sky's
// colour and the sun's colour come from one model, so a low sun is orange and its sky
// blue-to-gold, as V-Ray's and Corona's improved skies give. No data tables: the sky depends
// only on a direction's height and its angle from the sun, so a small table of those is
// traced once per sky and looked up for every pixel.

/** Scattering and absorption per metre at sea level, and scale heights. */
const RAYLEIGH: [number, number, number] = [5.802e-6, 13.558e-6, 33.1e-6];
const RAYLEIGH_H = 8000;
const MIE_SCATTER = 3.996e-6;
const MIE_EXTINCT = 3.996e-6 + 4.4e-6;
const MIE_H = 1200;
const MIE_G = 0.8;
const OZONE: [number, number, number] = [0.65e-6, 1.881e-6, 0.085e-6];
const EARTH = 6_360_000;
const TOP = 6_460_000;
/** The viewer's height above the ground (m). */
const VIEWER = 200;

type RGB = [number, number, number];

/** Air, haze and ozone densities at height `h` (m). */
function density(h: number): [number, number, number] {
  return [
    Math.exp(-h / RAYLEIGH_H),
    Math.exp(-h / MIE_H),
    Math.max(0, 1 - Math.abs(h - 25_000) / 15_000),
  ];
}

/** Where a ray from radius-vector (0, r0) along unit (dx, dy) leaves the sphere of radius
 * `R` (the far root), or -1. */
function exitDistance(r0: number, dy: number, R: number): number {
  const b = r0 * dy;
  const c = r0 * r0 - R * R;
  const disc = b * b - c;
  return disc < 0 ? -1 : -b + Math.sqrt(disc);
}

/** Whether the ray hits the ground before `limit`. */
function hitsGround(r0: number, dy: number): boolean {
  const b = r0 * dy;
  const c = r0 * r0 - EARTH * EARTH;
  const disc = b * b - c;
  return disc >= 0 && -b - Math.sqrt(disc) > 0;
}

/** Transmittance along a ray from height `h` at cosine `mu` from the zenith to space. */
function transmittance(h: number, mu: number, steps = 12): RGB {
  const r0 = EARTH + h;
  if (hitsGround(r0, mu)) return [0, 0, 0];
  const len = exitDistance(r0, mu, TOP);
  const ds = len / steps;
  let dr = 0;
  let dm = 0;
  let dz = 0;
  for (let i = 0; i < steps; i++) {
    const t = (i + 0.5) * ds;
    const y = r0 + mu * t;
    const x = Math.sqrt(Math.max(0, 1 - mu * mu)) * t;
    const hh = Math.hypot(x, y) - EARTH;
    const [a, m, o] = density(hh);
    dr += a * ds;
    dm += m * ds;
    dz += o * ds;
  }
  return [0, 1, 2].map((c) =>
    Math.exp(-(RAYLEIGH[c]! * dr + MIE_EXTINCT * dm + OZONE[c]! * dz)),
  ) as RGB;
}

/** The sun's light reaching the ground, relative to above the atmosphere, at `altitude`
 * degrees: white overhead, orange low. */
export function sunTransmittance(altitudeDeg: number): RGB {
  return transmittance(VIEWER, Math.sin((Math.max(altitudeDeg, -1) * Math.PI) / 180), 24);
}

/** Single-scattered sky radiance (relative to the sun's irradiance above the atmosphere)
 * seen at elevation `elev` (radians, ≥ 0) and azimuth `az` from the sun's, the sun at
 * elevation `sunElev`. */
export function skyRadiance(elev: number, az: number, sunElev: number, steps = 32): RGB {
  const r0 = EARTH + VIEWER;
  const mu = Math.sin(elev);
  const len = exitDistance(r0, mu, TOP);
  // The view and the sun, the viewer's up being y.
  const d = [Math.cos(elev) * Math.cos(az), mu, Math.cos(elev) * Math.sin(az)];
  const s = [Math.cos(sunElev), Math.sin(sunElev), 0];
  const cosG = d[0]! * s[0]! + d[1]! * s[1]!;
  // Phase functions: Rayleigh's, and Cornette-Shanks for haze.
  const pr = (3 / (16 * Math.PI)) * (1 + cosG * cosG);
  const g2 = MIE_G * MIE_G;
  const pm =
    ((3 / (8 * Math.PI)) * ((1 - g2) * (1 + cosG * cosG))) /
    ((2 + g2) * Math.pow(1 + g2 - 2 * MIE_G * cosG, 1.5));
  const out: RGB = [0, 0, 0];
  let dr = 0;
  let dm = 0;
  let dz = 0;
  // Steps grow away from the viewer, where the air thins.
  for (let i = 0; i < steps; i++) {
    const t0 = len * Math.pow(i / steps, 2);
    const t1 = len * Math.pow((i + 1) / steps, 2);
    const ds = t1 - t0;
    const t = (t0 + t1) / 2;
    // The point on the ray, from the earth's centre; the sun's zenith cosine there.
    const p = [d[0]! * t, r0 + d[1]! * t, d[2]! * t];
    const r = Math.hypot(p[0]!, p[1]!, p[2]!);
    const h = r - EARTH;
    const [a, m, o] = density(h);
    dr += a * ds;
    dm += m * ds;
    dz += o * ds;
    const muSun = (p[0]! * s[0]! + p[1]! * s[1]!) / r;
    const ts = transmittance(h, Math.max(-1, Math.min(1, muSun)), 6);
    for (let c = 0; c < 3; c++) {
      const tv = Math.exp(-(RAYLEIGH[c]! * dr + MIE_EXTINCT * dm + OZONE[c]! * dz));
      out[c] = out[c]! + tv * ts[c]! * (RAYLEIGH[c]! * a * pr + MIE_SCATTER * m * pm) * ds;
    }
  }
  return out;
}

/** A table of sky radiance over elevation (0–90°, `ne` rows, denser near the horizon) and
 * angle from the sun's azimuth (0–180°, `na` columns), for a sun at `sunElev` (radians). */
export function skyTable(sunElev: number, ne = 48, na = 64) {
  const data = new Float32Array(ne * na * 3);
  for (let j = 0; j < ne; j++) {
    const elev = (Math.PI / 2) * Math.pow(j / (ne - 1), 2);
    for (let i = 0; i < na; i++) {
      const az = (Math.PI * i) / (na - 1);
      const L = skyRadiance(elev, az, sunElev);
      data.set(L, (j * na + i) * 3);
    }
  }
  /** Radiance looking at elevation `elev` and azimuth `az` from the sun's (radians). */
  return (elev: number, az: number): RGB => {
    const fe = Math.sqrt(Math.max(0, Math.min(1, elev / (Math.PI / 2)))) * (ne - 1);
    let a = Math.abs(az) % (2 * Math.PI);
    if (a > Math.PI) a = 2 * Math.PI - a;
    const fa = (a / Math.PI) * (na - 1);
    const j0 = Math.min(ne - 2, Math.floor(fe));
    const i0 = Math.min(na - 2, Math.floor(fa));
    const tj = fe - j0;
    const ti = fa - i0;
    const at = (j: number, i: number, c: number) => data[(j * na + i) * 3 + c]!;
    return [0, 1, 2].map(
      (c) =>
        (at(j0, i0, c) * (1 - ti) + at(j0, i0 + 1, c) * ti) * (1 - tj) +
        (at(j0 + 1, i0, c) * (1 - ti) + at(j0 + 1, i0 + 1, c) * ti) * tj,
    ) as RGB;
  };
}
