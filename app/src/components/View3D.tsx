import { useEffect, useRef } from "react";
import * as THREE from "three";
import { OrbitControls } from "three/examples/jsm/controls/OrbitControls.js";
import type { SectionBox } from "../bindings/SectionBox";
import { apply } from "../fileActions";
import { errorMessage, ipc, type Mesh, type ViewInfo } from "../ipc";
import { useAppStore } from "../store";

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

/** 3D view. Geometry comes from Rust as triangle soup in mm, z-up. */
export function View3D({ view }: { view: ViewInfo }) {
  const revision = useAppStore((s) => s.app?.revision ?? 0);
  const selection = useAppStore((s) => s.selection);
  const sectionBox = view.sectionBox;
  const wrapRef = useRef<HTMLDivElement>(null);
  const three = useRef<Three | null>(null);
  // The box as shown (updated live while dragging a handle).
  const boxRef = useRef<SectionBox | null>(sectionBox);

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
    three.current = { renderer, scene, camera, controls, group, boxGroup, fitted: false };

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
    // Dragging a section box handle moves that face along its axis.
    let drag: { axis: number; end: "min" | "max"; at: THREE.Vector3 } | null = null;
    let down: [number, number] | null = null;
    const onDown = (e: PointerEvent) => {
      down = [e.clientX, e.clientY];
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
      if (!drag || !b) return;
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
      if (drag) {
        drag = null;
        controls.enabled = true;
        const b = boxRef.current;
        if (b) void apply(() => ipc.setSectionBox(view.id, b.min, b.max));
        return;
      }
      if (!down || Math.hypot(e.clientX - down[0], e.clientY - down[1]) > 4 || e.button !== 0)
        return;
      // Click (without dragging) selects the element under the cursor.
      const hit = rayAt(e).intersectObjects(
        group.children.filter((c) => c instanceof THREE.Mesh),
        false,
      )[0];
      useAppStore.getState().select(hit ? [hit.object.userData.el as string] : []);
    };
    renderer.domElement.addEventListener("pointerdown", onDown);
    renderer.domElement.addEventListener("pointermove", onMove);
    renderer.domElement.addEventListener("pointerup", onUp);
    useAppStore
      .getState()
      .setPrompt(
        "Drag to orbit, right-drag to pan, scroll to zoom. Click to select. Turn on Section Box in Properties, then drag its handles.",
      );

    return () => {
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
    ipc.meshes().then(
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
        applySelection(t.group, useAppStore.getState().selection);
      },
      (e) => useAppStore.getState().setError(errorMessage(e)),
    );
    return () => {
      live = false;
    };
  }, [revision, sectionBox]);

  useEffect(() => {
    if (three.current) applySelection(three.current.group, selection);
  }, [selection]);

  return <div ref={wrapRef} className="canvas-wrap view3d" />;
}

function applySelection(group: THREE.Group, selection: string[]) {
  const sel = new Set(selection);
  for (const child of group.children) {
    if (child instanceof THREE.Mesh) {
      const mat = child.material as THREE.MeshLambertMaterial;
      mat.color.setHex(sel.has(child.userData.el) ? COLORS.selected : child.userData.base);
    }
  }
}
