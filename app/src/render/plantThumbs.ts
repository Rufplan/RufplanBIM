// Asset Library thumbnails (ADR-064): each plant's own model and textures, lit by a soft
// sun and sky on a pale ground, framed to fill the tile. One renderer draws them in turn;
// results are kept for the session.
import * as THREE from "three";
import { RoomEnvironment } from "three/examples/jsm/environments/RoomEnvironment.js";
import type { PlantSource } from "../bindings/PlantSource";
import { plantAssets, plantGroup, sourceKey, type PlantLoader } from "./plants";

const SIZE = 256;
let shared: { renderer: THREE.WebGLRenderer; env: THREE.Texture } | null = null;
const cache = new Map<string, Promise<string | null>>();
let queue: Promise<unknown> = Promise.resolve();

function setup() {
  if (shared) return shared;
  const renderer = new THREE.WebGLRenderer({ antialias: true, preserveDrawingBuffer: true });
  renderer.setSize(SIZE, SIZE);
  renderer.setPixelRatio(1);
  renderer.outputColorSpace = THREE.SRGBColorSpace;
  renderer.toneMapping = THREE.ACESFilmicToneMapping;
  renderer.toneMappingExposure = 1.05;
  renderer.shadowMap.enabled = true;
  renderer.shadowMap.type = THREE.PCFShadowMap;
  const pmrem = new THREE.PMREMGenerator(renderer);
  const env = pmrem.fromScene(new RoomEnvironment(), 0.04).texture;
  pmrem.dispose();
  shared = { renderer, env };
  return shared;
}

async function draw(source: PlantSource, load: PlantLoader): Promise<string | null> {
  let s;
  try {
    s = setup();
  } catch {
    return null;
  }
  const a = await plantAssets(source, 0, load, 512);
  const scene = new THREE.Scene();
  scene.environment = s.env;
  scene.environmentIntensity = 0.5;
  scene.background = new THREE.Color(0xeef1ec);
  const g = plantGroup(
    [
      {
        assets: a,
        instances: [
          {
            el: "",
            type_id: "",
            variant: 0,
            spec_key: "",
            at: [0, 0, 0],
            rotation: 0.6,
            scale: 1,
          },
        ],
      },
    ],
    false,
  );
  scene.add(g);
  const h = Math.max(a.model.height, 300);
  const w = Math.max(a.model.spread, 300);
  const size = Math.max(h, w);
  const sun = new THREE.DirectionalLight(0xfff4e6, 2.6);
  sun.position.set(size * 1.2, -size * 1.4, size * 2.2);
  sun.target.position.set(0, 0, h * 0.4);
  sun.castShadow = true;
  sun.shadow.mapSize.set(1024, 1024);
  Object.assign(sun.shadow.camera, {
    left: -size,
    right: size,
    top: size,
    bottom: -size,
    near: 1,
    far: size * 6,
  });
  scene.add(sun, sun.target);
  const ground = new THREE.Mesh(
    new THREE.CircleGeometry(size * 1.2, 48),
    new THREE.MeshStandardMaterial({ color: 0xd9ddd2, roughness: 1 }),
  );
  ground.receiveShadow = true;
  scene.add(ground);
  const cam = new THREE.PerspectiveCamera(28, 1, size / 100, size * 20);
  cam.up.set(0, 0, 1);
  // Fit the plant: its height (and spread) in the frame, seen a little from above.
  const fov = (28 * Math.PI) / 180;
  const d = (Math.max(h, w * 0.95) * 0.62) / Math.tan(fov / 2);
  cam.position.set(d * 0.35, -d * 0.94, h * 0.5 + d * 0.12);
  cam.lookAt(0, 0, h * 0.47);
  s.renderer.render(scene, cam);
  const url = s.renderer.domElement.toDataURL("image/png");
  g.traverse((o) => {
    if (o instanceof THREE.InstancedMesh) o.dispose();
  });
  return url;
}

/** A plant's thumbnail (a data URL), drawn once per source. */
export function plantThumb(source: PlantSource, load: PlantLoader): Promise<string | null> {
  const key = sourceKey(source);
  let p = cache.get(key);
  if (!p) {
    p = queue.then(() => draw(source, load)).catch(() => null);
    queue = p;
    cache.set(key, p);
  }
  return p;
}

/** Forgets a type's thumbnail (it changed). */
export function forgetThumb(source: PlantSource) {
  cache.delete(sourceKey(source));
}
