import * as THREE from "three";

// The ViewCube (ADR-037), after Revit's: a labelled cube in the corner of a 3D view that
// turns with the model. Clicking a face, edge or corner (26 hotspots) turns the view to look
// from there and fits the model; dragging the cube orbits; the compass ring under it turns
// the view about the vertical, and its letters look from that side. Home returns to the
// view's home. The cube is drawn by the view's own renderer, in its corner, after the model.

/** Size of the cube's corner, CSS px, and its gap from the view's edges. */
export const CUBE_SIZE = 130;
export const CUBE_GAP = 6;

/** Where a hotspot looks from: -1, 0 or 1 on each axis (x east, y north, z up). */
export type Spot = [number, number, number];

/** How much of a face's half-width is its middle (the rest is edge and corner hotspots). */
const MIDDLE = 0.6;
const ANIM_MS = 420;
/** The direction the home view looks from by default (the view's first fit). */
export const HOME_DIR = new THREE.Vector3(-0.95, -1.25, 0.8).normalize();

const FACES: { label: string; n: Spot; up: Spot; shade: number }[] = [
  { label: "FRONT", n: [0, -1, 0], up: [0, 0, 1], shade: 0.97 },
  { label: "BACK", n: [0, 1, 0], up: [0, 0, 1], shade: 0.9 },
  { label: "RIGHT", n: [1, 0, 0], up: [0, 0, 1], shade: 0.93 },
  { label: "LEFT", n: [-1, 0, 0], up: [0, 0, 1], shade: 0.93 },
  { label: "TOP", n: [0, 0, 1], up: [0, 1, 0], shade: 1 },
  { label: "BOTTOM", n: [0, 0, -1], up: [0, -1, 0], shade: 0.85 },
];

const COMPASS: { letter: string; dir: Spot; side: string }[] = [
  { letter: "N", dir: [0, 1, 0], side: "north" },
  { letter: "E", dir: [1, 0, 0], side: "east" },
  { letter: "S", dir: [0, -1, 0], side: "south" },
  { letter: "W", dir: [-1, 0, 0], side: "west" },
];

/** The hotspot a point on the cube's surface falls in. */
export function spotAt(p: THREE.Vector3): Spot {
  const c = (v: number) => (v > MIDDLE ? 1 : v < -MIDDLE ? -1 : 0);
  return [c(p.x), c(p.y), c(p.z)];
}

/** A hotspot's name, as Revit's tooltips would say it ("Top, Front, Right"). */
export function spotName(s: Spot): string {
  const parts: string[] = [];
  if (s[2]) parts.push(s[2] > 0 ? "Top" : "Bottom");
  if (s[1]) parts.push(s[1] > 0 ? "Back" : "Front");
  if (s[0]) parts.push(s[0] > 0 ? "Right" : "Left");
  return parts.join(", ");
}

/** The direction to look from (target to eye) for a hotspot. Straight up and down lean a
 * hair to the south, so north is up the screen in a top view. */
export function spotDir(s: Spot): THREE.Vector3 {
  const d = new THREE.Vector3(...s);
  if (s[0] === 0 && s[1] === 0) d.y = -1e-4;
  return d.normalize();
}

/** How far from its center to see a sphere of `radius` whole. */
export function fitDistance(radius: number, fovDeg: number, aspect: number): number {
  const v = THREE.MathUtils.degToRad(fovDeg);
  const h = 2 * Math.atan(Math.tan(v / 2) * aspect);
  return (radius / Math.sin(Math.min(v, h) / 2)) * 1.02;
}

/** The axis nearest a direction, as a hotspot. */
export function nearestAxis(v: THREE.Vector3): Spot {
  const a = [Math.abs(v.x), Math.abs(v.y), Math.abs(v.z)];
  const i = a[0]! >= a[1]! && a[0]! >= a[2]! ? 0 : a[1]! >= a[2]! ? 1 : 2;
  const s: Spot = [0, 0, 0];
  s[i] = Math.sign([v.x, v.y, v.z][i]!) || 1;
  return s;
}

