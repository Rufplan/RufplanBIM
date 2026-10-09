// Atmosphere in a rendering (ADR-120): the haze and low mist that MIR's, The Boundary's and
// Squint/Opera's images are built on, as their Photoshop z-depth passes and V-Ray's and
// Corona's volume fog give, but exact rather than painted:
// - haze, the same everywhere: distant trees and hills fade toward the horizon's colour
//   (aerial perspective), by Koschmieder's law (3.912 / visibility per metre);
// - mist, exponential with height (Quilez's height fog): thick at the ground, gone a few
//   tens of metres up; its depth along any ray is integrated in closed form, so the sky
//   low down and the far lawn sink into it and the building rises out of it;
// - a glow toward the sun (forward scattering, Henyey-Greenstein g 0.4), warm with the
//   sun's colour, for back-lit golden hours.
// The fog's share per pixel comes from a rasterized depth pass of the scene, averaged over
// jittered draws (and the lens, with depth of field) so edges are antialiased as the trace
// is; the colour is added in the finishing pass (post.ts), in linear light.
import * as THREE from "three";
import { lensOf, renderJittered } from "./denoise";

/** Fog for a rendering, in scene units (mm), y up. */
export interface FogSettings {
  /** Haze: the distance (mm) at which a dark object fades to 2% contrast; 0 for none. */
  visibility: number;
  /** Mist: its visibility (mm) at the ground, 0 for none, and its scale height (mm). */
  mistVisibility: number;
  mistHeight: number;
  /** The ground's height (mm), where the mist is thickest. */
  ground: number;
  /** Where the fog begins (mm from the camera), as V-Ray's fog and the studios' z-depth
   * curves do: the near ground stays clear and the depth builds behind it. */
  start?: number;
  /** The haze's colour (linear radiance, before exposure): the horizon's. */
  color: [number, number, number];
  /** Unit vector toward the sun (y up) and how much the haze glows around it (0–1). */
  sunDir: [number, number, number] | null;
  glow: number;
  /** The sun's colour, for the glow. */
  sunColor: [number, number, number];
}

/** Koschmieder: extinction per unit length for a visibility. */
export const extinction = (visibility: number) => (visibility > 0 ? 3.912 / visibility : 0);

/** The mist's optical depth along a ray from height `y0` going `dy` (unit direction's y)
 * for `length` (Infinity for the sky): ∫ σ exp(-(y - ground) / H) dt in closed form. */
export function mistDepth(f: FogSettings, y0: number, dy: number, length: number): number {
  const s = extinction(f.mistVisibility);
  if (s <= 0 || f.mistHeight <= 0) return 0;
  const H = f.mistHeight;
  const a = Math.exp(-(y0 - f.ground) / H);
  if (!Number.isFinite(length)) return dy > 1e-3 ? (s * a * H) / dy : 50;
  if (Math.abs(dy) < 1e-5) return s * a * length;
  const b = Math.exp(-(y0 + dy * length - f.ground) / H);
  return (s * H * (a - b)) / dy;
}

/** The share of a surface `length` away along a ray (from `y0`, going `dy`) hidden by haze
 * and mist; or of the sky, for `length` Infinity (mist only: the sky holds its own haze). */
export function fogShare(f: FogSettings, y0: number, dy: number, length: number): number {
  // From where the fog starts.
  const s = Math.max(0, f.start ?? 0);
  if (length <= s) return 0;
  const from = y0 + dy * s;
  const rest = length - s;
  const haze = Number.isFinite(rest) ? extinction(f.visibility) * rest : 0;
  return 1 - Math.exp(-Math.min(60, haze + mistDepth(f, from, dy, rest)));
}

/** The GLSL of [`fogShare`], with its uniforms. */
export const FOG_GLSL = /* glsl */ `
  uniform float fogHaze;
  uniform float mistSigma;
  uniform float mistHeight;
  uniform float fogGround;
  uniform vec3 fogCam;
  uniform float fogStart;
  float mistDepth(float y0, float dy, float len, bool sky) {
    if (mistSigma <= 0.0) return 0.0;
    float a = exp(-(y0 - fogGround) / mistHeight);
    if (sky) return dy > 1e-3 ? mistSigma * a * mistHeight / dy : 50.0;
    if (abs(dy) < 1e-5) return mistSigma * a * len;
    float b = exp(-(y0 + dy * len - fogGround) / mistHeight);
    return mistSigma * mistHeight * (a - b) / dy;
  }
  float fogShare(vec3 dir, float len, bool sky) {
    if (!sky && len <= fogStart) return 0.0;
    float rest = len - fogStart;
    float haze = sky ? 0.0 : fogHaze * rest;
    return 1.0 - exp(-min(60.0, haze + mistDepth(fogCam.y + dir.y * fogStart, dir.y, rest, sky)));
  }
`;

