import * as THREE from "three";
import { RoomEnvironment } from "three/examples/jsm/environments/RoomEnvironment.js";
import type { OpeningThumb } from "../bindings/OpeningThumb";
import type { FixtureThumb } from "../bindings/FixtureThumb";
import type { FfeThumb } from "../bindings/FfeThumb";
import { toYUp } from "./pathtrace";

// Rendered thumbnails of door and window types for the type picker (ADR-033): the type in a
// short piece of wall, seen from outside at three-quarters, lit by a studio environment with
// a soft key light. One renderer draws them in turn; results are cached for the session.

const SIZE = 320;
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
  renderer.toneMappingExposure = 0.95;
  renderer.shadowMap.enabled = true;
  renderer.shadowMap.type = THREE.PCFSoftShadowMap;
  const pmrem = new THREE.PMREMGenerator(renderer);
  const env = pmrem.fromScene(new RoomEnvironment(), 0.04).texture;
  pmrem.dispose();
  shared = { renderer, env };
  return shared;
}

function geometry(tris: number[]): THREE.BufferGeometry {
  const pos = new Float32Array(tris.length);
  for (let i = 0; i < tris.length; i += 3) {
    const v = toYUp([tris[i]!, tris[i + 1]!, tris[i + 2]!]);
    pos[i] = v.x;
    pos[i + 1] = v.y;
    pos[i + 2] = v.z;
  }
  const g = new THREE.BufferGeometry();
  g.setAttribute("position", new THREE.BufferAttribute(pos, 3));
  g.computeVertexNormals();
  return g;
}

const srgb = (c: [number, number, number]) =>
  new THREE.Color().setRGB(c[0] / 255, c[1] / 255, c[2] / 255, THREE.SRGBColorSpace);

/** Draws one thumbnail; its data URL, or null where WebGL isn't available. */
export function renderThumb(t: OpeningThumb | FixtureThumb | FfeThumb): string | null {
  if ("parts" in t) return renderFfe(t);
  if ("lens" in t) return renderFixture(t);
  let s;
  try {
    s = setup();
  } catch {
    return null;
  }
  const scene = new THREE.Scene();
  scene.environment = s.env;
  scene.environmentIntensity = 0.55;
  scene.background = new THREE.Color(0xe6e4df);
  const metal = t.color[0] > 170 && Math.abs(t.color[0] - t.color[2]) < 12 && t.color[0] < 215;
  const mats = {
    wall: new THREE.MeshStandardMaterial({ color: 0xc9c1b4, roughness: 0.95 }),
    frame: new THREE.MeshStandardMaterial({
      color: srgb(t.color),
      roughness: metal ? 0.3 : 0.5,
      metalness: metal ? 0.8 : 0,
    }),
    glass: new THREE.MeshPhysicalMaterial({
      color: 0x5d7c86,
      roughness: 0.02,
      metalness: 0.1,
      transparent: true,
      opacity: 0.62,
      envMapIntensity: 3,
      clearcoat: 1,
    }),
  };
  const meshes: THREE.Mesh[] = [];
  for (const [part, tris] of [
    ["wall", t.wall],
    ["frame", t.frame],
    ["glass", t.glass],
  ] as const) {
    if (!tris.length) continue;
    const m = new THREE.Mesh(geometry(tris), mats[part]);
    m.castShadow = part !== "glass";
    m.receiveShadow = true;
    scene.add(m);
    meshes.push(m);
  }
  const box = new THREE.Box3();
  for (const m of meshes) box.expandByObject(m);
  const center = box.getCenter(new THREE.Vector3());
  const radius = box.getSize(new THREE.Vector3()).length() / 2;
  // Outside is model +y, three.js -z: look from the front, a little left and above.
  const cam = new THREE.PerspectiveCamera(28, 1, radius / 50, radius * 20);
  const dir = new THREE.Vector3(-0.42, 0.18, -1).normalize();
  const dist = radius / Math.sin(THREE.MathUtils.degToRad(14)) / 1.18;
  cam.position.copy(center).addScaledVector(dir, dist);
  cam.lookAt(center);
  // A raking key from the left and above, so panels and casings cast shadows.
  const key = new THREE.DirectionalLight(0xfff1e0, 2.6);
  key.position.copy(center).add(new THREE.Vector3(-radius * 3, radius * 2.2, -radius * 1.4));
  key.target.position.copy(center);
  key.castShadow = true;
  key.shadow.mapSize.set(1024, 1024);
  const sc = key.shadow.camera as THREE.OrthographicCamera;
  sc.left = sc.bottom = -radius * 1.5;
  sc.right = sc.top = radius * 1.5;
  sc.near = radius * 0.1;
  sc.far = radius * 10;
  key.shadow.bias = -0.0005;
  key.shadow.radius = 4;
  scene.add(key, key.target);
  try {
    s.renderer.render(scene, cam);
    return s.renderer.domElement.toDataURL("image/png");
  } catch {
    return null;
  } finally {
    for (const m of meshes) m.geometry.dispose();
    for (const m of Object.values(mats)) m.dispose();
    key.shadow.map?.dispose();
  }
}

/** A lighting fixture (ADR-057): its body and its lens lit, on a dark grey studio backdrop.
 * Ceiling and pendant fixtures are seen from below, the rest from the front. */