/** Looking straight at a face: that face and, for the arrows around the cube, the faces
 * that are up, down, left and right of it on screen. */
export interface Aligned {
  face: Spot;
  up: Spot;
  down: Spot;
  left: Spot;
  right: Spot;
}

export function alignedOf(camera: THREE.Camera, target: THREE.Vector3): Aligned | null {
  const dir = camera.position.clone().sub(target).normalize();
  const face = nearestAxis(dir);
  if (dir.dot(new THREE.Vector3(...face)) < 0.9999) return null;
  const up = nearestAxis(new THREE.Vector3(0, 1, 0).applyQuaternion(camera.quaternion));
  const right = nearestAxis(new THREE.Vector3(1, 0, 0).applyQuaternion(camera.quaternion));
  const neg = (s: Spot): Spot => [-s[0] || 0, -s[1] || 0, -s[2] || 0];
  return { face, up, down: neg(up), left: neg(right), right };
}

interface Anim {
  t0: number;
  from: THREE.Vector3;
  turn: THREE.Quaternion;
  dist0: number;
  dist1: number;
  target0: THREE.Vector3;
  target1: THREE.Vector3;
}

type Hover =
  { kind: "cube"; spot: Spot } | { kind: "ring" } | { kind: "letter"; dir: Spot; side: string };

function faceTexture(label: string): THREE.Texture | null {
  if (typeof document === "undefined") return null;
  const c = document.createElement("canvas");
  c.width = c.height = 256;
  const g = c.getContext("2d");
  if (!g) return null;
  const grad = g.createLinearGradient(0, 0, 0, 256);
  grad.addColorStop(0, "#fbfbfb");
  grad.addColorStop(1, "#dedfe1");
  g.fillStyle = grad;
  g.fillRect(0, 0, 256, 256);
  g.strokeStyle = "#a3a8ad";
  g.lineWidth = 6;
  g.strokeRect(3, 3, 250, 250);
  g.fillStyle = "#4d5257";
  g.font = `bold ${label.length > 5 ? 44 : 52}px Arial, Helvetica, sans-serif`;
  g.textAlign = "center";
  g.textBaseline = "middle";
  g.fillText(label, 128, 132);
  const t = new THREE.CanvasTexture(c);
  t.colorSpace = THREE.SRGBColorSpace;
  t.anisotropy = 4;
  return t;
}

function letterTexture(letter: string): THREE.Texture | null {
  if (typeof document === "undefined") return null;
  const c = document.createElement("canvas");
  c.width = c.height = 64;
  const g = c.getContext("2d");
  if (!g) return null;
  g.fillStyle = "#3f4449";
  g.font = "bold 44px Arial, Helvetica, sans-serif";
  g.textAlign = "center";
  g.textBaseline = "middle";
  g.fillText(letter, 32, 34);
  const t = new THREE.CanvasTexture(c);
  t.colorSpace = THREE.SRGBColorSpace;
  return t;
}

export interface CubeHost {
  camera: THREE.PerspectiveCamera;
  /** The orbit's pivot (the controls' target), moved in place. */
  target: THREE.Vector3;
  /** What "fit" fits: the section box, else the model; null when there's nothing. */
  bounds: () => THREE.Box3 | null;
  /** Home, if this view has one saved. */
  home: () => { eye: THREE.Vector3; target: THREE.Vector3 } | null;
}

