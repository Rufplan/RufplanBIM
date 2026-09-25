// Path-traced renderings of a 3D or camera view (ADR-027), with three-gpu-pathtracer.
// The model's meshes (from Rust) get physical materials by category; the sun comes from
// the site's location, date and time; a procedural sky lights the scene.
import * as THREE from "three";
import { WebGLPathTracer } from "three-gpu-pathtracer";
import type { CameraPose } from "../bindings/CameraPose";
import type { SunPosition } from "../bindings/SunPosition";
import type { Mesh } from "../ipc";
import { uvAt, type Imagery } from "../imagery";
import { meshColor } from "../components/View3D";

export interface RenderSettings {
  width: number;
  height: number;
  /** Stop after this many samples per pixel. */
  samples: number;
  /** None: no sun (night, or the sun below the horizon). */
  sun: SunPosition | null;
  background: "sky" | "white";
  exposure: number;
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
  opacity: number;
}

/** A category's physical surface: glass transmits, steel is metallic, the rest is matte. */
export function surfaceFor(m: Pick<Mesh, "category" | "color" | "exterior">): Surface {
  const base = meshColor(m as Mesh);
  const s: Surface = { color: base, roughness: 0.85, metalness: 0, transmission: 0, opacity: 1 };
  switch (m.category) {
    case "Window":
      return { color: 0xeef6f6, roughness: 0.02, metalness: 0, transmission: 1, opacity: 1 };
    case "Ceiling":
      return { ...s, color: 0xf2f2ef, roughness: 0.95 };
    case "Beam":
      return { ...s, roughness: 0.45, metalness: 0.6 };
    case "Railing":
      return { ...s, roughness: 0.35, metalness: 0.5 };
    case "Door":
      return { ...s, roughness: 0.6 };
    case "Roof":
      return { ...s, roughness: 0.75 };
    case "Site":
      return { ...s, roughness: 0.95 };
    default:
      return s;
  }
}

/** A blue sky fading to a hazy horizon, over a neutral ground: an equirectangular map for
 * the path tracer's environment light. Brighter toward the sun. */
export function skyTexture(sun: SunPosition | null, w = 256, h = 128): THREE.DataTexture {
  const data = new Float32Array(w * h * 4);
  const sd = sun ? toYUp(sun.dir).normalize() : null;
  const day = sun ? Math.max(0.15, Math.min(1, (sun.altitude + 6) / 30)) : 0.08;
  for (let j = 0; j < h; j++) {
    // three's equirect mapping: v = 0 (row 0, flipY off) is straight down, v = 1 up;
    // u = atan2(z, x) / 2π + 0.5.
    const elev = ((j + 0.5) / h - 0.5) * Math.PI;
    const up = Math.sin(elev);
    for (let i = 0; i < w; i++) {
      const phi = ((i + 0.5) / w - 0.5) * Math.PI * 2;
      const dir = new THREE.Vector3(
        Math.cos(elev) * Math.cos(phi),
        up,
        Math.cos(elev) * Math.sin(phi),
      );
      let r: number;
      let g: number;
      let b: number;
      if (up >= 0) {
        const t = Math.pow(up, 0.45);
        r = 0.95 * (1 - t) + 0.32 * t;
        g = 0.96 * (1 - t) + 0.52 * t;
        b = 1.0 * (1 - t) + 0.92 * t;
      } else {
        r = 0.34;
        g = 0.33;
        b = 0.31;
      }
      if (sd && up >= -0.05) {
        const glow = Math.pow(Math.max(0, dir.dot(sd)), 24) * 1.6;
        r += glow;
        g += glow * 0.92;
        b += glow * 0.78;
      }
      const k = (j * w + i) * 4;
      data[k] = r * day;
      data[k + 1] = g * day;
      data[k + 2] = b * day;
      data[k + 3] = 1;
    }
  }
  const tex = new THREE.DataTexture(data, w, h, THREE.RGBAFormat, THREE.FloatType);
  tex.mapping = THREE.EquirectangularReflectionMapping;
  tex.minFilter = THREE.LinearFilter;
  tex.magFilter = THREE.LinearFilter;
  tex.needsUpdate = true;
  return tex;
}

