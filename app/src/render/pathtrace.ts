// Path-traced renderings of a 3D or camera view (ADR-027, ADR-028), with three-gpu-pathtracer.
// Lit like V-Ray: a physical sun & sky (or a photo HDR as a dome light), physically based
// materials, many bounces of global illumination, filmic (AgX) tone mapping and a final
// denoise. The model renders over a transparent background; the backdrop is composited
// behind it, so one rendering saves with or without the background.
import * as THREE from "three";
import { FullScreenQuad } from "three/examples/jsm/postprocessing/Pass.js";
import { DenoiseMaterial, WebGLPathTracer } from "three-gpu-pathtracer";
import type { CameraPose } from "../bindings/CameraPose";
import type { Mesh } from "../ipc";
import { uvAt, type Imagery } from "../imagery";
import { meshColor } from "../components/View3D";
import { GROUND_ALBEDO } from "./sky";

export interface RenderSettings {
  width: number;
  height: number;
  /** Stop after this many samples per pixel. */
  samples: number;
  exposure: number;
  /** Tone curve: punchier contrast (ACES, the default) or soft filmic (AgX). */
  tone?: "filmic" | "contrast";
}

/** Model space is z-up; three.js (and the path tracer's sky) is y-up. */
export function toYUp([x, y, z]: [number, number, number] | number[]): THREE.Vector3 {
  return new THREE.Vector3(x!, z!, -y!);
}

export interface Surface {
  color: number;
  roughness: number;
  metalness: number;
  transmission: number;
  clearcoat: number;
  specularIntensity: number;
}

/** A category's physical surface, tuned like V-Ray's standard materials: glass with
 * Fresnel reflections, metals, satin paint on doors, and matte but not dead masonry. */
export function surfaceFor(m: Pick<Mesh, "category" | "color" | "exterior">): Surface {
  const base = meshColor(m as Mesh);
  const s: Surface = {
    color: base,
    roughness: 0.75,
    metalness: 0,
    transmission: 0,
    clearcoat: 0,
    specularIntensity: 0.5,
  };
  switch (m.category) {
    case "Window":
      return { ...s, color: 0xf4faf8, roughness: 0, transmission: 1, specularIntensity: 1 };
    case "Ceiling":
      return { ...s, color: 0xf2f2ef, roughness: 0.9, specularIntensity: 0.3 };
    case "Beam":
      return { ...s, roughness: 0.35, metalness: 0.85 };
    case "Railing":
      return { ...s, roughness: 0.3, metalness: 0.85 };
    case "Door":
      return { ...s, roughness: 0.45, clearcoat: 0.3 };
    case "Roof":
      return { ...s, roughness: 0.65 };
    case "Floor":
    case "Stair":
      return { ...s, roughness: 0.55 };
    case "Site":
      return { ...s, roughness: 0.95, specularIntensity: 0.2 };
    default:
      return s;
  }
}

/** V-Ray's dome ground projection: the photo's own ground mapped onto the ground plane
 * as seen from where the photo was taken, so the model casts shadows on it and it meets
 * the backdrop at the horizon. */
export interface GroundProjection {
  photo: THREE.Texture;
  /** The backdrop's rotation (radians), so the ground lines up with it. */
  rotation: number;
  /** Where the photo was taken from: over the camera, this high above the ground (mm). */
  center: THREE.Vector3;
  height: number;
  /** Photo radiance to ground albedo: π over the light's horizontal irradiance. */
  albedoScale: number;
}

export interface SceneOptions {
  /** The light: a sun & sky map or a photo HDR (V-Ray's Dome light). */
  environment: THREE.Texture;
  /** Brightness of the environment (a photo HDR is normalised to the sun & sky's). */
  environmentIntensity?: number;
  /** Turns the environment about the vertical (radians); for a photo HDR. */
  rotation: number;
  ground: "grass" | "paving" | "neutral";
  projection?: GroundProjection | null;
  imagery: Imagery | null;
  /** Elevation of the lowest level (mm), for the ground plane. */
  groundZ: number;
}

/** Light falling on a horizontal surface from an equirectangular environment map
 * (luminance-weighted), for matching photo ground and normalising HDRs. */
