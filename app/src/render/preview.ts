// Material previews (ADR-029): a path-traced cube in a studio, as V-Ray's material editor
// shows them. Library presets ship pre-rendered; edited materials render on demand.
import * as THREE from "three";
import { RoundedBoxGeometry } from "three/examples/jsm/geometries/RoundedBoxGeometry.js";
import { HDRLoader } from "three/examples/jsm/loaders/HDRLoader.js";
import { FullScreenQuad } from "three/examples/jsm/postprocessing/Pass.js";
import { DenoiseMaterial, WebGLPathTracer } from "three-gpu-pathtracer";
import type { Appearance } from "../bindings/Appearance";
import { boxUv, physicalMaterial, type TextureSet } from "./materials";

let studio: Promise<THREE.DataTexture> | null = null;

/** The studio lighting (Poly Haven's studio_small_09, CC0). */
export function studioHdr(url = "/backgrounds/studio.hdr"): Promise<THREE.DataTexture> {
  if (!studio) {
    const loader = new HDRLoader();
    loader.setDataType(THREE.FloatType);
    studio = loader.loadAsync(url).then((t) => {
      t.mapping = THREE.EquirectangularReflectionMapping;
      return t;
    });
    studio.catch(() => (studio = null));
  }
  return studio;
}

/** The preview scene: an 800 mm cube turned 35°, on a grey floor, lit by the studio. */
export function previewScene(
  a: Appearance,
  color: [number, number, number],
  tex: TextureSet | null,
  env: THREE.Texture,
): { scene: THREE.Scene; camera: THREE.PerspectiveCamera } {
  const scene = new THREE.Scene();
  // 500 mm with rounded edges: the edges catch highlights (metals, gloss), and patterns
  // read at their real size.
  const s = 500;
  const cube = new RoundedBoxGeometry(s, s, s, 4, 22).toNonIndexed();
  cube.rotateY((35 * Math.PI) / 180);
  cube.translate(0, s / 2, 0);
  boxUv(cube, a.scale, tex?.aspect ?? 1);
  cube.computeVertexNormals();
  scene.add(new THREE.Mesh(cube, physicalMaterial(a, color, tex)));
  const floor = new THREE.PlaneGeometry(40_000, 40_000);
  floor.rotateX(-Math.PI / 2);
  scene.add(
    new THREE.Mesh(
      floor,
      new THREE.MeshPhysicalMaterial({ color: 0x6f7174, roughness: 0.75, specularIntensity: 0.3 }),
    ),
  );
  scene.environment = env;
  scene.environmentIntensity = 1.0;
  // Two softboxes, as in V-Ray's material previews: a key from the front left and a rim
  // behind, so metals, glass and gloss show crisp reflections.
  const softbox = (w: number, h: number, intensity: number, at: [number, number, number]) => {
    const l = new THREE.RectAreaLight(0xffffff, intensity, w, h);
    l.position.set(...at);
    l.lookAt(0, 400, 0);
    scene.add(l);
  };
  softbox(2200, 2200, 1.2, [-1500, 1900, 1500]);
  softbox(1400, 2400, 0.8, [1700, 1300, -1300]);
  scene.background = null;
  const camera = new THREE.PerspectiveCamera(28, 1, 50, 1e6);
  camera.position.set(1450, 980, 1450);
  camera.lookAt(0, 205, 0);
  camera.updateProjectionMatrix();
  return { scene, camera };
}

/** Renders a preview to a new canvas (square, `size` px) with `samples` per pixel, over a
 * studio gradient. */
export async function renderPreview(
  a: Appearance,
  color: [number, number, number],
  tex: TextureSet | null,
  size = 256,
  samples = 96,
): Promise<HTMLCanvasElement> {
  const env = await studioHdr();
  const { scene, camera } = previewScene(a, color, tex, env);
  const renderer = new THREE.WebGLRenderer({
    antialias: false,
    alpha: true,
    premultipliedAlpha: true,
    preserveDrawingBuffer: true,
  });
  const out = document.createElement("canvas");
  out.width = size;
  out.height = size;
  try {
    renderer.setPixelRatio(1);
    renderer.setSize(size, size, false);
    renderer.setClearColor(0x000000, 0);
    renderer.toneMapping = THREE.ACESFilmicToneMapping;
    // A touch under, so whites keep their texture and greys read as grey.
    renderer.toneMappingExposure = 0.85;
    renderer.outputColorSpace = THREE.SRGBColorSpace;
    const tracer = new WebGLPathTracer(renderer);
    tracer.bounces = 6;
    // Tames sparkles from glossy reflections of the softboxes, as in renders.
    tracer.filterGlossyFactor = 0.5;
    tracer.multipleImportanceSampling = true;
    tracer.transmissiveBounces = 8;
    tracer.minSamples = 1;
    tracer.renderDelay = 0;
    tracer.fadeDuration = 0;
    tracer.rasterizeScene = false;
    tracer.tiles.set(1, 1);
    tracer.setScene(scene, camera);
    while (tracer.samples < samples) {
      tracer.renderSample();
      // Yield now and then so the page stays responsive.
      if (Math.floor(tracer.samples) % 8 === 0) await new Promise((r) => setTimeout(r, 0));
    }
    // The same edge-aware denoise as renders.
    const dn = new DenoiseMaterial({
      map: tracer.target.texture,
      blending: THREE.NoBlending,
      premultipliedAlpha: true,
    });
    // Light: more samples do the work, so texture detail stays crisp.
    dn.sigma = 2.5;
    dn.threshold = 0.03;
    const quad = new FullScreenQuad(dn);
    renderer.setRenderTarget(null);
    quad.render(renderer);
    quad.dispose();
    dn.dispose();
    const ctx = out.getContext("2d")!;
    const g = ctx.createLinearGradient(0, 0, 0, size);
    g.addColorStop(0, "#5a5d62");
    g.addColorStop(1, "#2a2c2f");
    ctx.fillStyle = g;
    ctx.fillRect(0, 0, size, size);
    ctx.drawImage(renderer.domElement, 0, 0);
    tracer.dispose();
  } finally {
    renderer.dispose();
  }
  return out;
}