/** The fog uniforms for a camera. */
export function fogUniforms(f: FogSettings, cam: THREE.Vector3): Record<string, THREE.IUniform> {
  return {
    fogHaze: { value: extinction(f.visibility) },
    mistSigma: { value: f.mistHeight > 0 ? extinction(f.mistVisibility) : 0 },
    mistHeight: { value: Math.max(f.mistHeight, 1) },
    fogGround: { value: f.ground },
    fogCam: { value: cam.clone() },
    fogStart: { value: Math.max(0, f.start ?? 0) },
  };
}

/** Whether `f` hides anything. */
export const hasFog = (f: FogSettings | null | undefined): f is FogSettings =>
  !!f && (f.visibility > 0 || (f.mistVisibility > 0 && f.mistHeight > 0));

const FOG_VERT = /* glsl */ `
  varying vec2 vUv;
  varying vec3 vWorld;
  void main() {
    vUv = uv;
    vec4 w = modelMatrix * vec4(position, 1.0);
    vWorld = w.xyz;
    gl_Position = projectionMatrix * viewMatrix * w;
  }
`;

const FOG_FRAG = /* glsl */ `
  ${FOG_GLSL}
  uniform sampler2D map;
  uniform bool useMap;
  uniform float alphaTest;
  varying vec2 vUv;
  varying vec3 vWorld;
  void main() {
    if (useMap && texture2D(map, vUv).a < alphaTest) discard;
    vec3 d = vWorld - fogCam;
    float len = length(d);
    gl_FragColor = vec4(fogShare(d / max(len, 1e-3), len, false), 0.0, 0.0, 1.0);
  }
`;

/** The fog's share at each pixel of the scene from `camera`: red the fog (weighted by the
 * coverage), alpha the model's coverage; where coverage is short of 1, the sky shows. */
export function fogPass(
  renderer: THREE.WebGLRenderer,
  scene: THREE.Scene,
  camera: THREE.PerspectiveCamera,
  width: number,
  height: number,
  f: FogSettings,
): THREE.WebGLRenderTarget {
  const rt = new THREE.WebGLRenderTarget(width, height, {
    type: THREE.HalfFloatType,
    format: THREE.RGBAFormat,
    minFilter: THREE.LinearFilter,
    magFilter: THREE.LinearFilter,
    depthBuffer: true,
  });
  const saved = new Map<THREE.Mesh, THREE.Material | THREE.Material[]>();
  const hidden: THREE.Object3D[] = [];
  const made: THREE.Material[] = [];
  const prevBg = scene.background;
  const prevTarget = renderer.getRenderTarget();
  const prevClear = renderer.getClearColor(new THREE.Color());
  const prevAlpha = renderer.getClearAlpha();
  const uniforms = fogUniforms(f, camera.position);
  try {
    scene.background = null;
    renderer.setClearColor(0x000000, 0);
    scene.traverse((o) => {
      if (!(o instanceof THREE.Mesh) || !o.visible) return;
      const m = (Array.isArray(o.material) ? o.material[0] : o.material) as
        THREE.MeshPhysicalMaterial | undefined;
      // Glass: the fog of what's behind it (a room, about as far).
      if (m && (m.transmission ?? 0) > 0.9) {
        o.visible = false;
        hidden.push(o);
        return;
      }
      const map = m?.map ?? null;
      const fm = new THREE.ShaderMaterial({
        vertexShader: FOG_VERT,
        fragmentShader: FOG_FRAG,
        uniforms: {
          ...uniforms,
          map: { value: map },
          useMap: { value: !!map && (m?.alphaTest ?? 0) > 0 },
          alphaTest: { value: m?.alphaTest ?? 0 },
        },
        side: THREE.DoubleSide,
      });
      made.push(fm);
      saved.set(o, o.material);
      o.material = fm;
    });
    const lens = lensOf(camera);
    renderJittered(renderer, scene, camera, rt, lens ? 12 : 6, lens);
  } finally {
    for (const [mesh, m] of saved) mesh.material = m;
    for (const o of hidden) o.visible = true;
    for (const m of made) m.dispose();
    scene.background = prevBg;
    renderer.setClearColor(prevClear, prevAlpha);
    renderer.setRenderTarget(prevTarget);
  }
  return rt;
}