export function horizontalIrradiance(tex: THREE.DataTexture): number {
  const img = tex.image as { data: ArrayLike<number>; width: number; height: number };
  const { data, width: w, height: h } = img;
  const ch = data.length / (w * h);
  let e = 0;
  // Rows run bottom-up unless the texture is flipped (loaded images and HDRs are).
  const flip = tex.flipY;
  for (let j = 0; j < h; j++) {
    const elev = (flip ? 0.5 - (j + 0.5) / h : (j + 0.5) / h - 0.5) * Math.PI;
    if (elev <= 0) continue;
    const weight = Math.sin(elev) * Math.cos(elev) * (Math.PI / h) * ((2 * Math.PI) / w);
    for (let i = 0; i < w; i++) {
      const k = (j * w + i) * ch;
      const lum = 0.2126 * data[k]! + 0.7152 * data[k + 1]! + 0.0722 * data[k + 2]!;
      e += lum * weight;
    }
  }
  return e;
}

/** Equirect texture coordinates of direction `d` turned by the backdrop's rotation, as
 * three samples a rotated background. */
export function equirectUv(d: THREE.Vector3, rotation: number): [number, number] {
  const a = -rotation;
  const x = d.x * Math.cos(a) + d.z * Math.sin(a);
  const z = -d.x * Math.sin(a) + d.z * Math.cos(a);
  const len = Math.hypot(x, d.y, z) || 1;
  return [
    Math.atan2(z, x) / (2 * Math.PI) + 0.5,
    Math.asin(Math.max(-1, Math.min(1, d.y / len))) / Math.PI + 0.5,
  ];
}

/** A ground disc out to the horizon, in rings around the photo's viewpoint (finest near
 * it), textured by the photo's ground as seen from there. */
export function projectedGround(
  groundY: number,
  pr: GroundProjection,
  // As far as the photo shows ground; past this the backdrop shows its own view (trees,
  // buildings) rather than smearing it across the ground.
  radius = 50_000,
): THREE.BufferGeometry {
  const rings = 90;
  const segs = 256;
  const r0 = 150;
  const cx = pr.center.x;
  const cz = pr.center.z;
  const ring = (i: number, s: number): [number, number, number] => {
    if (i < 0) return [cx, groundY, cz];
    const r = r0 * Math.pow(radius / r0, i / (rings - 1));
    const a = (s / segs) * Math.PI * 2;
    return [cx + Math.cos(a) * r, groundY, cz + Math.sin(a) * r];
  };
  const eye = new THREE.Vector3(cx, groundY + pr.height, cz);
  const pos: number[] = [];
  const uv: number[] = [];
  const v = new THREE.Vector3();
  const tri = (a: number[], b: number[], c: number[]) => {
    const us: [number, number][] = [a, b, c].map((q) => {
      v.set(q[0]! - eye.x, q[1]! - eye.y, q[2]! - eye.z);
      return equirectUv(v, pr.rotation);
    });
    // Straight below the viewpoint the direction has no heading: take its neighbours'.
    const pts = [a, b, c];
    const nadir = pts.map((q) => Math.hypot(q[0]! - eye.x, q[2]! - eye.z) < 1);
    const rest = us.filter((_, i) => !nadir[i]);
    if (rest.length && rest.length < 3) {
      const wrap = Math.max(...rest.map((q) => q[0])) - Math.min(...rest.map((q) => q[0])) > 0.5;
      const mean =
        rest.reduce((s, q) => s + (wrap && q[0] < 0.5 ? q[0] + 1 : q[0]), 0) / rest.length;
      us.forEach((q, i) => {
        if (nadir[i]) q[0] = mean;
      });
    }
    // A triangle across the panorama's seam: keep its u on one side (the map repeats).
    const lo = Math.min(...us.map((q) => q[0]));
    const hi = Math.max(...us.map((q) => q[0]));
    if (hi - lo > 0.5) for (const q of us) if (q[0] < 0.5) q[0] += 1;
    for (const q of [a, b, c]) pos.push(q[0]!, q[1]!, q[2]!);
    for (const q of us) uv.push(q[0], q[1]);
  };
  for (let i = -1; i < rings - 1; i++) {
    for (let s = 0; s < segs; s++) {
      const a = ring(i, s);
      const b = ring(i, s + 1);
      const c = ring(i + 1, s);
      const d = ring(i + 1, s + 1);
      // Counter-clockwise seen from above, so the ground faces up.
      if (i >= 0) tri(a, d, b);
      tri(a, c, d);
    }
  }
  const geo = new THREE.BufferGeometry();
  geo.setAttribute("position", new THREE.Float32BufferAttribute(pos, 3));
  geo.setAttribute("uv", new THREE.Float32BufferAttribute(uv, 2));
  geo.computeVertexNormals();
  return geo;
}