export class ViewCube {
  readonly scene = new THREE.Scene();
  readonly cam = new THREE.OrthographicCamera(-2.25, 2.25, 2.25, -2.25, 0.1, 20);
  private faces: THREE.Mesh[] = [];
  private faceMats: THREE.MeshBasicMaterial[] = [];
  private ring: THREE.Mesh;
  private letters: THREE.Sprite[] = [];
  private highlight: THREE.Mesh;
  private edges: THREE.LineSegments;
  private anim: Anim | null = null;
  private hover: Hover | null = null;
  private active = false;
  private opacity = 0.5;
  private aligned: Aligned | null = null;
  private el: HTMLElement | null = null;
  private drag: { x: number; y: number; moved: boolean; hit: Hover | null } | null = null;
  /** Called when looking straight at a face starts, changes or ends. */
  onAligned: (a: Aligned | null) => void = () => {};
  /** Called with the hotspot's name under the pointer, for a tooltip. */
  onHover: (name: string) => void = () => {};

  constructor(readonly host: CubeHost) {
    for (const f of FACES) {
      const n = new THREE.Vector3(...f.n);
      const up = new THREE.Vector3(...f.up);
      const right = up.clone().cross(n);
      const mat = new THREE.MeshBasicMaterial({
        map: faceTexture(f.label),
        color: new THREE.Color(f.shade, f.shade, f.shade),
        transparent: true,
      });
      const m = new THREE.Mesh(new THREE.PlaneGeometry(2, 2), mat);
      new THREE.Matrix4()
        .makeBasis(right, up, n)
        .setPosition(n)
        .decompose(m.position, m.quaternion, m.scale);
      m.name = f.label;
      this.faces.push(m);
      this.faceMats.push(mat);
      this.scene.add(m);
    }
    this.edges = new THREE.LineSegments(
      new THREE.EdgesGeometry(new THREE.BoxGeometry(2.004, 2.004, 2.004)),
      new THREE.LineBasicMaterial({ color: 0x8a9096, transparent: true }),
    );
    this.scene.add(this.edges);
    this.highlight = new THREE.Mesh(
      new THREE.BoxGeometry(1, 1, 1),
      new THREE.MeshBasicMaterial({
        color: 0x5ab0ff,
        transparent: true,
        opacity: 0.55,
        depthWrite: false,
      }),
    );
    this.highlight.visible = false;
    this.scene.add(this.highlight);
    this.ring = new THREE.Mesh(
      new THREE.RingGeometry(1.5, 1.95, 72),
      new THREE.MeshBasicMaterial({
        color: 0xc6cbd0,
        transparent: true,
        side: THREE.DoubleSide,
        depthWrite: false,
      }),
    );
    this.ring.position.z = -1;
    this.scene.add(this.ring);
    for (const c of COMPASS) {
      const s = new THREE.Sprite(
        new THREE.SpriteMaterial({ map: letterTexture(c.letter), transparent: true }),
      );
      s.position.set(c.dir[0] * 1.72, c.dir[1] * 1.72, -1);
      s.scale.set(0.5, 0.5, 1);
      s.userData = { dir: c.dir, side: c.side };
      this.letters.push(s);
      this.scene.add(s);
    }
    this.setOpacity(0.5);
  }

  /** Revit's cube is half see-through until the pointer is over it. */
  private setOpacity(o: number) {
    this.opacity = o;
    for (const m of this.faceMats) m.opacity = o;
    (this.edges.material as THREE.LineBasicMaterial).opacity = o;
    this.fadeCompass();
  }

  /** The compass fades out as the view comes level (seen edge-on it would only clutter the
   * cube's bottom edge). */
  private fadeCompass() {
    const { camera, target } = this.host;
    const z = Math.abs(camera.position.clone().sub(target).normalize().z);
    const k = THREE.MathUtils.clamp((z - 0.08) * 6, 0, 1);
    const show = k > 0.02;
    this.ring.visible = show;
    (this.ring.material as THREE.MeshBasicMaterial).opacity = this.opacity * 0.9 * k;
    for (const s of this.letters) {
      s.visible = show;
      (s.material as THREE.SpriteMaterial).opacity = this.opacity * k;
    }
  }

