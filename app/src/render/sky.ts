// A physically based daylight sky for renderings (ADR-028), in the manner of V-Ray's Sun &
// Sky: the Preetham sky model (clear sky luminance and colour from turbidity and the sun's
// height) plus the sun as a small, very bright disk. Lighting from the disk through the path
// tracer's importance sampling gives sun shadows with real, slightly soft edges.
import * as THREE from "three";
import { skyTable, sunTransmittance } from "./atmosphere";

/** Perez's sky distribution: luminance or chromaticity at zenith angle `theta`, angle
 * `gamma` from the sun. */
function perez(theta: number, gamma: number, c: number[]): number {
  const [A, B, C, D, E] = c as [number, number, number, number, number];
  const cosT = Math.max(Math.cos(theta), 0.01);
  return (1 + A * Math.exp(B / cosT)) * (1 + C * Math.exp(D * gamma) + E * Math.cos(gamma) ** 2);
}

/** The Preetham model's coefficients for turbidity `t`. */
function coefficients(t: number) {
  return {
    Y: [
      0.1787 * t - 1.463,
      -0.3554 * t + 0.4275,
      -0.0227 * t + 5.3251,
      0.1206 * t - 2.5771,
      -0.067 * t + 0.3703,
    ],
    x: [
      -0.0193 * t - 0.2592,
      -0.0665 * t + 0.0008,
      -0.0004 * t + 0.2125,
      -0.0641 * t - 0.8989,
      -0.0033 * t + 0.0452,
    ],
    y: [
      -0.0167 * t - 0.2608,
      -0.095 * t + 0.0092,
      -0.0079 * t + 0.2102,
      -0.0441 * t - 1.6537,
      -0.0109 * t + 0.0529,
    ],
  };
}

/** Zenith luminance (kcd/m²) and chromaticity for sun zenith angle `ts` (radians). */
function zenith(t: number, ts: number) {
  const chi = (4 / 9 - t / 120) * (Math.PI - 2 * ts);
  const Y = (4.0453 * t - 4.971) * Math.tan(chi) - 0.2155 * t + 2.4192;
  const t2 = ts * ts;
  const t3 = t2 * ts;
  const x =
    t * t * (0.00166 * t3 - 0.00375 * t2 + 0.00209 * ts) +
    t * (-0.02903 * t3 + 0.06377 * t2 - 0.03202 * ts + 0.00394) +
    (0.11693 * t3 - 0.21196 * t2 + 0.06052 * ts + 0.25886);
  const y =
    t * t * (0.00275 * t3 - 0.0061 * t2 + 0.00317 * ts) +
    t * (-0.04214 * t3 + 0.0897 * t2 - 0.04153 * ts + 0.00516) +
    (0.15346 * t3 - 0.26756 * t2 + 0.0667 * ts + 0.26688);
  return { Y: Math.max(Y, 0.05), x, y };
}

/** CIE xyY to linear sRGB. */
export function xyYToRgb(x: number, y: number, Y: number): [number, number, number] {
  const X = (x / y) * Y;
  const Z = ((1 - x - y) / y) * Y;
  return [
    Math.max(0, 3.2406 * X - 1.5372 * Y - 0.4986 * Z),
    Math.max(0, -0.9689 * X + 1.8758 * Y + 0.0415 * Z),
    Math.max(0, 0.0557 * X - 0.204 * Y + 1.057 * Z),
  ];
}

/** The sun's colour through the atmosphere: near white overhead, orange near the horizon
 * (the physical sky's transmittance, ADR-118), its brightest channel 1. */
export function sunColor(altitudeDeg: number): [number, number, number] {
  const c = sunTransmittance(Math.max(altitudeDeg, 0.5));
  const k = Math.max(c[0], c[1], c[2], 1e-9);
  return [c[0] / k, c[1] / k, c[2] / k];
}