/** The render scene: the model in y-up, the ground and the environment light. */
export function buildScene(meshes: Mesh[], o: SceneOptions): THREE.Scene {
  const scene = new THREE.Scene();
  let bounds = new THREE.Box3();
  let hasSite = false;
  // A plain lawn, paving or neutral ground in linear colour (a tiled texture bands and
  // shimmers toward the horizon).
  const groundColor = () =>
    new THREE.Color().setRGB(...GROUND_ALBEDO[o.ground], THREE.LinearSRGBColorSpace);
  for (const m of meshes) {
    if (m.positions.length === 0) continue;
    const p = m.positions;
    const out = new Float32Array(p.length);
    for (let i = 0; i < p.length; i += 3) {
      out[i] = p[i]!;
      out[i + 1] = p[i + 2]!;
      out[i + 2] = -p[i + 1]!;
    }
    const geo = new THREE.BufferGeometry();
    geo.setAttribute("position", new THREE.Float32BufferAttribute(out, 3));
    geo.computeVertexNormals();
    const s = surfaceFor(m);
    const mat = new THREE.MeshPhysicalMaterial({
      color: s.color,
      roughness: s.roughness,
      metalness: s.metalness,
      transmission: s.transmission,
      clearcoat: s.clearcoat,
      specularIntensity: s.specularIntensity,
      ior: s.transmission ? 1.52 : 1.5,
      // Thin glass: panes reflect and transmit without bending light or trapping rays
      // inside the window's box, as V-Ray's architectural glass does.
      thickness: 0,
      side: THREE.DoubleSide,
    });
    // The sun shines straight through glass (no caustic noise), as V-Ray does by default.
    if (s.transmission) Object.assign(mat, { castShadow: false });
    if (m.category === "Site") {
      hasSite = true;
      if (o.imagery) {
        const uv = new Float32Array((p.length / 3) * 2);
        for (let i = 0, k = 0; i < p.length; i += 3, k += 2) {
          const [u, v] = uvAt(o.imagery.frame, p[i]!, p[i + 1]!);
          uv[k] = u;
          uv[k + 1] = v;
        }
        geo.setAttribute("uv", new THREE.BufferAttribute(uv, 2));
        const tex = new THREE.Texture(o.imagery.image);
        tex.colorSpace = THREE.SRGBColorSpace;
        tex.needsUpdate = true;
        mat.map = tex;
        mat.color.setHex(0xffffff);
      } else {
        mat.color.copy(groundColor());
      }
    }
    const mesh = new THREE.Mesh(geo, mat);
    // Topography is ground: left out of the transparent cut-out.
    if (m.category === "Site") mesh.userData.ground = true;
    scene.add(mesh);
    geo.computeBoundingBox();
    bounds = bounds.union(geo.boundingBox!);
  }
  // Without topography: the photo's own ground projected around the model (V-Ray's
  // ground projection), or a plain lawn or paving out toward the horizon.
  if (!hasSite && o.projection) {
    const pr = o.projection;
    const geo = projectedGround(o.groundZ - 5, pr);
    const photo = pr.photo.clone();
    photo.mapping = THREE.UVMapping;
    photo.wrapS = THREE.RepeatWrapping;
    photo.needsUpdate = true;
    const k = Math.min(1, pr.albedoScale);
    const ground = new THREE.Mesh(
      geo,
      new THREE.MeshPhysicalMaterial({
        map: photo,
        color: new THREE.Color(k, k, k),
        roughness: 0.95,
        specularIntensity: 0.15,
        side: THREE.DoubleSide,
      }),
    );
    ground.userData.ground = true;
    scene.add(ground);
  } else if (!hasSite) {
    const size = 10_000_000;
    const geo = new THREE.PlaneGeometry(size, size);
    geo.rotateX(-Math.PI / 2);
    const c = bounds.isEmpty() ? new THREE.Vector3() : bounds.getCenter(new THREE.Vector3());
    geo.translate(c.x, o.groundZ - 5, c.z);
    const ground = new THREE.Mesh(
      geo,
      new THREE.MeshPhysicalMaterial({
        color: groundColor(),
        roughness: 0.95,
        specularIntensity: 0.2,
        side: THREE.DoubleSide,
      }),
    );
    ground.userData.ground = true;
    scene.add(ground);
  }
  scene.environment = o.environment;
  scene.environmentIntensity = o.environmentIntensity ?? 1;
  scene.environmentRotation.set(0, o.rotation, 0);
  // Transparent where rays leave the model: the backdrop goes behind.
  scene.background = null;
  return scene;
}