function renderFixture(t: FixtureThumb): string | null {
  let s;
  try {
    s = setup();
  } catch {
    return null;
  }
  const scene = new THREE.Scene();
  scene.environment = s.env;
  scene.environmentIntensity = 0.6;
  scene.background = new THREE.Color(0x2f3336);
  const glow = srgb(t.glow);
  const mats = {
    body: new THREE.MeshStandardMaterial({ color: srgb(t.color), roughness: 0.45, metalness: 0.2 }),
    lens: new THREE.MeshStandardMaterial({ color: glow, emissive: glow, emissiveIntensity: 1.4 }),
  };
  const meshes: THREE.Mesh[] = [];
  for (const [part, tris] of [
    ["body", t.body],
    ["lens", t.lens],
  ] as const) {
    if (!tris.length) continue;
    const m = new THREE.Mesh(geometry(tris), mats[part]);
    scene.add(m);
    meshes.push(m);
  }
  const box = new THREE.Box3();
  for (const m of meshes) box.expandByObject(m);
  const lensBox = new THREE.Box3();
  if (meshes.length > 1) lensBox.setFromObject(meshes[1]!);
  const center = box.getCenter(new THREE.Vector3());
  const radius = Math.max(box.getSize(new THREE.Vector3()).length() / 2, 1);
  // The lens faces down when it's the bottom of the fixture.
  const below = !lensBox.isEmpty() && lensBox.getCenter(new THREE.Vector3()).y < center.y - 1;
  const cam = new THREE.PerspectiveCamera(28, 1, radius / 50, radius * 20);
  const dir = new THREE.Vector3(-0.5, below ? -0.55 : 0.25, -1).normalize();
  const dist = radius / Math.sin(THREE.MathUtils.degToRad(14)) / 1.12;
  cam.position.copy(center).addScaledVector(dir, dist);
  cam.lookAt(center);
  const key = new THREE.DirectionalLight(0xffffff, 1.6);
  key.position.copy(center).add(new THREE.Vector3(-radius * 3, radius * 3, -radius * 2));
  scene.add(key);
  try {
    s.renderer.render(scene, cam);
    return s.renderer.domElement.toDataURL("image/png");
  } catch {
    return null;
  } finally {
    for (const m of meshes) m.geometry.dispose();
    for (const m of Object.values(mats)) m.dispose();
  }
}

/** Furniture or equipment (ADR-090): its parts in their finishes on a light studio floor,
 * seen from the front (model -y) at three-quarters and above. */
function renderFfe(t: FfeThumb): string | null {
  let s;
  try {
    s = setup();
  } catch {
    return null;
  }
  const scene = new THREE.Scene();
  scene.environment = s.env;
  scene.environmentIntensity = 0.6;
  scene.background = new THREE.Color(0xeceae6);
  const meshes: THREE.Mesh[] = [];
  const mats: THREE.Material[] = [];
  for (const p of t.parts) {
    const [r, , b] = p.color;
    const metal = r > 160 && r < 190 && Math.abs(r - b) < 12;
    const glass = r < 60 && b > r + 15;
    const mat = new THREE.MeshStandardMaterial({
      color: srgb(p.color),
      roughness: glass ? 0.08 : metal ? 0.3 : 0.75,
      metalness: metal ? 0.85 : glass ? 0.3 : 0,
    });
    mats.push(mat);
    const m = new THREE.Mesh(geometry(p.positions), mat);
    m.castShadow = true;
    m.receiveShadow = true;
    scene.add(m);
    meshes.push(m);
  }
  const box = new THREE.Box3();
  for (const m of meshes) box.expandByObject(m);
  if (box.isEmpty()) return null;
  const center = box.getCenter(new THREE.Vector3());
  const size = box.getSize(new THREE.Vector3());
  const radius = Math.max(size.length() / 2, 1);
  const floor = new THREE.Mesh(
    new THREE.PlaneGeometry(radius * 12, radius * 12).rotateX(-Math.PI / 2),
    new THREE.ShadowMaterial({ opacity: 0.18 }),
  );
  floor.position.y = box.min.y;
  floor.receiveShadow = true;
  scene.add(floor);
  // Model -y (the front) is three.js +z.
  const cam = new THREE.PerspectiveCamera(28, 1, radius / 50, radius * 20);
  const dir = new THREE.Vector3(-0.55, 0.5, 1).normalize();
  const dist = radius / Math.sin(THREE.MathUtils.degToRad(14)) / 0.92;
  cam.position.copy(center).addScaledVector(dir, dist);
  cam.lookAt(center);
  const key = new THREE.DirectionalLight(0xfff4e8, 2.2);
  key.position.copy(center).add(new THREE.Vector3(-radius * 2.5, radius * 4, radius * 2));
  key.target.position.copy(center);
  key.castShadow = true;
  key.shadow.mapSize.set(1024, 1024);
  const sc = key.shadow.camera as THREE.OrthographicCamera;
  sc.left = sc.bottom = -radius * 2;
  sc.right = sc.top = radius * 2;
  sc.near = radius * 0.1;
  sc.far = radius * 12;
  key.shadow.bias = -0.0005;
  key.shadow.radius = 4;
  scene.add(key, key.target);
  try {
    s.renderer.render(scene, cam);
    return s.renderer.domElement.toDataURL("image/png");
  } catch {
    return null;
  } finally {
    for (const m of meshes) m.geometry.dispose();
    for (const m of mats) m.dispose();
    floor.geometry.dispose();
    (floor.material as THREE.Material).dispose();
    key.shadow.map?.dispose();
  }
}

/** A cached, queued thumbnail for `key`, drawn from the triangles `load` fetches. */
export function thumbnail(
  key: string,
  load: () => Promise<OpeningThumb | FixtureThumb | FfeThumb>,
): Promise<string | null> {
  let p = cache.get(key);
  if (!p) {
    p = queue.then(async () => renderThumb(await load()));
    queue = p.catch(() => {});
    p.catch(() => cache.delete(key));
    cache.set(key, p);
  }
  return p;
}

/** Forgets thumbnails (types were edited). */
export function forgetThumbs(prefix: string) {
  for (const k of [...cache.keys()]) if (k.startsWith(prefix)) cache.delete(k);
}