export interface SkyOptions {
  /** Unit vector toward the sun, three.js world (y up). */
  sunDir: THREE.Vector3;
  altitude: number;
  /** Haze: 2 very clear, 3 clear (V-Ray's default), 6 hazy. */
  turbidity?: number;
  /** Sun disk radius, degrees: the real sun is 0.27°; like V-Ray's size multiplier, a
   * larger sun (1° by default) gives softer shadow edges and far less noise. */
  sunRadius?: number;
  /** Sunlit : sky-lit horizontal illuminance at high sun (about 6 on a clear day). */
  sunToSky?: number;
  width?: number;
  height?: number;
  /** Ground seen below the horizon (linear albedo). */
  ground?: [number, number, number];
  /** D5's default sky (ADR-065): soft fair-weather cumulus covering this share of the sky
   * (0 clear, about 0.35 a D5 day), lit by the sun. */
  clouds?: number;
}

/** Periodic-free value noise for the clouds, 0-1. */
function cloudNoise(x: number, y: number): number {
  const h = (i: number, j: number) => {
    let n = (i * 374761393 + j * 668265263) | 0;
    n = (n ^ (n >>> 13)) * 1274126177;
    return ((n ^ (n >>> 16)) >>> 0) / 4294967296;
  };
  let sum = 0;
  let amp = 0.5;
  let norm = 0;
  let fx = x;
  let fy = y;
  for (let o = 0; o < 5; o++) {
    const i = Math.floor(fx);
    const j = Math.floor(fy);
    const tx = fx - i;
    const ty = fy - j;
    const sx = tx * tx * (3 - 2 * tx);
    const sy = ty * ty * (3 - 2 * ty);
    const a = h(i, j) * (1 - sx) + h(i + 1, j) * sx;
    const b = h(i, j + 1) * (1 - sx) + h(i + 1, j + 1) * sx;
    sum += amp * (a * (1 - sy) + b * sy);
    norm += amp;
    amp *= 0.5;
    // Rotate each octave a little so the lattice doesn't show.
    const nx = fx * 1.6 - fy * 1.2 + 17.3;
    fy = fx * 1.2 + fy * 1.6 + 3.1;
    fx = nx;
  }
  return sum / norm;
}

const smooth = (a: number, b: number, x: number) => {
  const t = Math.min(1, Math.max(0, (x - a) / (b - a)));
  return t * t * (3 - 2 * t);
};

