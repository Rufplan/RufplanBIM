import { useEffect, useRef } from "react";
import * as THREE from "three";
import { OrbitControls } from "three/examples/jsm/controls/OrbitControls.js";
import type { CameraPose } from "../bindings/CameraPose";
import type { SectionBox } from "../bindings/SectionBox";
import { apply } from "../fileActions";
import { errorMessage, ipc, type Mesh, type Pt, type ViewInfo } from "../ipc";
import { siteImagery, uvAt, type Imagery } from "../imagery";
import { drawOptions, editBoundary, filletRadius } from "../sketch";
import { useAppStore } from "../store";
import { samePt, sketchPrompt } from "../tools";

const COLORS = {
  exteriorWall: 0xe9e7e2,
  interiorWall: 0xf7f7f5,
  floor: 0xb9b9b4,
  ceiling: 0xd7f3fc,
  door: 0x8c7b68,
  glass: 0x9fe3f7,
  roof: 0x5a5f66,
  stair: 0xc9c2b6,
  column: 0xa9a49a,
  steel: 0x6d7b86,
  railing: 0x3a3d40,
  selected: 0x3ecff7,
  edge: 0x1c1c1c,
  box: 0x1aa7d4,
};

function categoryColor(m: Mesh): number {
  switch (m.category) {
    case "Wall":
      return m.exterior ? COLORS.exteriorWall : COLORS.interiorWall;
    case "Floor":
      return COLORS.floor;
    case "Door":
      return COLORS.door;
    case "Window":
      return COLORS.glass;
    case "Roof":
      return COLORS.roof;
    case "Stair":
      return COLORS.stair;
    case "Column":
      return m.exterior ? COLORS.column : COLORS.exteriorWall;
    case "Beam":
      return COLORS.steel;
    case "Railing":
      return COLORS.railing;
    default:
      return COLORS.ceiling;
  }
}

/** Shaded color: the element's material (ADR-020) when it has one, else by category. */
export function meshColor(m: Mesh): number {
  if (m.color && m.category !== "Window" && m.category !== "Ceiling") {
    const [r, g, b] = m.color;
    return (r << 16) | (g << 8) | b;
  }
  return categoryColor(m);
}

function material(m: Mesh, planes: THREE.Plane[]) {
  const seeThrough = m.category === "Ceiling" || m.category === "Window";
  return new THREE.MeshLambertMaterial({
    color: meshColor(m),
    transparent: seeThrough,
    opacity: m.category === "Ceiling" ? 0.55 : m.category === "Window" ? 0.45 : 1,
    polygonOffset: true,
    polygonOffsetFactor: 1,
    polygonOffsetUnits: 1,
    clippingPlanes: planes,
  });
}

/** The six planes keeping what's inside a section box (three.js clips negative distances). */
export function boxPlanes(b: SectionBox | null): THREE.Plane[] {
  if (!b) return [];
  const planes: THREE.Plane[] = [];
  for (let i = 0; i < 3; i++) {
    const n = new THREE.Vector3(i === 0 ? 1 : 0, i === 1 ? 1 : 0, i === 2 ? 1 : 0);
    planes.push(new THREE.Plane(n.clone(), -b.min[i]!));
    planes.push(new THREE.Plane(n.clone().negate(), b.max[i]!));
  }
  return planes;
}

/**
 * Where a drag ray passes closest to the axis through `at` along axis `axis` (0 x, 1 y,
 * 2 z): the new coordinate for a section box face.
 */
export function dragCoordinate(
  rayOrigin: THREE.Vector3,
  rayDir: THREE.Vector3,
  at: THREE.Vector3,
  axis: number,
): number {
  const a = new THREE.Vector3(axis === 0 ? 1 : 0, axis === 1 ? 1 : 0, axis === 2 ? 1 : 0);
  const d = rayDir.clone().normalize();
  const w0 = at.clone().sub(rayOrigin);
  const b = a.dot(d);
  const denom = 1 - b * b;
  if (Math.abs(denom) < 1e-6) return at.getComponent(axis);
  const s = (b * d.dot(w0) - a.dot(w0)) / denom;
  return at.getComponent(axis) + s;
}

interface Three {
  renderer: THREE.WebGLRenderer;
  scene: THREE.Scene;
  camera: THREE.PerspectiveCamera;
  controls: OrbitControls;
  group: THREE.Group;
  boxGroup: THREE.Group;
  gridGroup: THREE.Group;
  fitted: boolean;
}

/** Wireframe and six face handles of the section box. */
function drawBox(t: Three, b: SectionBox | null) {
  for (const child of [...t.boxGroup.children]) {
    t.boxGroup.remove(child);
    if (child instanceof THREE.Mesh || child instanceof THREE.LineSegments) {
      child.geometry.dispose();
      (child.material as THREE.Material).dispose();
    }
  }
  if (!b) return;
  const min = new THREE.Vector3(...b.min);
  const max = new THREE.Vector3(...b.max);
  const box = new THREE.Box3(min, max);
  const edges = new THREE.LineSegments(
    new THREE.EdgesGeometry(new THREE.BoxGeometry(...box.getSize(new THREE.Vector3()).toArray())),
    new THREE.LineBasicMaterial({ color: COLORS.box }),
  );
  edges.position.copy(box.getCenter(new THREE.Vector3()));
  t.boxGroup.add(edges);
  const size = box.getSize(new THREE.Vector3()).length() * 0.012;
  const c = box.getCenter(new THREE.Vector3());
  for (let axis = 0; axis < 3; axis++) {
    for (const end of ["min", "max"] as const) {
      const p = c.clone();
      p.setComponent(axis, (end === "min" ? min : max).getComponent(axis));
      const handle = new THREE.Mesh(
        new THREE.SphereGeometry(size, 16, 12),
        new THREE.MeshBasicMaterial({ color: COLORS.box }),
      );
      handle.position.copy(p);
      handle.userData = { axis, end };
      t.boxGroup.add(handle);
    }
  }
}