export function cameraFor(pose: CameraPose, aspect: number): THREE.PerspectiveCamera {
  const cam = new THREE.PerspectiveCamera(pose.fov, aspect, 50, 2e7);
  cam.position.copy(toYUp(pose.eye));
  cam.up.set(0, 1, 0);
  cam.lookAt(toYUp(pose.target));
  cam.updateProjectionMatrix();
  return cam;
}

/** The backdrop as the camera sees it: a panorama (a photo, or the physical sky
 * tone-mapped like the render) or white. */
export function renderBackdrop(
  width: number,
  height: number,
  camera: THREE.PerspectiveCamera,
  pano: {
    texture: THREE.Texture;
    rotation: number;
    exposure: number;
    tone?: "filmic" | "contrast";
  } | null,
): HTMLCanvasElement {
  const out = document.createElement("canvas");
  out.width = width;
  out.height = height;
  const ctx = out.getContext("2d")!;
  if (!pano) {
    ctx.fillStyle = "#ffffff";
    ctx.fillRect(0, 0, width, height);
    return out;
  }
  const renderer = new THREE.WebGLRenderer({ antialias: true, preserveDrawingBuffer: true });
  try {
    renderer.setPixelRatio(1);
    renderer.setSize(width, height, false);
    renderer.outputColorSpace = THREE.SRGBColorSpace;
    // The same tone curve as the render, so projected ground meets the backdrop cleanly.
    renderer.toneMapping =
      pano.tone === "filmic" ? THREE.AgXToneMapping : THREE.ACESFilmicToneMapping;
    renderer.toneMappingExposure = pano.exposure;
    const scene = new THREE.Scene();
    scene.background = pano.texture;
    scene.backgroundRotation.set(0, pano.rotation, 0);
    renderer.render(scene, camera);
    ctx.drawImage(renderer.domElement, 0, 0);
  } finally {
    renderer.dispose();
  }
  return out;
}

/** A rendering in progress: sharpens every frame until it has `samples` or is stopped. */
export class RenderJob {
  readonly canvas: HTMLCanvasElement;
  private renderer: THREE.WebGLRenderer;
  private tracer: WebGLPathTracer;
  private raf = 0;
  private stopped = false;
  private started = 0;

  constructor(settings: RenderSettings) {
    this.renderer = new THREE.WebGLRenderer({
      antialias: false,
      alpha: true,
      premultipliedAlpha: true,
      preserveDrawingBuffer: true,
    });
    this.renderer.setPixelRatio(1);
    this.renderer.setSize(settings.width, settings.height, false);
    this.renderer.setClearColor(0x000000, 0);
    this.renderer.toneMapping =
      settings.tone === "filmic" ? THREE.AgXToneMapping : THREE.ACESFilmicToneMapping;
    this.renderer.toneMappingExposure = settings.exposure;
    this.renderer.outputColorSpace = THREE.SRGBColorSpace;
    this.canvas = this.renderer.domElement;
    this.tracer = new WebGLPathTracer(this.renderer);
    // Global illumination as V-Ray's brute force: many diffuse bounces, deep glass.
    this.tracer.bounces = 8;
    this.tracer.transmissiveBounces = 12;
    this.tracer.filterGlossyFactor = 0.5;
    this.tracer.multipleImportanceSampling = true;
    this.tracer.minSamples = 1;
    this.tracer.renderDelay = 0;
    this.tracer.fadeDuration = 0;
    this.tracer.dynamicLowRes = false;
    this.tracer.rasterizeScene = false;
    // Tiles keep the app responsive while a large image renders.
    const tiles = settings.width > 2000 ? 3 : 2;
    this.tracer.tiles.set(tiles, tiles);
  }