/** An equirectangular HDR map of the sky and sun, scaled so the zenith is about 1. */
export function physicalSky(o: SkyOptions): THREE.DataTexture {
  const w = o.width ?? 2048;
  const h = o.height ?? 1024;
  // A cloudy D5 sky is a clearer, deeper blue between its clouds.
  const t = o.turbidity ?? (o.clouds ? 2.4 : 3);
  const sun = o.sunDir.clone().normalize();
  const ts = Math.min(Math.acos(Math.max(-1, Math.min(1, sun.y))), (89.5 * Math.PI) / 180);
  const co = coefficients(t);
  const z = zenith(t, ts);
  const zY = z.Y / perez(0, ts, co.Y);
  const zx = z.x / perez(0, ts, co.x);
  const zy = z.y / perez(0, ts, co.y);
  // Night and twilight dim the whole sky.
  const day = Math.max(0.02, Math.min(1, (o.altitude + 4) / 14));
  const scale = day / Math.max(z.Y, 0.5);
  const data = new Float32Array(w * h * 4);
  const horizon: [number, number, number] = [0, 0, 0];
  let hn = 0;
  const dir = new THREE.Vector3();
  let irradiance = 0;
  // The clear sky's colour and shape from the physical atmosphere (ADR-118), scaled to the
  // Preetham sky's light on the ground, so exposures and the sun's balance hold: blue
  // overhead, white-gold toward a low sun, the haze bright around it.
  const sunElev = Math.asin(Math.max(-0.12, Math.min(1, sun.y)));
  const sunAz = Math.atan2(sun.z, sun.x);
  const atmo = skyTable(sunElev);
  const preethamY = (elev: number, gamma: number) =>
    zY * perez(Math.PI / 2 - elev, gamma, co.Y) * scale;
  let pIrr = 0;
  let aIrr = 0;
  for (let j = 0; j < 32; j++) {
    const elev = ((j + 0.5) / 32) * (Math.PI / 2);
    for (let i = 0; i < 64; i++) {
      const phi = ((i + 0.5) / 64 - 0.5) * Math.PI * 2;
      dir.set(Math.cos(elev) * Math.cos(phi), Math.sin(elev), Math.cos(elev) * Math.sin(phi));
      const gamma = Math.acos(Math.max(-1, Math.min(1, dir.dot(sun))));
      const w = Math.sin(elev) * Math.cos(elev);
      pIrr += preethamY(elev, gamma) * w;
      const a = atmo(elev, phi - sunAz);
      aIrr += (0.2126 * a[0] + 0.7152 * a[1] + 0.0722 * a[2]) * w;
    }
  }
  const atmoScale = aIrr > 0 ? pIrr / aIrr : 0;
  const cover = Math.min(0.9, Math.max(0, o.clouds ?? 0));
  const cloudLum = z.Y * scale * (2.4 + 1.6 * Math.max(0, sun.y));
  const tint = sunColor(Math.max(o.altitude, 1));
  for (let j = 0; j < h; j++) {
    // three's equirect: row 0 straight down, the last row straight up.
    const elev = ((j + 0.5) / h - 0.5) * Math.PI;
    const cosE = Math.cos(elev);
    for (let i = 0; i < w; i++) {
      const phi = ((i + 0.5) / w - 0.5) * Math.PI * 2;
      dir.set(cosE * Math.cos(phi), Math.sin(elev), cosE * Math.sin(phi));
      const k = (j * w + i) * 4;
      if (elev < 0) continue;
      const theta = Math.PI / 2 - elev;
      const gamma = Math.acos(Math.max(-1, Math.min(1, dir.dot(sun))));
      let r: number;
      let g: number;
      let b: number;
      if (atmoScale > 0) {
        [r, g, b] = atmo(elev, phi - sunAz).map((v) => v * atmoScale) as [number, number, number];
      } else {
        const Y = zY * perez(theta, gamma, co.Y) * scale;
        [r, g, b] = xyYToRgb(zx * perez(theta, gamma, co.x), zy * perez(theta, gamma, co.y), Y);
      }
      let lum = 0.2126 * r + 0.7152 * g + 0.0722 * b;
      if (cover > 0 && dir.y > 0.01) {
        // A cloud deck overhead, seen in perspective: features shrink toward the horizon.
        const kk = 1 / Math.max(dir.y, 0.04);
        const n = cloudNoise(dir.x * kk * 0.85 + 40, dir.z * kk * 0.85 + 40);
        const d = smooth(1 - cover - 0.04, 1 - cover + 0.12, n) * smooth(0.02, 0.16, dir.y) * 0.97;
        if (d > 0) {
          const toward = Math.max(0, dir.dot(sun));
          // Bright sunlit tops, silver edges toward the sun, greyer thick undersides.
          const silver = 1 + 1.4 * Math.pow(toward, 10) * (1 - d);
          const shade = 1 - 0.42 * d * d;
          const cl = cloudLum * silver * shade;
          const c = [
            cl * (0.78 + 0.22 * tint[0]),
            cl * (0.78 + 0.22 * tint[1]),
            cl * (0.78 + 0.22 * tint[2]),
          ];
          r = r * (1 - d) + c[0]! * d;
          g = g * (1 - d) + c[1]! * d;
          b = b * (1 - d) + c[2]! * d;
          lum = 0.2126 * r + 0.7152 * g + 0.0722 * b;
        }
      }
      data[k] = r;
      data[k + 1] = g;
      data[k + 2] = b;
      data[k + 3] = 1;
      // Horizontal illuminance from the sky: L cosθ dΩ.
      const dOmega = cosE * (Math.PI / h) * ((2 * Math.PI) / w);
      irradiance += lum * Math.sin(elev) * dOmega;
      if (elev < 0.05) {
        horizon[0] += r;
        horizon[1] += g;
        horizon[2] += b;
        hn++;
      }
    }
  }
  // Below the horizon: the ground, lit by the sky and the sun.
  const albedo = o.ground ?? [0.18, 0.17, 0.15];
  const avg = horizon.map((v) => v / Math.max(hn, 1)) as [number, number, number];
  const lum = 0.2126 * avg[0] + 0.7152 * avg[1] + 0.0722 * avg[2];
  const groundLight =
    Math.max(lum, 0.05) * (1 + Math.max(0, Math.sin((o.altitude * Math.PI) / 180)) * 2);
  for (let j = 0; j < h / 2; j++) {
    for (let i = 0; i < w; i++) {
      const k = (j * w + i) * 4;
      data[k] = albedo[0] * groundLight;
      data[k + 1] = albedo[1] * groundLight;
      data[k + 2] = albedo[2] * groundLight;
      data[k + 3] = 1;
    }
  }
  // The sun disk, bright enough that sunlit : skylit is `sunToSky` at this sky.
  if (o.altitude > -1) {
    // At least 1.5 pixels across, so a coarse map still has a sun (same total light).
    const radius = Math.max(((o.sunRadius ?? 1) * Math.PI) / 180, (1.5 * Math.PI) / h);
    const omega = Math.PI * radius * radius;
    const col = sunColor(o.altitude);
    const ratio = (o.sunToSky ?? 6) * Math.min(1, Math.max(0.15, (o.altitude + 2) / 20));
    const L = (ratio * irradiance) / omega;
    // Only the rows near the sun.
    const sunElev = Math.asin(sun.y);
    const j0 = Math.max(0, Math.floor(((sunElev - radius * 2) / Math.PI + 0.5) * h));
    const j1 = Math.min(h - 1, Math.ceil(((sunElev + radius * 2) / Math.PI + 0.5) * h));
    for (let j = j0; j <= j1; j++) {
      const elev = ((j + 0.5) / h - 0.5) * Math.PI;
      const cosE = Math.cos(elev);
      for (let i = 0; i < w; i++) {
        const phi = ((i + 0.5) / w - 0.5) * Math.PI * 2;
        dir.set(cosE * Math.cos(phi), Math.sin(elev), cosE * Math.sin(phi));
        const a = Math.acos(Math.max(-1, Math.min(1, dir.dot(sun))));
        if (a > radius) continue;
        // Limb darkening.
        const mu = Math.sqrt(Math.max(0, 1 - (a / radius) ** 2));
        const f = L * (0.4 + 0.6 * mu);
        const k = (j * w + i) * 4;
        data[k] = data[k]! + col[0] * f;
        data[k + 1] = data[k + 1]! + col[1] * f;
        data[k + 2] = data[k + 2]! + col[2] * f;
      }
    }
  }
  const tex = new THREE.DataTexture(data, w, h, THREE.RGBAFormat, THREE.FloatType);
  tex.mapping = THREE.EquirectangularReflectionMapping;
  tex.minFilter = THREE.LinearFilter;
  tex.magFilter = THREE.LinearFilter;
  tex.generateMipmaps = false;
  tex.needsUpdate = true;
  return tex;
}

/** Linear albedo of the plain grounds: lawn, paving and a neutral grey. */
export const GROUND_ALBEDO: Record<"grass" | "paving" | "neutral", [number, number, number]> = {
  grass: [0.04, 0.058, 0.02],
  paving: [0.2, 0.19, 0.18],
  neutral: [0.24, 0.24, 0.22],
};