  /** Advances a turn in progress; call before the controls update each frame. */
  step(now = performance.now()) {
    const a = this.anim;
    if (!a) return;
    const k = Math.min(1, (now - a.t0) / ANIM_MS);
    const e = k < 0.5 ? 4 * k * k * k : 1 - Math.pow(-2 * k + 2, 3) / 2;
    const q = new THREE.Quaternion().slerpQuaternions(new THREE.Quaternion(), a.turn, e);
    const dir = a.from.clone().applyQuaternion(q);
    const dist = a.dist0 + (a.dist1 - a.dist0) * e;
    this.host.target.lerpVectors(a.target0, a.target1, e);
    this.host.camera.position.copy(this.host.target).addScaledVector(dir, dist);
    this.host.camera.lookAt(this.host.target);
    if (k >= 1) this.anim = null;
  }

  /** Follows the face alignment and the hotspot under the pointer; returns an unsubscribe. */
  watch(onAligned: (a: Aligned | null) => void, onHover: (name: string) => void): () => void {
    this.onAligned = onAligned;
    this.onHover = onHover;
    onAligned(this.aligned);
    return () => {
      this.onAligned = () => {};
      this.onHover = () => {};
    };
  }

  /** Stops a turn (the user took over). */
  stop() {
    this.anim = null;
  }

  get turning() {
    return this.anim !== null;
  }

  /** Turns the view to look from `dir`, fitting the model if `fit` (Revit's "Fit-to-view on
   * view change"); `animate` false jumps there. */
  orient(
    dir: THREE.Vector3,
    opts: { fit?: boolean; animate?: boolean; target?: THREE.Vector3; dist?: number } = {},
  ) {
    const { camera, target } = this.host;
    const offset = camera.position.clone().sub(target);
    const from = offset.clone().normalize();
    let target1 = opts.target?.clone() ?? target.clone();
    let dist1 = opts.dist ?? offset.length();
    if (opts.fit !== false && !opts.target) {
      const b = this.host.bounds();
      if (b && !b.isEmpty()) {
        const sphere = b.getBoundingSphere(new THREE.Sphere());
        target1 = sphere.center;
        dist1 = fitDistance(sphere.radius, camera.fov, camera.aspect);
      }
    }
    const to = dir.clone().normalize();
    this.anim = {
      t0: performance.now(),
      from,
      turn: new THREE.Quaternion().setFromUnitVectors(from, to),
      dist0: offset.length(),
      dist1,
      target0: target.clone(),
      target1,
    };
    if (opts.animate === false) this.step(Infinity);
  }

  /** Fits the model without turning (ZF). */
  fit(animate = true) {
    const { camera, target } = this.host;
    this.orient(camera.position.clone().sub(target), { animate });
  }

  /** Goes to the view's home: its saved one, else the view's first fit. */
  goHome(animate = true) {
    const h = this.host.home();
    if (h) {
      const off = h.eye.clone().sub(h.target);
      this.orient(off, { target: h.target, dist: off.length(), animate });
    } else {
      this.orient(HOME_DIR, { animate });
    }
  }

  /** Orbits by a screen drag of (dx, dy) px: about the vertical, and up and down. */
  orbit(dx: number, dy: number, vertical = true) {
    this.anim = null;
    const { camera, target } = this.host;
    const off = camera.position.clone().sub(target);
    off.applyAxisAngle(new THREE.Vector3(0, 0, 1), -dx * 0.012);
    if (vertical) {
      const r = off.length();
      const phi = THREE.MathUtils.clamp(Math.acos(off.z / r) - dy * 0.012, 1e-3, Math.PI - 1e-3);
      const theta = Math.atan2(off.y, off.x);
      off.set(
        r * Math.sin(phi) * Math.cos(theta),
        r * Math.sin(phi) * Math.sin(theta),
        r * Math.cos(phi),
      );
    }
    camera.position.copy(target).add(off);
    camera.lookAt(target);
  }

