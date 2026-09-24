import { useEffect, useRef } from "react";
import * as THREE from "three";
import { OrbitControls } from "three/examples/jsm/controls/OrbitControls.js";
import { errorMessage, ipc, type Mesh } from "../ipc";
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
  selected: 0x3ecff7,
  edge: 0x1c1c1c,
};

function material(m: Mesh) {
  const color =
    m.category === "Wall"
      ? m.exterior
        ? COLORS.exteriorWall
        : COLORS.interiorWall
      : m.category === "Floor"
        ? COLORS.floor
        : m.category === "Door"
          ? COLORS.door
          : m.category === "Window"
            ? COLORS.glass
            : m.category === "Roof"
              ? COLORS.roof
              : m.category === "Stair"
                ? COLORS.stair
                : COLORS.ceiling;
  const seeThrough = m.category === "Ceiling" || m.category === "Window";
  return new THREE.MeshLambertMaterial({
    color,
    transparent: seeThrough,
    opacity: m.category === "Ceiling" ? 0.55 : m.category === "Window" ? 0.45 : 1,
    polygonOffset: true,
    polygonOffsetFactor: 1,
    polygonOffsetUnits: 1,
  });
}

/** 3D view. Geometry comes from Rust as triangle soup in mm, z-up. */
export function View3D() {
  const revision = useAppStore((s) => s.app?.revision ?? 0);
  const selection = useAppStore((s) => s.selection);
  const wrapRef = useRef<HTMLDivElement>(null);
  const three = useRef<{
    renderer: THREE.WebGLRenderer;
    scene: THREE.Scene;
    camera: THREE.PerspectiveCamera;
    controls: OrbitControls;
    group: THREE.Group;
    fitted: boolean;
  } | null>(null);

  useEffect(() => {
    const wrap = wrapRef.current;
    if (!wrap) return;
    const renderer = new THREE.WebGLRenderer({ antialias: true });
    renderer.setPixelRatio(window.devicePixelRatio || 1);
    renderer.setClearColor(0xf8f8f6);
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
    three.current = { renderer, scene, camera, controls, group, fitted: false };

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

    // Click (without dragging) selects the element under the cursor.
    let down: [number, number] | null = null;
    const onDown = (e: PointerEvent) => (down = [e.clientX, e.clientY]);
    const onUp = (e: PointerEvent) => {
      if (!down || Math.hypot(e.clientX - down[0], e.clientY - down[1]) > 4 || e.button !== 0)
        return;
      const r = renderer.domElement.getBoundingClientRect();
      const ndc = new THREE.Vector2(
        ((e.clientX - r.left) / r.width) * 2 - 1,
        -((e.clientY - r.top) / r.height) * 2 + 1,
      );
      const ray = new THREE.Raycaster();
      ray.setFromCamera(ndc, camera);
      const hit = ray.intersectObjects(
        group.children.filter((c) => c instanceof THREE.Mesh),
        false,
      )[0];
      useAppStore.getState().select(hit ? [hit.object.userData.el as string] : []);
    };
    renderer.domElement.addEventListener("pointerdown", onDown);
    renderer.domElement.addEventListener("pointerup", onUp);
    useAppStore
      .getState()
      .setPrompt("Drag to orbit, right-drag to pan, scroll to zoom. Click to select.");

    return () => {
      cancelAnimationFrame(raf);
      ro.disconnect();
      controls.dispose();
      renderer.dispose();
      renderer.domElement.remove();
      three.current = null;
    };
  }, []);

  useEffect(() => {
    let live = true;
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
        for (const m of meshes) {
          const geo = new THREE.BufferGeometry();
          geo.setAttribute("position", new THREE.Float32BufferAttribute(m.positions, 3));
          geo.computeVertexNormals();
          const mesh = new THREE.Mesh(geo, material(m));
          mesh.userData = {
            el: m.el,
            base: (mesh.material as THREE.MeshLambertMaterial).color.getHex(),
          };
          t.group.add(mesh);
          const edges = new THREE.LineSegments(
            new THREE.EdgesGeometry(geo, 25),
            new THREE.LineBasicMaterial({ color: COLORS.edge }),
          );
          t.group.add(edges);
        }
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
  }, [revision]);

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