  async start(
    scene: THREE.Scene,
    camera: THREE.PerspectiveCamera,
    samples: number,
    onProgress: (
      samples: number,
      seconds: number,
      phase: "preparing" | "rendering" | "done",
    ) => void,
  ) {
    onProgress(0, 0, "preparing");
    // Let the status paint, then build the BVH (synchronously: the async build needs a
    // worker, which the app's content policy keeps out).
    await new Promise((ok) => setTimeout(ok, 30));
    if (this.stopped) return;
    this.tracer.setScene(scene, camera);
    this.started = performance.now();
    const loop = () => {
      if (this.stopped) return;
      this.tracer.renderSample();
      const n = Math.floor(this.tracer.samples);
      const secs = (performance.now() - this.started) / 1000;
      if (n >= samples) {
        this.stopped = true;
        onProgress(n, secs, "done");
        return;
      }
      onProgress(n, secs, "rendering");
      this.raf = requestAnimationFrame(loop);
    };
    loop();
  }

  stop() {
    this.stopped = true;
    cancelAnimationFrame(this.raf);
  }

  /** Smooths the remaining noise, edge-aware, as V-Ray's denoiser does at the end. */
  denoise() {
    const mat = new DenoiseMaterial({
      map: this.tracer.target.texture,
      blending: THREE.NoBlending,
      premultipliedAlpha: true,
    });
    // Stronger than the library's default, keeping edges: interiors seen through glass
    // are the noisiest part of an exterior view.
    mat.sigma = 7;
    mat.threshold = 0.08;
    mat.kSigma = 1.2;
    const quad = new FullScreenQuad(mat);
    this.renderer.setRenderTarget(null);
    quad.render(this.renderer);
    quad.dispose();
    mat.dispose();
  }

  dispose() {
    this.stop();
    this.tracer.dispose();
    this.renderer.dispose();
  }
}

/** Draws the render over its backdrop. */
export function composite(
  display: HTMLCanvasElement,
  backdrop: HTMLCanvasElement | null,
  render: HTMLCanvasElement,
) {
  const ctx = display.getContext("2d");
  if (!ctx) return;
  ctx.clearRect(0, 0, display.width, display.height);
  if (backdrop) ctx.drawImage(backdrop, 0, 0, display.width, display.height);
  ctx.drawImage(render, 0, 0, display.width, display.height);
}

/** A canvas as PNG bytes. */
export async function pngOf(canvas: HTMLCanvasElement): Promise<Uint8Array> {
  const blob = await new Promise<Blob | null>((ok) => canvas.toBlob(ok, "image/png"));
  if (!blob) throw new Error("the rendering couldn't be read back");
  return new Uint8Array(await blob.arrayBuffer());
}

/** The building alone for a transparent PNG: the render kept only where the model (not the
 * ground or the sky) is in front of the camera. The mask is a quick antialiased raster. */
export function cutout(
  render: HTMLCanvasElement,
  scene: THREE.Scene,
  camera: THREE.PerspectiveCamera,
): HTMLCanvasElement {
  const w = render.width;
  const h = render.height;
  const renderer = new THREE.WebGLRenderer({
    antialias: true,
    alpha: true,
    preserveDrawingBuffer: true,
  });
  const out = document.createElement("canvas");
  out.width = w;
  out.height = h;
  try {
    renderer.setPixelRatio(1);
    renderer.setSize(w, h, false);
    renderer.setClearColor(0x000000, 0);
    const mask = new THREE.Scene();
    const white = new THREE.MeshBasicMaterial({ color: 0xffffff, side: THREE.DoubleSide });
    for (const c of scene.children) {
      if (c instanceof THREE.Mesh && !c.userData.ground)
        mask.add(new THREE.Mesh(c.geometry, white));
    }
    renderer.render(mask, camera);
    const ctx = out.getContext("2d")!;
    ctx.drawImage(render, 0, 0);
    ctx.globalCompositeOperation = "destination-in";
    ctx.drawImage(renderer.domElement, 0, 0);
    white.dispose();
  } finally {
    renderer.dispose();
  }
  return out;
}