/** The ground plane's grid (ADR-022): hairline Rufplan cyan, 4' squares with a stronger line
 * every 20', centered on the model. */
export function groundGrid(center: THREE.Vector3, half: number, z: number): THREE.Group {
  const minor = 1219.2;
  const n = Math.ceil(half / minor);
  const cx = Math.round(center.x / (minor * 5)) * minor * 5;
  const cy = Math.round(center.y / (minor * 5)) * minor * 5;
  const lines = (major: boolean) => {
    const pts: number[] = [];
    for (let i = -n; i <= n; i++) {
      if ((i % 5 === 0) !== major) continue;
      const o = i * minor;
      pts.push(cx + o, cy - n * minor, z, cx + o, cy + n * minor, z);
      pts.push(cx - n * minor, cy + o, z, cx + n * minor, cy + o, z);
    }
    const geo = new THREE.BufferGeometry();
    geo.setAttribute("position", new THREE.Float32BufferAttribute(pts, 3));
    const mat = new THREE.LineBasicMaterial({
      color: 0x3ecff7,
      transparent: true,
      opacity: major ? 0.38 : 0.16,
      depthWrite: false,
    });
    return new THREE.LineSegments(geo, mat);
  };
  const g = new THREE.Group();
  g.add(lines(false), lines(true));
  g.renderOrder = -1;
  return g;
}

/** Each open 3D view's camera as last shown, for Render (ADR-027). */
export const liveCameras = new Map<string, CameraPose>();

/** Whether two poses are the same to the millimetre and the degree. */
export function samePose(a: CameraPose, b: CameraPose): boolean {
  const near = (p: number[], q: number[], tol: number) =>
    p.every((v, i) => Math.abs(v - q[i]!) < tol);
  return near(a.eye, b.eye, 1) && near(a.target, b.target, 1) && Math.abs(a.fov - b.fov) < 0.5;
}

/** The floor plan of a level (3D tools place through it, ADR-022). */
function planOf(level: string | null | undefined): string | null {
  const app = useAppStore.getState().app;
  return (
    app?.views.find((v) => v.viewType === "Plan" && v.level === level && v.calloutOf === null)
      ?.id ?? null
  );
}

/** The status-bar prompt for a tool in 3D (`n`: points placed so far). */
export function prompt3d(tool: string, n = 0): string {
  switch (tool) {
    case "sketch":
      // Typed lengths are for plan views.
      return `${sketchPrompt(useAppStore.getState().sketchUi.mode, n).replace(", or type a length and press Enter", "")} Drawn on the level's work plane.`;
    case "move":
    case "copy":
      return n === 0
        ? `Click the start point to ${tool} from (on an element or the work plane).`
        : `Click the end point to ${tool} to.`;
    case "door":
    case "window":
      return `Hover over a wall and click to place the ${tool}; the face you point at sets which way it faces.`;
    case "wall":
      return "Click the wall's start on the level's work plane (Level in the options bar), then each next point. Esc finishes.";
    case "column":
      return "Click on the level's work plane to place a column.";
    case "floorAuto":
      return "Click a wall: a floor at the outer faces of that level's walls.";
    case "ceilingAuto":
      return "Click a floor inside a room: a ceiling filling that room.";
    case "roof":
      return "Click a wall: a hip roof over that level's walls.";
    case "room":
      return "Click a floor inside an area enclosed by walls to place a room.";
    default:
      return "Drag to orbit, right-drag to pan, scroll to zoom. Click to select. Turn on Section Box in Properties, then drag its handles.";
  }
}