  /** Draws the cube in the top-right corner of the renderer, over what's there. */
  render(renderer: THREE.WebGLRenderer) {
    const { camera, target } = this.host;
    this.cam.quaternion.copy(camera.quaternion);
    this.cam.position.set(0, 0, 10).applyQuaternion(camera.quaternion);
    this.cam.updateMatrixWorld();
    this.fadeCompass();
    const a = alignedOf(camera, target);
    const key = (x: Aligned | null) => (x ? `${x.face}|${x.up}` : "");
    if (key(a) !== key(this.aligned)) {
      this.aligned = a;
      this.onAligned(a);
    }
    const size = renderer.getSize(new THREE.Vector2());
    const x = size.x - CUBE_SIZE - CUBE_GAP;
    const y = size.y - CUBE_SIZE - CUBE_GAP;
    if (x < 0 || y < 0) return;
    const auto = renderer.autoClear;
    renderer.autoClear = false;
    renderer.setScissorTest(true);
    renderer.setScissor(x, y, CUBE_SIZE, CUBE_SIZE);
    renderer.setViewport(x, y, CUBE_SIZE, CUBE_SIZE);
    renderer.clearDepth();
    renderer.render(this.scene, this.cam);
    renderer.setScissorTest(false);
    renderer.setViewport(0, 0, size.x, size.y);
    renderer.autoClear = auto;
  }

  /** What's under a point in the cube's corner (CSS px within it). */
  pick(px: number, py: number): Hover | null {
    const ndc = new THREE.Vector2((px / CUBE_SIZE) * 2 - 1, -(py / CUBE_SIZE) * 2 + 1);
    this.scene.updateMatrixWorld();
    this.cam.updateMatrixWorld();
    const ray = new THREE.Raycaster();
    ray.setFromCamera(ndc, this.cam);
    const compass = this.ring.visible ? [...this.letters, this.ring] : [];
    const hits = ray.intersectObjects([...this.faces, ...compass], false);
    const h = hits[0];
    if (!h) return null;
    if (this.faces.includes(h.object as THREE.Mesh)) return { kind: "cube", spot: spotAt(h.point) };
    if (h.object instanceof THREE.Sprite) {
      const u = h.object.userData as { dir: Spot; side: string };
      return { kind: "letter", dir: u.dir, side: u.side };
    }
    return { kind: "ring" };
  }

  private setHover(h: Hover | null) {
    this.hover = h;
    const hl = this.highlight;
    const ringMat = this.ring.material as THREE.MeshBasicMaterial;
    ringMat.color.set(h && h.kind !== "cube" ? 0x9fcdf5 : 0xc6cbd0);
    if (h?.kind === "cube") {
      const span = (s: number): [number, number] =>
        s > 0 ? [MIDDLE, 1.006] : s < 0 ? [-1.006, -MIDDLE] : [-MIDDLE, MIDDLE];
      const [x, y, z] = h.spot.map(span) as [number, number][];
      hl.scale.set(x![1] - x![0], y![1] - y![0], z![1] - z![0]);
      hl.position.set((x![0] + x![1]) / 2, (y![0] + y![1]) / 2, (z![0] + z![1]) / 2);
      hl.visible = true;
      this.onHover(spotName(h.spot));
    } else {
      hl.visible = false;
      this.onHover(
        h?.kind === "letter"
          ? `Look from the ${h.side}`
          : h?.kind === "ring"
            ? "Drag to rotate the view"
            : "",
      );
    }
    if (this.el) this.el.style.cursor = h ? "pointer" : "default";
  }

  /** Clicks a hotspot or compass letter. */
  click(h: Hover) {
    if (h.kind === "cube") this.orient(spotDir(h.spot));
    else if (h.kind === "letter") {
      // Look from that side at the same height.
      const { camera, target } = this.host;
      const off = camera.position.clone().sub(target).normalize();
      const el = Math.asin(THREE.MathUtils.clamp(off.z, -1, 1));
      const d = new THREE.Vector3(h.dir[0], h.dir[1], 0).multiplyScalar(Math.cos(el));
      d.z = Math.sin(el);
      this.orient(d);
    }
  }