/** The render scene: the model in y-up, the ground, the sun and the sky. */
export function buildScene(
  meshes: Mesh[],
  settings: Pick<RenderSettings, "sun" | "background">,
  imagery: Imagery | null,
  groundZ: number,
): THREE.Scene {
  const scene = new THREE.Scene();
  let bounds = new THREE.Box3();
  let hasSite = false;
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
      ior: 1.5,
      thickness: s.transmission ? 6 : 0,
      side: THREE.DoubleSide,
    });
    if (m.category === "Site") {
      hasSite = true;
      if (imagery) {
        const uv = new Float32Array((p.length / 3) * 2);
        for (let i = 0, k = 0; i < p.length; i += 3, k += 2) {
          const [u, v] = uvAt(imagery.frame, p[i]!, p[i + 1]!);
          uv[k] = u;
          uv[k + 1] = v;
        }
        geo.setAttribute("uv", new THREE.BufferAttribute(uv, 2));
        const tex = new THREE.Texture(imagery.image);
        tex.colorSpace = THREE.SRGBColorSpace;
        tex.needsUpdate = true;
        mat.map = tex;
        mat.color.setHex(0xffffff);
      }
    }
    scene.add(new THREE.Mesh(geo, mat));
    geo.computeBoundingBox();
    bounds = bounds.union(geo.boundingBox!);
  }
  // Without topography, a plain ground so the building doesn't float.
  if (!hasSite) {
    const size = Math.max(200_000, bounds.getSize(new THREE.Vector3()).length() * 8);
    const ground = new THREE.Mesh(
      new THREE.PlaneGeometry(size, size),
      new THREE.MeshPhysicalMaterial({ color: 0xa9aaa2, roughness: 0.95 }),
    );
    ground.rotation.x = -Math.PI / 2;
    const c = bounds.isEmpty() ? new THREE.Vector3() : bounds.getCenter(new THREE.Vector3());
    ground.position.set(c.x, groundZ - 5, c.z);
    scene.add(ground);
  }
  const sky = skyTexture(settings.sun);
  scene.environment = sky;
  scene.background = settings.background === "sky" ? sky : new THREE.Color(0xffffff);
  if (settings.sun && settings.sun.altitude > 0) {
    const sun = new THREE.DirectionalLight(0xfff4e5, 3.2);
    const c = bounds.isEmpty() ? new THREE.Vector3() : bounds.getCenter(new THREE.Vector3());
    sun.position.copy(c.clone().add(toYUp(settings.sun.dir).normalize().multiplyScalar(100_000)));
    sun.target.position.copy(c);
    scene.add(sun, sun.target);
  }
  return scene;
}

export function cameraFor(pose: CameraPose, aspect: number): THREE.PerspectiveCamera {
  const cam = new THREE.PerspectiveCamera(pose.fov, aspect, 50, 1e7);
  cam.position.copy(toYUp(pose.eye));
  cam.up.set(0, 1, 0);
  cam.lookAt(toYUp(pose.target));
  cam.updateProjectionMatrix();
  return cam;
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
    this.renderer = new THREE.WebGLRenderer({ antialias: false, preserveDrawingBuffer: true });
    this.renderer.setPixelRatio(1);
    this.renderer.setSize(settings.width, settings.height, false);
    this.renderer.toneMapping = THREE.ACESFilmicToneMapping;
    this.renderer.toneMappingExposure = settings.exposure;
    this.renderer.outputColorSpace = THREE.SRGBColorSpace;
    this.canvas = this.renderer.domElement;
    this.tracer = new WebGLPathTracer(this.renderer);
    this.tracer.bounces = 6;
    this.tracer.transmissiveBounces = 10;
    this.tracer.filterGlossyFactor = 0.5;
    this.tracer.minSamples = 1;
    this.tracer.renderDelay = 0;
    this.tracer.fadeDuration = 0;
    this.tracer.dynamicLowRes = false;
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
    // Let the "Building the scene" status paint, then build the BVH (synchronously: the
    // async build needs a worker, which the app's content policy keeps out).
    await new Promise((ok) => setTimeout(ok, 30));
    this.tracer.setScene(scene, camera);
    if (this.stopped) return;
    this.started = performance.now();
    const loop = () => {
      if (this.stopped) return;
      this.tracer.renderSample();
      const n = Math.floor(this.tracer.samples);
      const secs = (performance.now() - this.started) / 1000;
      if (n >= samples) {
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

  /** The image as PNG bytes. */
  async png(): Promise<Uint8Array> {
    const blob = await new Promise<Blob | null>((ok) => this.canvas.toBlob(ok, "image/png"));
    if (!blob) throw new Error("the rendering couldn't be read back");
    return new Uint8Array(await blob.arrayBuffer());
  }

  dispose() {
    this.stop();
    this.tracer.dispose();
    this.renderer.dispose();
  }
}