/** 3D view. Geometry comes from Rust as triangle soup in mm, z-up. */
export function View3D({ view }: { view: ViewInfo }) {
  const revision = useAppStore((s) => s.app?.revision ?? 0);
  const selection = useAppStore((s) => s.selection);
  const sectionBox = view.sectionBox;
  const wrapRef = useRef<HTMLDivElement>(null);
  const three = useRef<Three | null>(null);
  // The box as shown (updated live while dragging a handle).
  const boxRef = useRef<SectionBox | null>(sectionBox);
  // The satellite image draped on the ground, when on (ADR-026).
  const imagery = useRef<Imagery | null>(null);

  useEffect(() => {
    const wrap = wrapRef.current;
    if (!wrap) return;
    const renderer = new THREE.WebGLRenderer({ antialias: true });
    renderer.setPixelRatio(window.devicePixelRatio || 1);
    renderer.setClearColor(0xf8f8f6);
    renderer.localClippingEnabled = true;
    wrap.appendChild(renderer.domElement);
    const scene = new THREE.Scene();
    scene.add(new THREE.HemisphereLight(0xffffff, 0xb8b8b0, 2.2));
    const sun = new THREE.DirectionalLight(0xffffff, 1.4);
    sun.position.set(-0.6, -1, 1.4);
    scene.add(sun);
    const camera = new THREE.PerspectiveCamera(40, 1, 50, 1e7);
    camera.up.set(0, 0, 1);
    const controls = new OrbitControls(camera, renderer.domElement);
    controls.enableDamping = true;
    controls.screenSpacePanning = true;
    const group = new THREE.Group();
    scene.add(group);
    const boxGroup = new THREE.Group();
    scene.add(boxGroup);
    // Placement ghosts (a door's box, a wall's rubber band) and the ground grid.
    const ghost = new THREE.Group();
    scene.add(ghost);
    const gridGroup = new THREE.Group();
    scene.add(gridGroup);
    three.current = {
      renderer,
      scene,
      camera,
      controls,
      group,
      boxGroup,
      gridGroup,
      fitted: false,
    };

    // A camera view saves its pose when navigation settles (ADR-027).
    let saveTimer = 0;
    const onCameraChange = () => {
      const pose: CameraPose = {
        eye: camera.position.toArray(),
        target: controls.target.toArray(),
        fov: camera.fov,
      };
      liveCameras.set(view.id, pose);
      const saved = useAppStore.getState().app?.views.find((v) => v.id === view.id)?.camera;
      if (!saved || samePose(saved, pose)) return;
      window.clearTimeout(saveTimer);
      saveTimer = window.setTimeout(() => {
        const now = useAppStore.getState().app?.views.find((v) => v.id === view.id)?.camera;
        if (now && !samePose(now, pose)) void apply(() => ipc.setCameraPose(view.id, pose));
      }, 700);
    };
    controls.addEventListener("change", onCameraChange);

    let raf = 0;
    const loop = () => {
      controls.update();
      renderer.render(scene, camera);
      raf = requestAnimationFrame(loop);
    };
    loop();
    const ro = new ResizeObserver(() => {
      const r = wrap.getBoundingClientRect();
      renderer.setSize(r.width, r.height);
      camera.aspect = r.width / Math.max(r.height, 1);
      camera.updateProjectionMatrix();
    });
    ro.observe(wrap);

    const rayAt = (e: PointerEvent) => {
      const r = renderer.domElement.getBoundingClientRect();
      const ndc = new THREE.Vector2(
        ((e.clientX - r.left) / r.width) * 2 - 1,
        -((e.clientY - r.top) / r.height) * 2 + 1,
      );
      const ray = new THREE.Raycaster();
      ray.setFromCamera(ndc, camera);
      return ray;
    };
    // ---- Placing in 3D (ADR-022) ----
    const clearGhost = () => {
      for (const c of [...ghost.children]) {
        ghost.remove(c);
        if (c instanceof THREE.Mesh || c instanceof THREE.Line) {
          c.geometry.dispose();
          (c.material as THREE.Material).dispose();
        }
      }
    };
    const meshHit = (e: PointerEvent) => {
      const hit = rayAt(e).intersectObjects(
        group.children.filter((c) => c instanceof THREE.Mesh && c.visible),
        false,
      )[0];
      if (!hit) return null;
      const u = hit.object.userData as { el: string; category: string; level: string | null };
      return { ...u, point: hit.point };
    };
    // The work plane of the level picked in the options bar.
    const planeHit = (e: PointerEvent) => {
      const s = useAppStore.getState();
      const levels = s.app?.levels ?? [];
      const level = s.level3d ?? levels[0]?.id ?? null;
      const i = levels.findIndex((l) => l.id === level);
      const z = s.app?.levelElevations[i] ?? 0;
      const q = new THREE.Vector3();
      const ok = rayAt(e).ray.intersectPlane(new THREE.Plane(new THREE.Vector3(0, 0, 1), -z), q);
      return ok ? { point: q, level, z } : null;
    };
    const snapTol = (q: THREE.Vector3) => Math.max(100, camera.position.distanceTo(q) * 0.012);
    let wallFrom: { x: number; y: number } | null = null;
    const addGhostMesh = (positions: number[], valid: boolean) => {
      const geo = new THREE.BufferGeometry();
      geo.setAttribute("position", new THREE.Float32BufferAttribute(positions, 3));
      geo.computeVertexNormals();
      ghost.add(
        new THREE.Mesh(
          geo,
          new THREE.MeshBasicMaterial({
            color: valid ? 0x3ecff7 : 0xc0352b,
            transparent: true,
            opacity: 0.45,
            depthTest: false,
          }),
        ),
      );
    };
    const addGhostLine = (pts: THREE.Vector3[]) => {
      const geo = new THREE.BufferGeometry().setFromPoints(pts);
      ghost.add(
        new THREE.Line(geo, new THREE.LineBasicMaterial({ color: 0x3ecff7, depthTest: false })),
      );
    };
    const addCursor = (q: THREE.Vector3) => {
      const r = Math.max(60, camera.position.distanceTo(q) * 0.006);
      addGhostLine([q.clone().setX(q.x - r), q.clone().setX(q.x + r)]);
      addGhostLine([q.clone().setY(q.y - r), q.clone().setY(q.y + r)]);
    };
    // ---- Boundary sketches in 3D (ADR-025): drawn on the sketch's work plane ----
    const sketchGroup = new THREE.Group();
    scene.add(sketchGroup);
    let skPts: Pt[] = [];
    let skFirst: { i: number; at: Pt } | null = null;
    let skPreview: Pt[][] = [];
    let skCursor: THREE.Vector3 | null = null;
    let vertexDrag: { from: Pt; to: Pt | null } | null = null;
    const skLine = (pts: Pt[], z: number, color: number, opacity = 1) => {
      const geo = new THREE.BufferGeometry().setFromPoints(
        pts.map((q) => new THREE.Vector3(q.x, q.y, z)),
      );
      const line = new THREE.Line(
        geo,
        new THREE.LineBasicMaterial({
          color,
          depthTest: false,
          transparent: opacity < 1,
          opacity,
        }),
      );
      line.renderOrder = 10;
      sketchGroup.add(line);
    };
    /** Ends of the selected boundary lines (their grips). */
    const sketchGrips = (): Pt[] => {
      const s = useAppStore.getState();
      const sk = s.app?.sketch;
      if (!sk || s.sketchUi.mode !== "Modify") return [];
      const out: Pt[] = [];
      for (const i of s.sketchUi.sel) {
        const c = sk.curves[i];
        if (c?.isLine) out.push(c.pts[0]!, c.pts[c.pts.length - 1]!);
      }
      return out;
    };
    const drawSketch3d = () => {
      for (const c of [...sketchGroup.children]) {
        sketchGroup.remove(c);
        if (c instanceof THREE.Mesh || c instanceof THREE.Line) {
          c.geometry.dispose();
          (c.material as THREE.Material).dispose();
        }
      }
      const s = useAppStore.getState();
      const sk = s.app?.sketch;
      if (!sk) return;
      const z = sk.elevation + 5;
      const sel = new Set(s.sketchUi.sel);
      const bad = new Set(sk.bad);
      const vd = vertexDrag;
      sk.curves.forEach((c, i) => {
        const pts = vd?.to
          ? c.pts.map((q) => (Math.hypot(q.x - vd.from.x, q.y - vd.from.y) < 1 ? vd.to! : q))
          : c.pts;
        skLine(pts, z, bad.has(i) ? 0xe0261d : sel.has(i) ? 0x3ecff7 : 0xc832b4);
      });
      for (const pl of skPreview) skLine(pl, z, 0x3ecff7, 0.85);
      if (skCursor) {
        const r = Math.max(60, camera.position.distanceTo(skCursor) * 0.006);
        skLine(
          [
            { x: skCursor.x - r, y: skCursor.y },
            { x: skCursor.x + r, y: skCursor.y },
          ],
          z,
          0x3ecff7,
        );
        skLine(
          [
            { x: skCursor.x, y: skCursor.y - r },
            { x: skCursor.x, y: skCursor.y + r },
          ],
          z,
          0x3ecff7,
        );
        if (skPts.length) skLine([skPts[skPts.length - 1]!, skCursor], z, 0x3ecff7, 0.5);
      }
      for (const g of sketchGrips()) {
        const at = new THREE.Vector3(g.x, g.y, z);
        const grip = new THREE.Mesh(
          new THREE.SphereGeometry(Math.max(40, camera.position.distanceTo(at) * 0.005), 12, 8),
          new THREE.MeshBasicMaterial({ color: 0x3ecff7, depthTest: false }),
        );
        grip.position.copy(at);
        grip.renderOrder = 11;
        sketchGroup.add(grip);
      }
    };
    /** Where a ray meets the sketch's work plane. */
    const sketchPlaneHit = (e: PointerEvent) => {
      const sk = useAppStore.getState().app?.sketch;
      if (!sk) return null;
      const q = new THREE.Vector3();
      const plane = new THREE.Plane(new THREE.Vector3(0, 0, 1), -sk.elevation);
      return rayAt(e).ray.intersectPlane(plane, q) ? q : null;
    };
    /** Pick Walls / Pick Lines: the wall face under the cursor (nudged off it, so the side is
     * clear), else the work plane. */
    const pickCursor = (e: PointerEvent) => {
      const h = rayAt(e).intersectObjects(
        group.children.filter((c) => c instanceof THREE.Mesh && c.visible),
        false,
      )[0];
      if (h && h.object.userData.category === "Wall" && h.face) {
        const n = h.face.normal;
        return { at: { x: h.point.x + n.x * 5, y: h.point.y + n.y * 5 }, q: h.point };
      }
      const q = sketchPlaneHit(e);
      return q ? { at: { x: q.x, y: q.y }, q } : null;
    };
    /** The selected line end under the cursor (within 8 px), to drag. */
    const gripAt3d = (e: PointerEvent): Pt | null => {
      const sk = useAppStore.getState().app?.sketch;
      if (!sk) return null;
      const r = renderer.domElement.getBoundingClientRect();
      for (const g of sketchGrips()) {
        const v = new THREE.Vector3(g.x, g.y, sk.elevation).project(camera);
        const sx = ((v.x + 1) / 2) * r.width + r.left;
        const sy = ((1 - v.y) / 2) * r.height + r.top;
        if (Math.hypot(sx - e.clientX, sy - e.clientY) <= 8) return g;
      }
      return null;
    };
    const sketchHover = async (e: PointerEvent) => {
      const s = useAppStore.getState();
      const sk = s.app?.sketch;
      if (!sk) return;
      const ui = s.sketchUi;
      const m = ui.mode;
      if (vertexDrag) {
        const q = sketchPlaneHit(e);
        if (q) vertexDrag.to = (await ipc.snap(sk.view, { x: q.x, y: q.y }, null, snapTol(q))).pt;
        drawSketch3d();
        return;
      }
      skCursor = null;
      skPreview = [];
      if (m === "PickWalls" || m === "PickLines") {
        const c = pickCursor(e);
        if (c)
          skPreview = await ipc.sketchPreview(
            m,
            [],
            c.at,
            await drawOptions(),
            snapTol(c.q),
            ui.tab,
            ui.core,
          );
      } else if (m !== "Modify" && m !== "Trim" && m !== "FilletArc") {
        const q = sketchPlaneHit(e);
        if (q) {
          const from = skPts[skPts.length - 1] ?? null;
          const sn = await ipc.snap(sk.view, { x: q.x, y: q.y }, from, snapTol(q));
          skCursor = new THREE.Vector3(sn.pt.x, sn.pt.y, sk.elevation);
          s.setCursor(sn.label ?? "");
          if (skPts.length)
            skPreview = await ipc.sketchPreview(
              m,
              skPts,
              sn.pt,
              await drawOptions(),
              snapTol(q),
              ui.tab,
              ui.core,
            );
        }
      }
      drawSketch3d();
    };
    const sketchClick3d = async (e: PointerEvent) => {
      const s = useAppStore.getState();
      const sk = s.app?.sketch;
      if (!sk) return;
      const ui = s.sketchUi;
      const m = ui.mode;
      const offset = async () => (await drawOptions()).offset;
      if (m === "PickWalls" || m === "PickLines") {
        const c = pickCursor(e);
        if (!c) return;
        const tol = snapTol(c.q);
        if (m === "PickWalls") {
          await apply(async () => ipc.sketchPickWalls(c.at, tol, ui.tab, ui.core, await offset()));
          s.setSketchUi({ tab: false });
        } else {
          await apply(async () => ipc.sketchPickLine(c.at, tol, await offset(), ui.lock));
        }
        return;
      }
      const q = sketchPlaneHit(e);
      if (!q) return;
      const raw = { x: q.x, y: q.y };
      const tol = snapTol(q);
      if (m === "Modify") {
        const i = await ipc.sketchHit(raw, tol);
        const shift = e.shiftKey;
        if (i === null) s.setSketchUi({ sel: shift ? ui.sel : [] });
        else if (shift)
          s.setSketchUi({
            sel: ui.sel.includes(i) ? ui.sel.filter((x) => x !== i) : [...ui.sel, i],
          });
        else s.setSketchUi({ sel: [i] });
      } else if (m === "Trim" || m === "FilletArc") {
        const i = await ipc.sketchHit(raw, tol);
        if (i === null) s.setError("Click a boundary line.");
        else if (!skFirst) skFirst = { i, at: raw };
        else {
          const a = skFirst;
          skFirst = null;
          if (m === "Trim") await apply(() => ipc.sketchTrim(a.i, a.at, i, raw));
          else {
            const r = await filletRadius();
            await apply(() => ipc.sketchFillet(a.i, i, r));
          }
        }
      } else {
        const from = skPts[skPts.length - 1] ?? null;
        const pt = (await ipc.snap(sk.view, raw, from, tol)).pt;
        if (from && samePt(from, pt)) return;
        const need = m === "StartEndRadiusArc" || m === "CenterEndsArc" ? 3 : 2;
        const all = [...skPts, pt];
        if (all.length < need) skPts = all;
        else {
          const options = await drawOptions();
          const ok = await apply(() => ipc.sketchDraw(m as never, all, options));
          // Chained lines continue from the end of the last one.
          skPts = ok && m === "Line" && ui.chain ? [pt] : ok ? [] : skPts;
          skPreview = [];
        }
      }
      s.setPrompt(prompt3d("sketch", skFirst ? 1 : skPts.length));
      drawSketch3d();
    };
    const cancelSketch3d = () => {
      const s = useAppStore.getState();
      // Esc ends the current chain, then returns to Modify; it never leaves sketch mode.
      if (skPts.length || skFirst) {
        skPts = [];
        skFirst = null;
      } else if (s.sketchUi.mode !== "Modify") {
        s.setSketchUi({ mode: "Modify" });
      } else {
        s.setSketchUi({ sel: [] });
      }
      skPreview = [];
      skCursor = null;
      s.setPrompt(prompt3d("sketch"));
      drawSketch3d();
    };
    const unsubSketch = useAppStore.subscribe((s, prev) => {
      if (s.sketchUi.mode !== prev.sketchUi.mode) {
        skPts = [];
        skFirst = null;
        skPreview = [];
        if (s.tool === "sketch") s.setPrompt(prompt3d("sketch"));
      }
      if (s.app?.sketch !== prev.app?.sketch || s.sketchUi !== prev.sketchUi) drawSketch3d();
    });
    drawSketch3d();

    // ---- Move and Copy in 3D (ADR-025): two points on a horizontal plane ----
    let moveFrom: { q: THREE.Vector3; plan: string | null } | null = null;
    const movePoint = async (e: PointerEvent) => {
      if (!moveFrom) return null;
      const q = new THREE.Vector3();
      const plane = new THREE.Plane(new THREE.Vector3(0, 0, 1), -moveFrom.q.z);
      if (!rayAt(e).ray.intersectPlane(plane, q)) return null;
      const from = { x: moveFrom.q.x, y: moveFrom.q.y };
      const sn = moveFrom.plan
        ? await ipc.snap(moveFrom.plan, { x: q.x, y: q.y }, from, snapTol(q))
        : null;
      return new THREE.Vector3(sn?.pt.x ?? q.x, sn?.pt.y ?? q.y, moveFrom.q.z);
    };

    let hoverBusy = false;
    let hoverNext: PointerEvent | null = null;
    const hover3d = async (e: PointerEvent) => {
      const s = useAppStore.getState();
      const tool = s.tool;
      if (tool === "door" || tool === "window") {
        const h = meshHit(e);
        const typeId = tool === "door" ? s.toolTypes.door : s.toolTypes.window;
        const pv =
          h?.category === "Wall" && typeId
            ? await ipc.openingPreview3d(typeId, h.el, { x: h.point.x, y: h.point.y })
            : null;
        clearGhost();
        if (pv) addGhostMesh(pv.positions, pv.preview.valid);
      } else if (tool === "wall" || tool === "column") {
        const h = planeHit(e);
        clearGhost();
        if (!h) return;
        const plan = planOf(h.level);
        const sn = plan
          ? await ipc.snap(plan, { x: h.point.x, y: h.point.y }, wallFrom, snapTol(h.point))
          : null;
        const q = new THREE.Vector3(sn?.pt.x ?? h.point.x, sn?.pt.y ?? h.point.y, h.z);
        clearGhost();
        addCursor(q);
        if (tool === "wall" && wallFrom)
          addGhostLine([new THREE.Vector3(wallFrom.x, wallFrom.y, h.z), q]);
        s.setCursor(sn?.label ?? "");
      } else if (tool === "sketch") {
        await sketchHover(e);
      } else if ((tool === "move" || tool === "copy") && moveFrom) {
        const q = await movePoint(e);
        clearGhost();
        if (!q || !moveFrom) return;
        addCursor(q);
        addGhostLine([moveFrom.q, q]);
      } else {
        clearGhost();
      }
    };
    const onHover = (e: PointerEvent) => {
      if (useAppStore.getState().tool === "select") return;
      if (hoverBusy) {
        hoverNext = e;
        return;
      }
      hoverBusy = true;
      void hover3d(e).finally(() => {
        hoverBusy = false;
        const next = hoverNext;
        hoverNext = null;
        if (next) onHover(next);
      });
    };
    const click3d = async (e: PointerEvent) => {
      const s = useAppStore.getState();
      const tool = s.tool;
      if (tool === "sketch") {
        await sketchClick3d(e);
        return;
      }
      if (tool === "move" || tool === "copy") {
        if (s.selection.length === 0) {
          s.setError(
            `Select what to ${tool} first, then choose ${tool === "move" ? "Move" : "Copy"}.`,
          );
          return;
        }
        if (!moveFrom) {
          const h = meshHit(e);
          const w = h ? null : planeHit(e);
          const q = h?.point ?? w?.point;
          if (!q) return;
          moveFrom = { q: q.clone(), plan: planOf(h?.level ?? w?.level) };
          s.setPrompt(prompt3d(tool, 1));
          return;
        }
        const q = await movePoint(e);
        if (!q) return;
        const delta = { x: q.x - moveFrom.q.x, y: q.y - moveFrom.q.y };
        if (Math.hypot(delta.x, delta.y) < 0.5) return;
        const ok =
          tool === "move"
            ? await apply(() => ipc.moveElements(s.selection, delta))
            : await apply(() => ipc.copyElements(s.selection, delta, 1));
        clearGhost();
        if (tool === "copy" && s.options.copyMultiple) return;
        moveFrom = null;
        if (ok) s.setTool("select");
        return;
      }
      if (tool === "door" || tool === "window") {
        const h = meshHit(e);
        const typeId = tool === "door" ? s.toolTypes.door : s.toolTypes.window;
        if (h?.category !== "Wall" || !typeId) {
          s.setError(`Click a wall to place the ${tool}.`);
          return;
        }
        const pv = await ipc.openingPreview3d(typeId, h.el, { x: h.point.x, y: h.point.y });
        if (!pv?.preview.valid) {
          s.setError("That spot overlaps another door or window in this wall.");
          return;
        }
        await apply(() =>
          ipc.createOpening(typeId, pv.preview.host, pv.preview.offset, pv.preview.flipFacing),
        );
        clearGhost();
      } else if (tool === "wall" || tool === "column") {
        const h = planeHit(e);
        const plan = h ? planOf(h.level) : null;
        if (!h || !plan) {
          s.setError("That level has no floor plan to place on.");
          return;
        }
        const sn = await ipc.snap(plan, { x: h.point.x, y: h.point.y }, wallFrom, snapTol(h.point));
        const q = sn.pt;
        if (tool === "column") {
          await apply(() => ipc.createColumn(plan, s.toolTypes.column, q));
        } else if (!wallFrom) {
          wallFrom = q;
        } else if (s.toolTypes.wall) {
          const from = wallFrom;
          if (await apply(() => ipc.createWall(plan, s.toolTypes.wall!, from, q))) wallFrom = q;
        }
      } else {
        const h = meshHit(e);
        const plan = planOf(h?.level);
        if (!h || !plan) {
          s.setError(tool === "roof" || tool === "floorAuto" ? "Click a wall." : "Click a floor.");
          return;
        }
        const at = { x: h.point.x, y: h.point.y };
        if (tool === "floorAuto" && s.toolTypes.floor)
          await apply(() => ipc.createFloor(plan, s.toolTypes.floor!, []));
        else if (tool === "ceilingAuto" && s.toolTypes.ceiling)
          await apply(() => ipc.createCeiling(plan, s.toolTypes.ceiling!, [], at));
        else if (tool === "roof") await apply(() => ipc.createRoof(plan, s.toolTypes.roof));
        else if (tool === "room") await apply(() => ipc.createRoom(plan, at));
      }
    };
    const onCancel = () => {
      const s = useAppStore.getState();
      if (s.tool === "sketch") {
        cancelSketch3d();
        return;
      }
      if (wallFrom) wallFrom = null;
      else if (moveFrom) moveFrom = null;
      else if (s.tool !== "select") s.setTool("select");
      clearGhost();
    };
    window.addEventListener("tool-cancel", onCancel);
    const unsubTool = useAppStore.subscribe((s, prev) => {
      if (s.tool !== prev.tool) {
        wallFrom = null;
        moveFrom = null;
        skPts = [];
        skFirst = null;
        skPreview = [];
        skCursor = null;
        clearGhost();
        s.setPrompt(prompt3d(s.tool));
      }
    });

    // Dragging a section box handle moves that face along its axis.
    let drag: { axis: number; end: "min" | "max"; at: THREE.Vector3 } | null = null;
    let down: [number, number] | null = null;
    const onDown = (e: PointerEvent) => {
      down = [e.clientX, e.clientY];
      // Dragging a selected boundary line's end (sketch Modify).
      if (e.button === 0 && useAppStore.getState().tool === "sketch") {
        const g = gripAt3d(e);
        if (g) {
          vertexDrag = { from: g, to: null };
          controls.enabled = false;
          renderer.domElement.setPointerCapture(e.pointerId);
          return;
        }
      }
      if (e.button !== 0 || !boxRef.current) return;
      const hit = rayAt(e).intersectObjects(
        boxGroup.children.filter((c) => c instanceof THREE.Mesh),
        false,
      )[0];
      if (hit) {
        const { axis, end } = hit.object.userData as { axis: number; end: "min" | "max" };
        drag = { axis, end, at: hit.object.position.clone() };
        controls.enabled = false;
        renderer.domElement.setPointerCapture(e.pointerId);
      }
    };
    const onMove = (e: PointerEvent) => {
      const b = boxRef.current;
      if (!drag || !b) {
        onHover(e);
        return;
      }
      const ray = rayAt(e).ray;
      const v = dragCoordinate(ray.origin, ray.direction, drag.at, drag.axis);
      const next: SectionBox = { min: [...b.min], max: [...b.max] };
      // Keep at least 1' between opposite faces.
      if (drag.end === "min") next.min[drag.axis] = Math.min(v, b.max[drag.axis]! - 304.8);
      else next.max[drag.axis] = Math.max(v, b.min[drag.axis]! + 304.8);
      boxRef.current = next;
      const t = three.current;
      if (t) {
        const planes = boxPlanes(next);
        for (const child of t.group.children) {
          const mats = (child as THREE.Mesh).material as THREE.Material;
          if (mats) mats.clippingPlanes = planes;
        }
        drawBox(t, next);
      }
    };
    const onUp = (e: PointerEvent) => {
      if (vertexDrag) {
        const vd = vertexDrag;
        vertexDrag = null;
        controls.enabled = true;
        if (vd.to && !samePt(vd.from, vd.to))
          void apply(() => ipc.sketchMoveVertex(vd.from, vd.to!));
        drawSketch3d();
        return;
      }
      if (drag) {
        drag = null;
        controls.enabled = true;
        const b = boxRef.current;
        if (b) void apply(() => ipc.setSectionBox(view.id, b.min, b.max));
        return;
      }
      if (!down || Math.hypot(e.clientX - down[0], e.clientY - down[1]) > 4) return;
      if (e.button === 2 && useAppStore.getState().tool !== "select") {
        onCancel();
        return;
      }
      if (e.button !== 0) return;
      if (useAppStore.getState().tool !== "select") {
        void click3d(e).finally(() => useAppStore.getState().setSnapOverride(null));
        return;
      }
      // Click (without dragging) selects the element under the cursor.
      const hit = rayAt(e).intersectObjects(
        group.children.filter((c) => c instanceof THREE.Mesh && c.visible),
        false,
      )[0];
      useAppStore.getState().select(hit ? [hit.object.userData.el as string] : []);
    };
    // Double-click a floor or ceiling: Edit Boundary, as in plans.
    const onDouble = (e: MouseEvent) => {
      if (useAppStore.getState().tool !== "select") return;
      const h = rayAt(e as PointerEvent).intersectObjects(
        group.children.filter((c) => c instanceof THREE.Mesh && c.visible),
        false,
      )[0];
      const u = h?.object.userData as { el: string; category: string } | undefined;
      if (u && (u.category === "Floor" || u.category === "Ceiling")) void editBoundary(u.el);
    };
    renderer.domElement.addEventListener("dblclick", onDouble);
    renderer.domElement.addEventListener("pointerdown", onDown);
    renderer.domElement.addEventListener("pointermove", onMove);
    renderer.domElement.addEventListener("pointerup", onUp);
    useAppStore.getState().setPrompt(prompt3d(useAppStore.getState().tool));

    return () => {
      window.removeEventListener("tool-cancel", onCancel);
      window.clearTimeout(saveTimer);
      controls.removeEventListener("change", onCameraChange);
      renderer.domElement.removeEventListener("dblclick", onDouble);
      unsubTool();
      unsubSketch();
      cancelAnimationFrame(raf);
      ro.disconnect();
      controls.dispose();
      renderer.dispose();
      renderer.domElement.remove();
      three.current = null;
    };
  }, [view.id]);

  useEffect(() => {
    let live = true;
    boxRef.current = sectionBox;
    ipc.meshes(view.id).then(
      (meshes) => {
        const t = three.current;
        if (!live || !t) return;
        for (const child of [...t.group.children]) {
          t.group.remove(child);
          if (child instanceof THREE.Mesh || child instanceof THREE.LineSegments) {
            child.geometry.dispose();
            (child.material as THREE.Material).dispose();
          }
        }
        const planes = boxPlanes(sectionBox);
        for (const m of meshes) {
          const geo = new THREE.BufferGeometry();
          geo.setAttribute("position", new THREE.Float32BufferAttribute(m.positions, 3));
          geo.computeVertexNormals();
          const mesh = new THREE.Mesh(geo, material(m, planes));
          mesh.userData = {
            el: m.el,
            category: m.category,
            level: m.level,
            base: (mesh.material as THREE.MeshLambertMaterial).color.getHex(),
          };
          t.group.add(mesh);
          const edges = new THREE.LineSegments(
            new THREE.EdgesGeometry(geo, 25),
            new THREE.LineBasicMaterial({ color: COLORS.edge, clippingPlanes: planes }),
          );
          t.group.add(edges);
        }
        drawBox(t, sectionBox);
        if (!t.fitted && meshes.length > 0) {
          const box = new THREE.Box3().setFromObject(t.group);
          const c = box.getCenter(new THREE.Vector3());
          const d = box.getSize(new THREE.Vector3()).length();
          t.camera.position.set(c.x - d * 0.95, c.y - d * 1.25, c.z + d * 0.8);
          t.controls.target.copy(c);
          t.fitted = true;
        } else if (!t.fitted) {
          t.camera.position.set(-15000, -20000, 12000);
          t.controls.target.set(6000, 4500, 1500);
        }
        // The ground grid around the model, on the lowest level.
        for (const c of [...t.gridGroup.children]) t.gridGroup.remove(c);
        if (meshes.length > 0) {
          const box = new THREE.Box3().setFromObject(t.group);
          const c = box.getCenter(new THREE.Vector3());
          const half = box.getSize(new THREE.Vector3()).length() * 0.9 + 12000;
          const z0 = Math.min(0, ...(useAppStore.getState().app?.levelElevations ?? [0]));
          t.gridGroup.add(groundGrid(c, half, z0 - 1));
        }
        t.gridGroup.visible = useAppStore.getState().grid3d;
        const st = useAppStore.getState();
        applyImagery(t.group, imagery.current);
        applyDisplay(t.group, st.tempHide[view.id] ?? null, st.visualStyle);
        applySelection(t.group, st.selection);
      },
      (e) => useAppStore.getState().setError(errorMessage(e)),
    );
    return () => {
      live = false;
    };
  }, [revision, sectionBox, view.id]);

  // A camera view looks from its camera; the default 3D view keeps its own orbit.
  const pose = view.camera;
  const poseKey = pose ? JSON.stringify(pose) : "";
  useEffect(() => {
    const t = three.current;
    if (!t || !pose) return;
    const now: CameraPose = {
      eye: t.camera.position.toArray(),
      target: t.controls.target.toArray(),
      fov: t.camera.fov,
    };
    if (t.fitted && samePose(now, pose)) return;
    t.camera.position.set(pose.eye[0]!, pose.eye[1]!, pose.eye[2]!);
    t.controls.target.set(pose.target[0]!, pose.target[1]!, pose.target[2]!);
    t.camera.fov = pose.fov;
    t.camera.updateProjectionMatrix();
    t.fitted = true;
    liveCameras.set(view.id, pose);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [poseKey, view.id]);

  // Temporary Hide/Isolate and the visual style (ADR-024), applied to the meshes shown.
  const temp = useAppStore((s) => s.tempHide[view.id] ?? null);
  const visualStyle = useAppStore((s) => s.visualStyle);
  const sketchTarget = useAppStore((s) => s.app?.sketch?.target ?? null);
  useEffect(() => {
    if (three.current) applyDisplay(three.current.group, temp, visualStyle);
  }, [temp, visualStyle, revision, sketchTarget]);

  useEffect(() => {
    if (three.current) applySelection(three.current.group, selection);
  }, [selection]);

  const satellite = useAppStore((s) => s.satellite);
  const hasSite = useAppStore((s) => !!s.app?.site);
  useEffect(() => {
    let live = true;
    const off = () => {
      imagery.current = null;
      const t = three.current;
      if (!t) return;
      applyImagery(t.group, null);
      const st = useAppStore.getState();
      applyDisplay(t.group, st.tempHide[view.id] ?? null, st.visualStyle);
      applySelection(t.group, st.selection);
    };
    if (!satellite || !hasSite) {
      off();
      return;
    }
    siteImagery().then(
      (im) => {
        const t = three.current;
        if (!live || !t) return;
        imagery.current = im;
        applyImagery(t.group, im);
        const st = useAppStore.getState();
        applyDisplay(t.group, st.tempHide[view.id] ?? null, st.visualStyle);
        applySelection(t.group, st.selection);
      },
      (e) => {
        if (!live) return;
        useAppStore.getState().setError(errorMessage(e));
        useAppStore.getState().setSatellite(false);
      },
    );
    return () => {
      live = false;
    };
  }, [satellite, hasSite, revision, view.id]);
  const setSatellite = useAppStore((s) => s.setSatellite);

  const grid3d = useAppStore((s) => s.grid3d);
  const setGrid3d = useAppStore((s) => s.setGrid3d);
  const tool = useAppStore((s) => s.tool);
  useEffect(() => {
    if (three.current) three.current.gridGroup.visible = grid3d;
  }, [grid3d]);

  return (
    <div ref={wrapRef} className={`canvas-wrap view3d${temp ? " temp-hide" : ""}`} data-tool={tool}>
      <button
        className={`view3d-chip${grid3d ? " on" : ""}`}
        aria-pressed={grid3d}
        onClick={() => setGrid3d(!grid3d)}
        title="Show or hide the ground plane grid"
      >
        Ground Grid
      </button>
      {hasSite && (
        <button
          className={`view3d-chip sat${satellite ? " on" : ""}`}
          aria-pressed={satellite}
          onClick={() => setSatellite(!satellite)}
          title="Drape Google satellite imagery over the topography"
        >
          Satellite
        </button>
      )}
      {hasSite && satellite && <span className="view3d-credit">Imagery ©Google</span>}
    </div>
  );
}

/** Drapes the satellite image on the ground (Site meshes), or takes it off (ADR-026). */
function applyImagery(group: THREE.Group, im: Imagery | null) {
  for (const child of group.children) {
    if (!(child instanceof THREE.Mesh) || child.userData.category !== "Site") continue;
    const mat = child.material as THREE.MeshLambertMaterial;
    const u = child.userData as { base: number; ground?: number };
    mat.map?.dispose();
    if (im) {
      const pos = child.geometry.getAttribute("position");
      const uv = new Float32Array(pos.count * 2);
      for (let i = 0; i < pos.count; i++) {
        const [a, b] = uvAt(im.frame, pos.getX(i), pos.getY(i));
        uv[i * 2] = a;
        uv[i * 2 + 1] = b;
      }
      child.geometry.setAttribute("uv", new THREE.BufferAttribute(uv, 2));
      const tex = new THREE.Texture(im.image);
      tex.colorSpace = THREE.SRGBColorSpace;
      tex.anisotropy = 4;
      tex.needsUpdate = true;
      mat.map = tex;
      u.ground ??= u.base;
      u.base = 0xffffff;
    } else {
      mat.map = null;
      if (u.ground !== undefined) u.base = u.ground;
    }
    mat.color.setHex(u.base);
    mat.needsUpdate = true;
  }
}

/** Temporary Hide/Isolate and the visual style on the shown meshes (ADR-024). */
function applyDisplay(
  group: THREE.Group,
  temp: { isolate: boolean; ids: string[]; categories: string[] } | null,
  visualStyle: string,
) {
  // The floor or ceiling whose boundary is being edited is hidden meanwhile.
  const editing = useAppStore.getState().app?.sketch?.target ?? null;
  let lastMesh: THREE.Mesh | null = null;
  for (const child of group.children) {
    if (child instanceof THREE.Mesh) {
      lastMesh = child;
      const u = child.userData as { el: string; category: string; base: number };
      const hit = temp ? temp.ids.includes(u.el) || temp.categories.includes(u.category) : false;
      child.visible = (!temp || (temp.isolate ? hit : !hit)) && u.el !== editing;
      const mat = child.material as THREE.MeshLambertMaterial;
      mat.wireframe = visualStyle === "wireframe";
      mat.color.setHex(visualStyle === "hiddenLine" ? 0xffffff : u.base);
    } else if (child instanceof THREE.LineSegments && lastMesh) {
      // Each mesh's edges follow it.
      child.visible = lastMesh.visible && visualStyle !== "wireframe";
    }
  }
}

function applySelection(group: THREE.Group, selection: string[]) {
  const sel = new Set(selection);
  for (const child of group.children) {
    if (child instanceof THREE.Mesh) {
      const mat = child.material as THREE.MeshLambertMaterial;
      const base =
        useAppStore.getState().visualStyle === "hiddenLine" ? 0xffffff : child.userData.base;
      mat.color.setHex(sel.has(child.userData.el) ? COLORS.selected : base);
    }
  }
}