  /** Takes pointer input from the element over the cube's corner. */
  attach(el: HTMLElement): () => void {
    this.el = el;
    const local = (e: PointerEvent) => {
      const r = el.getBoundingClientRect();
      return [e.clientX - r.left, e.clientY - r.top] as const;
    };
    const enter = () => {
      this.active = true;
      this.setOpacity(1);
    };
    const leave = () => {
      if (this.drag) return;
      this.active = false;
      this.setOpacity(0.5);
      this.setHover(null);
    };
    const down = (e: PointerEvent) => {
      if (e.button !== 0) return;
      const [x, y] = local(e);
      this.drag = { x: e.clientX, y: e.clientY, moved: false, hit: this.pick(x, y) };
      el.setPointerCapture?.(e.pointerId);
      e.preventDefault();
    };
    const move = (e: PointerEvent) => {
      const d = this.drag;
      if (d) {
        const dx = e.clientX - d.x;
        const dy = e.clientY - d.y;
        if (!d.moved && Math.hypot(dx, dy) < 3) return;
        d.moved = true;
        this.orbit(dx, dy, d.hit?.kind === "cube" || !d.hit);
        d.x = e.clientX;
        d.y = e.clientY;
        return;
      }
      const [x, y] = local(e);
      const h = this.pick(x, y);
      if (JSON.stringify(h) !== JSON.stringify(this.hover)) this.setHover(h);
    };
    const up = (e: PointerEvent) => {
      const d = this.drag;
      this.drag = null;
      el.releasePointerCapture?.(e.pointerId);
      if (d && !d.moved && d.hit) this.click(d.hit);
      if (!this.active) this.setOpacity(0.5);
    };
    el.addEventListener("pointerenter", enter);
    el.addEventListener("pointerleave", leave);
    el.addEventListener("pointerdown", down);
    el.addEventListener("pointermove", move);
    el.addEventListener("pointerup", up);
    return () => {
      el.removeEventListener("pointerenter", enter);
      el.removeEventListener("pointerleave", leave);
      el.removeEventListener("pointerdown", down);
      el.removeEventListener("pointermove", move);
      el.removeEventListener("pointerup", up);
      this.el = null;
    };
  }

  dispose() {
    this.scene.traverse((o) => {
      if (o instanceof THREE.Mesh || o instanceof THREE.LineSegments || o instanceof THREE.Sprite) {
        o.geometry?.dispose();
        const m = o.material as THREE.Material & { map?: THREE.Texture | null };
        m.map?.dispose();
        m.dispose();
      }
    });
  }
}

/** Home per view, kept on this computer (the model file is unchanged). */
const HOME_KEY = (viewId: string) => `rufplan.viewcube.home.${viewId}`;

export function savedHome(viewId: string): { eye: THREE.Vector3; target: THREE.Vector3 } | null {
  try {
    const raw = localStorage.getItem(HOME_KEY(viewId));
    if (!raw) return null;
    const h = JSON.parse(raw) as { eye: number[]; target: number[] };
    return {
      eye: new THREE.Vector3().fromArray(h.eye),
      target: new THREE.Vector3().fromArray(h.target),
    };
  } catch {
    return null;
  }
}

export function saveHome(viewId: string, eye: THREE.Vector3 | null, target?: THREE.Vector3) {
  try {
    if (!eye || !target) localStorage.removeItem(HOME_KEY(viewId));
    else
      localStorage.setItem(
        HOME_KEY(viewId),
        JSON.stringify({ eye: eye.toArray(), target: target.toArray() }),
      );
  } catch {
    // Storage off: home stays the first fit.
  }
}
