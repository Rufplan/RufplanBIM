import { useEffect, useRef, useState } from "react";
import type { PickCandidate } from "../bindings/PickCandidate";
import { paintElement } from "../actions";
import * as THREE from "three";
import { OrbitControls } from "three/examples/jsm/controls/OrbitControls.js";
import type { CameraPose } from "../bindings/CameraPose";
import type { SectionBox } from "../bindings/SectionBox";
import { apply } from "../fileActions";
import { errorMessage, ipc, type Mesh, type Pt, type ViewInfo } from "../ipc";
import { siteImagery, uvAt, type Imagery } from "../imagery";
import {
  drawOptions,
  editBoundary,
  filletRadius,
  editWallOpening,
  startWallOpening,
} from "../sketch";
import { buildStructural3d, disposeGroup, ghostArchitecture } from "../render/structural";
import { StructuralInfoCard, StructuralLegend, type InfoAt } from "./StructuralOverlay";
import type { Discipline } from "../bindings/Discipline";
import type { OverlayMesh } from "../bindings/OverlayMesh";
import { litOf, styleOf, useAppStore } from "../store";
import { alignDelta, canAlign, faceAt, faceHighlight, type AlignFace } from "../render/align3d";
import { placePlant } from "../vegetation";
import { samePt, sketchPrompt } from "../tools";
import { savedHome, ViewCube } from "../render/viewCube";
import { ViewCubeOverlay } from "./ViewCubeOverlay";
import { VisualStyleToggle } from "./VisualStyleToggle";
import {
  EDGE_COLOR,
  isGlass,
  realMaterials,
  realUv,
  SELECTED,
  skyTexture,
  styleMaterial,
  type MeshInfo,
  type RealMaterial,
  type VisualStyle,
} from "../render/visualStyle";
import { RoomEnvironment } from "three/examples/jsm/environments/RoomEnvironment.js";
import { LineSegments2 } from "three/examples/jsm/lines/LineSegments2.js";
import { LineSegmentsGeometry } from "three/examples/jsm/lines/LineSegmentsGeometry.js";
import { EffectComposer } from "three/examples/jsm/postprocessing/EffectComposer.js";
import { RenderPass } from "three/examples/jsm/postprocessing/RenderPass.js";
import { GTAOPass } from "three/examples/jsm/postprocessing/GTAOPass.js";
import { OutputPass } from "three/examples/jsm/postprocessing/OutputPass.js";
import { ShaderPass } from "three/examples/jsm/postprocessing/ShaderPass.js";
import { UnrealBloomPass } from "three/examples/jsm/postprocessing/UnrealBloomPass.js";
import { fromLook, look, lookDir, navStep, type NavState } from "../render/navigate";
import { NavBar, SunPanel } from "./View3DPanels";
import type { LightInfo } from "../bindings/LightInfo";
import { LineMaterial } from "three/examples/jsm/lines/LineMaterial.js";
import type { Cap } from "../bindings/Cap";
import type { Terrain } from "../bindings/Terrain";
import { TerrainBar } from "./TerrainBar";
import { loadPlantEntries, plantGroup, wind } from "../render/plants";
import { grassGroup, type GrassSurface } from "../render/grass";
import { plantLoader } from "./AssetLibrary";

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
  // A door or window frame mesh carries its finish colour; its glass has none (ADR-031).
  if ((m.category === "Window" || m.category === "Door") && !m.color) return COLORS.glass;
  if (m.color && m.category !== "Ceiling") {
    const [r, g, b] = m.color;
    return (r << 16) | (g << 8) | b;
  }
  return categoryColor(m);
}

/** The orbit center for a view that should zoom and orbit about `point` (ADR-043): on the
 * camera's line of sight (so the view doesn't move) at `point`'s depth. Null when the point
 * is behind or at the camera. */
export function pivotAt(
  camera: THREE.PerspectiveCamera,
  point: THREE.Vector3,
): THREE.Vector3 | null {
  const fwd = camera.getWorldDirection(new THREE.Vector3());
  const depth = point.clone().sub(camera.position).dot(fwd);
  if (!(depth > camera.near * 4)) return null;
  return camera.position.clone().addScaledVector(fwd, depth);
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
  /** The section box's caps (ADR-044). */
  capGroup: THREE.Group;
  /** The site's earth block, contours and labels (ADR-045). */
  terrainGroup: THREE.Group;
  gridGroup: THREE.Group;
  fitted: boolean;
  hemi: THREE.HemisphereLight;
  sun: THREE.DirectionalLight;
  /** Realistic's ground and sky, and its image-based light (made on first use). */
  ground: THREE.Mesh;
  sky: THREE.Texture | null;
  env: THREE.Texture | null;
  /** Realistic's sun from Sun Settings (ADR-062): toward it, its strength and colour. */
  sunDir?: THREE.Vector3;
  sunIntensity?: number;
  sunColor?: THREE.Color;
  /** Realistic's ambient occlusion, bloom and antialiasing (made on first use). */
  composer?: EffectComposer | null;
  /** The building's lit fixtures, after dark (ADR-063). */
  fixtures?: THREE.Group;
  /** After dark: the exposure raised for the fixtures. */
  night?: boolean;
  /** Enscape's auto exposure: scales the sun and sky's light to a steady brightness. */
  autoExposure?: number;
  /** Realistic's plants and grass (ADR-064), on layer 1 (left out of ambient occlusion,
   * whose normal pass can't cut leaves out of their cards). */
  plants?: THREE.Group;
  grass?: THREE.Group;
  aoCamera?: THREE.PerspectiveCamera;
  /** The base ground's material on Realistic's ground, and what it was made from. */
  groundKey?: string;
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
    case "paint":
      return "Click a face of a wall, floor, roof or ceiling to paint it. Shift-click paints the whole assembly (its type). Esc finishes.";
    case "align":
      return n === 0
        ? "Align: pick the face to align to (a wall face, a floor edge or top, a column side…)."
        : "Pick a parallel face on the element to move onto it. Esc picks a new reference.";
    case "wall":
      return "Click the wall's start on the level's work plane (Level in the options bar), then each next point. Esc finishes.";
    case "column":
      return "Click on the level's work plane to place a column.";
    case "light":
      return "Click a ceiling, wall or floor to place the lighting fixture on it.";
    case "grassBrush":
      return "Drag over the ground, a floor or a roof to paint grass; Erase on the options bar takes it away. Esc finishes.";
    case "plant":
      return "Click the ground, a floor or a roof to place the plant; keep clicking to place more. Esc finishes.";
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
/** D5's colour (ADR-065): saturation and contrast on the finished image. */
export const D5_SATURATION = 1.14;
export const D5_CONTRAST = 1.06;
const D5_GRADE = {
  uniforms: {
    tDiffuse: { value: null },
    saturation: { value: D5_SATURATION },
    contrast: { value: D5_CONTRAST },
  },
  vertexShader: `varying vec2 vUv;
    void main() { vUv = uv; gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0); }`,
  fragmentShader: `uniform sampler2D tDiffuse; uniform float saturation; uniform float contrast;
    varying vec2 vUv;
    void main() {
      vec4 c = texture2D(tDiffuse, vUv);
      float l = dot(c.rgb, vec3(0.2126, 0.7152, 0.0722));
      vec3 s = mix(vec3(l), c.rgb, saturation);
      gl_FragColor = vec4(clamp((s - 0.5) * contrast + 0.5, 0.0, 1.0), c.a);
    }`,
};

/** D5's grass kinds and their looks, fetched once for the brush. */
let grassKinds: Promise<import("../bindings/GrassKindInfo").GrassKindInfo[]> | null = null;

/** D5's default sky: the share of it that soft cumulus covers (ADR-065). */
export const D5_CLOUDS = 0.42;

/** Auto exposure's target: the light on level ground, sun and sky, that maps to exposure 1. */
const AUTO_EXPOSURE = 5.5;

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
  // The ViewCube (ADR-037), made with the renderer.
  const [cube, setCube] = useState<ViewCube | null>(null);
  // Bumped when the meshes are rebuilt; Realistic's materials are loaded for them.
  const [meshRev, setMeshRev] = useState(0);
  const real = useRef<{ rev: number; map: Map<string, RealMaterial> } | null>(null);
  // The structural and MEPT overlays (ADR-080, ADR-082): coloured members over the ghosted
  // architecture, each in its own group; a click explains what it hits.
  const structuralOn = useAppStore((s) => s.structuralOverlay && !!s.app?.structuralLayer);
  const structuralAlpha = useAppStore((s) => s.structuralAlpha);
  const mepKey = useAppStore((s) =>
    s.mepOverlay.filter((d) => s.app?.mepLayers.includes(d)).join(","),
  );
  const revision3d = useAppStore((s) => s.app?.revision ?? 0);
  const [stInfo, setStInfo] = useState<InfoAt | null>(null);
  const anyOverlay = structuralOn || !!mepKey;
  const overlayGroup = (name: string, on: boolean, load: () => Promise<OverlayMesh[]>) => {
    const t = three.current;
    if (!t) return () => {};
    const old = t.scene.getObjectByName(name) as THREE.Group | undefined;
    if (old) {
      t.scene.remove(old);
      disposeGroup(old);
    }
    if (!on) return () => {};
    let live = true;
    load().then(
      (meshes) => {
        const t2 = three.current;
        if (!live || !t2) return;
        const g = buildStructural3d(meshes, structuralAlpha);
        g.name = name;
        t2.scene.add(g);
      },
      () => {},
    );
    return () => {
      live = false;
    };
  };
  useEffect(
    () => overlayGroup("structural-overlay", structuralOn, () => ipc.structuralOverlay3d()),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [structuralOn, structuralAlpha, revision3d, meshRev],
  );
  useEffect(
    () =>
      overlayGroup("mep-overlay", !!mepKey, () =>
        ipc.mepOverlay3d(mepKey.split(",") as Discipline[]),
      ),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [mepKey, structuralAlpha, revision3d, meshRev],
  );
  useEffect(() => {
    const t = three.current;
    if (!t) return;
    ghostArchitecture(t.group, anyOverlay);
    if (!anyOverlay) return;
    const canvas = t.renderer.domElement;
    const onClick = (e: MouseEvent) => {
      const t3 = three.current;
      if (!t3) return;
      const groups = ["structural-overlay", "mep-overlay"]
        .map((n) => t3.scene.getObjectByName(n))
        .filter((g): g is THREE.Object3D => !!g);
      const r = canvas.getBoundingClientRect();
      const ray = new THREE.Raycaster();
      ray.setFromCamera(
        new THREE.Vector2(
          ((e.clientX - r.left) / r.width) * 2 - 1,
          -((e.clientY - r.top) / r.height) * 2 + 1,
        ),
        t3.camera,
      );
      const hit = ray.intersectObjects(
        groups.flatMap((g) => g.children),
        false,
      )[0];
      if (!hit) {
        setStInfo(null);
        return;
      }
      const u = hit.object.userData as { member: number | null; flag: number | null };
      const fromMep = hit.object.parent?.name === "mep-overlay";
      const ask = fromMep ? ipc.mepInfo(u.member, u.flag) : ipc.structuralInfo(u.member, u.flag);
      void ask.then((info) => {
        if (info) setStInfo({ info, x: e.clientX - r.left, y: e.clientY - r.top, pinned: true });
      });
    };
    canvas.addEventListener("click", onClick);
    return () => {
      canvas.removeEventListener("click", onClick);
    };
  }, [anyOverlay, revision3d, meshRev]);

  useEffect(() => {
    const wrap = wrapRef.current;
    if (!wrap) return;
    const renderer = new THREE.WebGLRenderer({ antialias: true });
    renderer.setPixelRatio(window.devicePixelRatio || 1);
    renderer.setClearColor(0xf8f8f6);
    renderer.localClippingEnabled = true;
    wrap.appendChild(renderer.domElement);
    const scene = new THREE.Scene();
    const hemi = new THREE.HemisphereLight(0xffffff, 0xb8b8b0, 2.2);
    scene.add(hemi);
    const sun = new THREE.DirectionalLight(0xffffff, 1.4);
    sun.position.set(-0.6, -1, 1.4);
    scene.add(sun, sun.target);
    renderer.shadowMap.enabled = true;
    // three r186 dropped PCFSoftShadowMap: PCF with a blur radius gives the soft sun edge.
    renderer.shadowMap.type = THREE.PCFShadowMap;
    Object.assign(sun.shadow, { bias: -0.0005, normalBias: 20, radius: 3, blurSamples: 12 });
    sun.shadow.mapSize.set(2048, 2048);
    // Realistic's ground: a matte lawn under the model when there's no topography.
    const ground = new THREE.Mesh(
      new THREE.CircleGeometry(1, 64),
      new THREE.MeshStandardMaterial({ color: 0x6f7d5c, roughness: 1 }),
    );
    ground.receiveShadow = true;
    ground.visible = false;
    scene.add(ground);
    const camera = new THREE.PerspectiveCamera(40, 1, 50, 1e7);
    camera.up.set(0, 0, 1);
    camera.layers.enable(1);
    const controls = new OrbitControls(camera, renderer.domElement);
    controls.enableDamping = true;
    controls.screenSpacePanning = true;
    // The wheel zooms toward the cursor (ADR-043); see `retarget` for the depth.
    controls.zoomToCursor = true;
    const group = new THREE.Group();
    scene.add(group);
    const boxGroup = new THREE.Group();
    scene.add(boxGroup);
    const capGroup = new THREE.Group();
    scene.add(capGroup);
    const terrainGroup = new THREE.Group();
    scene.add(terrainGroup);
    // Placement ghosts (a door's box, a wall's rubber band) and the ground grid.
    const ghost = new THREE.Group();
    scene.add(ghost);
    const gridGroup = new THREE.Group();
    scene.add(gridGroup);
    // Align's face highlights (ADR-097).
    const alignGroup = new THREE.Group();
    scene.add(alignGroup);
    three.current = {
      renderer,
      scene,
      camera,
      controls,
      group,
      boxGroup,
      capGroup,
      terrainGroup,
      gridGroup,
      fitted: false,
      hemi,
      sun,
      ground,
      sky: null,
      env: null,
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

    // Fit and the cube's turns fit the section box when there is one, else the model.
    const cube = new ViewCube({
      camera,
      target: controls.target,
      bounds: () => {
        const model = new THREE.Box3().setFromObject(group);
        const b = boxRef.current;
        if (!b) return model.isEmpty() ? null : model;
        const box = new THREE.Box3(
          new THREE.Vector3().fromArray(b.min),
          new THREE.Vector3().fromArray(b.max),
        );
        return model.isEmpty() ? box : box.intersect(model);
      },
      home: () => savedHome(view.id),
    });
    const stopTurn = () => cube.stop();
    controls.addEventListener("start", stopTurn);
    const onFit = () => cube.fit();
    window.addEventListener("view-fit", onFit);
    setCube(cube);

    let raf = 0;
    // Realistic renders through ambient occlusion (GTAO), multisampled, then tone mapped.
    const composer = () => {
      const t = three.current;
      if (!t) return null;
      if (t.composer === undefined) {
        const r = wrap.getBoundingClientRect();
        const target = new THREE.WebGLRenderTarget(r.width, r.height, {
          samples: 4,
          type: THREE.HalfFloatType,
        });
        const c = new EffectComposer(renderer, target);
        c.addPass(new RenderPass(scene, camera));
        // Ambient occlusion sees the model, not the foliage (layer 1).
        const aoCamera = new THREE.PerspectiveCamera();
        t.aoCamera = aoCamera;
        const ao = new GTAOPass(scene, aoCamera, r.width, r.height);
        // Model units are mm: contact shadow within about 2'.
        ao.updateGtaoMaterial({ radius: 600, distanceExponent: 1.4, thickness: 300, scale: 1 });
        ao.blendIntensity = 0.85;
        c.addPass(ao);
        // A soft bloom on the brightest light: sun glints, lit lenses (Enscape's).
        c.addPass(new UnrealBloomPass(new THREE.Vector2(r.width, r.height), 0.12, 0.5, 6));
        c.addPass(new OutputPass());
        // D5's look (ADR-065): a touch more colour and contrast after tone mapping.
        c.addPass(new ShaderPass(D5_GRADE));
        t.composer = c;
      }
      return t.composer;
    };
    // ---- Enscape's Walk and Fly (ADR-063) ----
    let nav: NavState | null = null;
    let navLast = performance.now();
    let navSpeed = 1;
    const keys = new Set<string>();
    // Walk steps through foliage (and doesn't stand on it).
    const solids = () =>
      [...group.children, ...terrainGroup.children].filter(
        (c): c is THREE.Mesh =>
          c instanceof THREE.Mesh && c.visible && c.userData.category !== "Planting",
      );
    const world = {
      ground: (x: number, y: number, z: number) => {
        const ray = new THREE.Raycaster(
          new THREE.Vector3(x, y, z),
          new THREE.Vector3(0, 0, -1),
          0,
          1e6,
        );
        const hit = ray
          .intersectObjects([...solids(), ground], false)
          .find((h) => h.object.visible);
        if (hit) return hit.point.z;
        const levels = useAppStore.getState().app?.levelElevations ?? [0];
        return Math.min(0, ...levels);
      },
      blocked: (a: [number, number, number], b: [number, number, number]) => {
        const from = new THREE.Vector3(...a);
        const to = new THREE.Vector3(...b);
        const len = from.distanceTo(to);
        if (len < 1) return false;
        const ray = new THREE.Raycaster(from, to.sub(from).normalize(), 0, len);
        // Doors swing open as you walk through, as Enscape's do.
        return ray
          .intersectObjects(solids(), false)
          .some((h) => h.object.userData.category !== "Door");
      },
    };
    const typing = (e: KeyboardEvent) => {
      const el = e.target as HTMLElement | null;
      return (
        !!el && (el.tagName === "INPUT" || el.tagName === "SELECT" || el.tagName === "TEXTAREA")
      );
    };
    const NAV_KEYS = [
      "w",
      "a",
      "s",
      "d",
      "q",
      "e",
      "arrowup",
      "arrowdown",
      "arrowleft",
      "arrowright",
      "shift",
    ];
    const onNavKey = (e: KeyboardEvent) => {
      const s = useAppStore.getState();
      if (s.nav3d === "orbit" || typing(e) || s.activeView !== view.id) return;
      const k = e.key.toLowerCase();
      if (e.type === "keydown" && k === " ") {
        s.setNav3d(s.nav3d === "walk" ? "fly" : "walk");
      } else if (e.type === "keydown" && k === "escape") {
        s.setNav3d("orbit");
      } else if (NAV_KEYS.includes(k)) {
        if (e.type === "keydown") keys.add(k);
        else keys.delete(k);
      } else return;
      // The view has the keys: no two-letter shortcuts while walking.
      e.preventDefault();
      e.stopImmediatePropagation();
    };
    window.addEventListener("keydown", onNavKey, { capture: true });
    window.addEventListener("keyup", onNavKey, { capture: true });
    const onBlur = () => keys.clear();
    window.addEventListener("blur", onBlur);
    // Dragging looks around.
    const onNavLook = (e: PointerEvent) => {
      if (!nav || !(e.buttons & 1)) return;
      nav = look(nav, e.movementX, e.movementY);
    };
    renderer.domElement.addEventListener("pointermove", onNavLook);
    const onNavWheel = (e: WheelEvent) => {
      if (!nav) return;
      navSpeed = Math.min(20, Math.max(0.1, navSpeed * (e.deltaY < 0 ? 1.2 : 1 / 1.2)));
    };
    renderer.domElement.addEventListener("wheel", onNavWheel, { passive: true });
    const stepNav = () => {
      const now = performance.now();
      const dt = Math.min(0.1, (now - navLast) / 1000);
      navLast = now;
      const mode = useAppStore.getState().nav3d;
      if (mode === "orbit") {
        if (nav) {
          // Back to Orbit: about a point ahead.
          const d = lookDir(nav);
          controls.target.set(
            nav.pos[0] + d[0] * 4000,
            nav.pos[1] + d[1] * 4000,
            nav.pos[2] + d[2] * 4000,
          );
          controls.enabled = true;
          nav = null;
          keys.clear();
        }
        return false;
      }
      controls.enabled = false;
      nav ??= fromLook(
        camera.position.toArray() as [number, number, number],
        controls.target.toArray() as [number, number, number],
      );
      const has = (...k: string[]) => k.some((x) => keys.has(x));
      const input = {
        forward: (has("w", "arrowup") ? 1 : 0) - (has("s", "arrowdown") ? 1 : 0),
        right: (has("d", "arrowright") ? 1 : 0) - (has("a", "arrowleft") ? 1 : 0),
        up: (has("e") ? 1 : 0) - (has("q") ? 1 : 0),
        fast: has("shift"),
      };
      const before = nav.pos.join();
      nav = navStep(nav, input, dt, mode, navSpeed, mode === "walk" ? world : undefined);
      const d = lookDir(nav);
      camera.position.set(...nav.pos);
      controls.target.set(
        nav.pos[0] + d[0] * 4000,
        nav.pos[1] + d[1] * 4000,
        nav.pos[2] + d[2] * 4000,
      );
      camera.lookAt(controls.target);
      if (nav.pos.join() !== before) onCameraChange();
      return true;
    };
    const loop = () => {
      cube.step();
      if (!stepNav()) controls.update();
      const realistic = styleOf(useAppStore.getState(), view.id) === "realistic";
      // Realistic's exposure (the sun panel's), raised after dark for the lights.
      renderer.toneMappingExposure = realistic
        ? useAppStore.getState().exposure3d * (three.current?.autoExposure ?? 1)
        : 1;
      const c = realistic && !boxRef.current ? composer() : null;
      const ac = three.current?.aoCamera;
      if (c && ac) {
        ac.copy(camera);
        ac.layers.set(0);
      }
      // The plants' and grass's wind.
      if (realistic) {
        wind.time.value = performance.now() / 1000;
        // Grass grows around where you are: the camera near the ground, else what it
        // looks at.
        const g = three.current?.grass as
          (THREE.Group & { follow?: (x: number, y: number) => void }) | undefined;
        if (g?.follow) {
          const low = camera.position.z - controls.target.z < 40_000;
          const p = low ? camera.position : controls.target;
          g.follow(p.x, p.y);
        }
      }
      if (c) c.render();
      else renderer.render(scene, camera);
      cube.render(renderer);
      raf = requestAnimationFrame(loop);
    };
    loop();
    const ro = new ResizeObserver(() => {
      const r = wrap.getBoundingClientRect();
      renderer.setSize(r.width, r.height);
      three.current?.composer?.setSize(r.width, r.height);
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
    // Zoom and orbit about what's under the cursor, else the selection (ADR-043): the orbit
    // center slides along the view line to that depth, so the view doesn't jump and the zoom
    // heads at what's pointed at instead of the middle of the screen.
    const retarget = (e: MouseEvent) => {
      const box = boxRef.current;
      const inBox = (q: THREE.Vector3) =>
        !box ||
        (q.x >= box.min[0]! &&
          q.x <= box.max[0]! &&
          q.y >= box.min[1]! &&
          q.y <= box.max[1]! &&
          q.z >= box.min[2]! &&
          q.z <= box.max[2]!);
      const meshes = group.children.filter((c) => c instanceof THREE.Mesh && c.visible);
      let point: THREE.Vector3 | null =
        rayAt(e as PointerEvent)
          .intersectObjects(meshes, false)
          .find((h) => inBox(h.point))?.point ?? null;
      if (!point) {
        const sel = new Set(useAppStore.getState().selection);
        const b = new THREE.Box3();
        for (const c of meshes) if (sel.has(c.userData.el as string)) b.expandByObject(c);
        if (!b.isEmpty()) point = b.getCenter(new THREE.Vector3());
      }
      if (!point) return;
      const next = pivotAt(camera, point);
      if (next) controls.target.copy(next);
    };
    const onWheelFirst = (e: WheelEvent) => retarget(e);
    const onOrbitStart = (e: PointerEvent) => {
      if (e.button === 0 && useAppStore.getState().tool === "select") retarget(e);
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
    let skCursor: Pt | null = null;
    let vertexDrag: { from: Pt; to: Pt | null } | null = null;
    /** A sketch point in 3D: on the level's work plane, or on the wall face of a wall
     * opening's sketch (ADR-058), nudged 5 mm toward the viewer. */
    const toWorld = (q: Pt): THREE.Vector3 => {
      const sk = useAppStore.getState().app?.sketch;
      const w = sk?.wall;
      if (!w) return new THREE.Vector3(q.x, q.y, (sk?.elevation ?? 0) + 5);
      const f = w.frame;
      const off = f.half + 5;
      return new THREE.Vector3(
        f.start.x + f.dir.x * q.x + f.normal.x * off,
        f.start.y + f.dir.y * q.x + f.normal.y * off,
        f.base_z + q.y,
      );
    };
    const skLine = (pts: Pt[], color: number, opacity = 1) => {
      const geo = new THREE.BufferGeometry().setFromPoints(pts.map(toWorld));
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
      const sel = new Set(s.sketchUi.sel);
      const bad = new Set(sk.bad);
      const vd = vertexDrag;
      sk.curves.forEach((c, i) => {
        const pts = vd?.to
          ? c.pts.map((q) => (Math.hypot(q.x - vd.from.x, q.y - vd.from.y) < 1 ? vd.to! : q))
          : c.pts;
        skLine(pts, bad.has(i) ? 0xe0261d : sel.has(i) ? 0x3ecff7 : 0xc832b4);
      });
      for (const pl of skPreview) skLine(pl, 0x3ecff7, 0.85);
      if (skCursor) {
        const r = Math.max(60, camera.position.distanceTo(toWorld(skCursor)) * 0.006);
        skLine(
          [
            { x: skCursor.x - r, y: skCursor.y },
            { x: skCursor.x + r, y: skCursor.y },
          ],
          0x3ecff7,
        );
        skLine(
          [
            { x: skCursor.x, y: skCursor.y - r },
            { x: skCursor.x, y: skCursor.y + r },
          ],
          0x3ecff7,
        );
        if (skPts.length) skLine([skPts[skPts.length - 1]!, skCursor], 0x3ecff7, 0.5);
      }
      for (const g of sketchGrips()) {
        const at = toWorld(g);
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
    const sketchPlaneHit = (e: PointerEvent): { pt: Pt; q: THREE.Vector3 } | null => {
      const sk = useAppStore.getState().app?.sketch;
      if (!sk) return null;
      const q = new THREE.Vector3();
      const w = sk.wall;
      if (w) {
        // A wall opening's sketch: the wall face, in the wall's (u, z).
        const f = w.frame;
        const n = new THREE.Vector3(f.normal.x, f.normal.y, 0);
        const on = new THREE.Vector3(
          f.start.x + f.normal.x * f.half,
          f.start.y + f.normal.y * f.half,
          f.base_z,
        );
        const plane = new THREE.Plane().setFromNormalAndCoplanarPoint(n, on);
        if (!rayAt(e).ray.intersectPlane(plane, q)) return null;
        const u = (q.x - f.start.x) * f.dir.x + (q.y - f.start.y) * f.dir.y;
        return { pt: { x: u, y: q.z - f.base_z }, q };
      }
      const plane = new THREE.Plane(new THREE.Vector3(0, 0, 1), -sk.elevation);
      return rayAt(e).ray.intersectPlane(plane, q) ? { pt: { x: q.x, y: q.y }, q } : null;
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
      const hp = sketchPlaneHit(e);
      return hp ? { at: hp.pt, q: hp.q } : null;
    };
    /** The selected line end under the cursor (within 8 px), to drag. */
    const gripAt3d = (e: PointerEvent): Pt | null => {
      const sk = useAppStore.getState().app?.sketch;
      if (!sk) return null;
      const r = renderer.domElement.getBoundingClientRect();
      for (const g of sketchGrips()) {
        const v = toWorld(g).project(camera);
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
        const h = sketchPlaneHit(e);
        if (h) vertexDrag.to = (await ipc.snap(sk.view, h.pt, null, snapTol(h.q))).pt;
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
        const h = sketchPlaneHit(e);
        if (h) {
          const q = h.q;
          const from = skPts[skPts.length - 1] ?? null;
          const sn = await ipc.snap(sk.view, h.pt, from, snapTol(q));
          skCursor = sn.pt;
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
      const h = sketchPlaneHit(e);
      if (!h) return;
      const raw = h.pt;
      const tol = snapTol(h.q);
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
    /** Orient to the work plane: look square at a wall opening's face (ADR-058), from as
     * far as the camera is now, centered on the sketch (or the wall). */
    const orientToSketch = () => {
      const w = useAppStore.getState().app?.sketch?.wall;
      if (!w) return;
      const f = w.frame;
      const pts = useAppStore.getState().app?.sketch?.curves.flatMap((c) => c.pts) ?? [];
      const mid =
        pts.length > 0
          ? {
              x: pts.reduce((a, p) => a + p.x, 0) / pts.length,
              y: pts.reduce((a, p) => a + p.y, 0) / pts.length,
            }
          : { x: f.length / 2, y: f.height / 2 };
      const target = toWorld(mid);
      const dist = Math.max(camera.position.distanceTo(controls.target), f.length * 0.9, 4000);
      camera.position.set(target.x + f.normal.x * dist, target.y + f.normal.y * dist, target.z);
      controls.target.copy(target);
      camera.lookAt(target);
      controls.update();
    };
    const onOrient = () => orientToSketch();
    window.addEventListener("orient-to-sketch", onOrient);
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
            ? await ipc.openingPreview3d(
                typeId,
                h.el,
                { x: h.point.x, y: h.point.y },
                s.openingTurns,
              )
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
        const pv = await ipc.openingPreview3d(
          typeId,
          h.el,
          { x: h.point.x, y: h.point.y },
          s.openingTurns,
        );
        if (!pv?.preview.valid) {
          s.setError("That spot overlaps another door or window in this wall.");
          return;
        }
        await apply(() =>
          ipc.createOpening(
            typeId,
            pv.preview.host,
            pv.preview.offset,
            pv.preview.flipFacing,
            pv.preview.flipHand,
          ),
        );
        clearGhost();
      } else if (tool === "wallOpening") {
        // Revit's work plane: the face picked (ADR-058). The view turns to face it.
        const hit = rayAt(e).intersectObjects(
          group.children.filter((c) => c instanceof THREE.Mesh && c.visible),
          false,
        )[0];
        const el = hit?.object.userData.el as string | undefined;
        const cats = el ? await ipc.selectionCategories([el]) : [];
        const n = hit?.face?.normal;
        if (!el || !n || !cats.includes("Wall") || Math.abs(n.z) > 0.5) {
          s.setError("Click the face of a wall to cut the opening in.");
          return;
        }
        if (await startWallOpening(el, { x: n.x, y: n.y })) orientToSketch();
      } else if (tool === "plant") {
        // On the ground, a floor or a roof terrace clicked (not on other plants).
        const hit = rayAt(e).intersectObjects(
          group.children.filter(
            (c) =>
              c instanceof THREE.Mesh &&
              c.visible &&
              (c.userData.info as MeshInfo | undefined)?.mesh.category !== "Planting",
          ),
          false,
        )[0];
        const q = hit?.point ?? planeHit(e)?.point;
        const levels = s.app?.levels ?? [];
        const elevs = s.app?.levelElevations ?? [];
        if (!q || levels.length === 0) {
          s.setError("Click the ground, a floor or a roof to place the plant.");
          return;
        }
        let i = elevs.reduce(
          (best, z, k) => (Math.abs(z) < Math.abs(elevs[best] ?? Infinity) ? k : best),
          0,
        );
        elevs.forEach((z, k) => {
          if (z <= q.z + 1 && z > (elevs[i] ?? -Infinity)) i = k;
        });
        await placePlant(view.id, { x: q.x, y: q.y }, levels[i]!.id);
      } else if (tool === "light") {
        // On the face clicked, as Revit hosts fixtures: under a ceiling at its height, on a
        // wall at the height clicked; on a floor or the ground at the type's own height.
        const hit = rayAt(e).intersectObjects(
          group.children.filter((c) => c instanceof THREE.Mesh && c.visible),
          false,
        )[0];
        const levels = s.app?.levels ?? [];
        const elevs = s.app?.levelElevations ?? [];
        const q = hit?.point ?? planeHit(e)?.point;
        if (!q || levels.length === 0) {
          s.setError("Click a ceiling, wall or floor to place the fixture.");
          return;
        }
        // The level at or below the point.
        let i = 0;
        elevs.forEach((z, k) => {
          if (z <= q.z + 1 && z >= (elevs[i] ?? -Infinity)) i = k;
        });
        const up = (hit?.face?.normal.z ?? 1) > 0.5;
        const elevation = up ? null : q.z - (elevs[i] ?? 0);
        await apply(() =>
          ipc.createLightingFixture(
            view.id,
            s.toolTypes.light,
            { x: q.x, y: q.y },
            levels[i]!.id,
            elevation,
          ),
        );
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
    // ---- Align (ADR-097): the reference face, then a parallel face on what moves ----
    let alignRef: AlignFace | null = null;
    const alignFaceAt = (e: PointerEvent): AlignFace | null => {
      const h = rayAt(e).intersectObjects(
        group.children.filter((c) => c instanceof THREE.Mesh && c.visible),
        false,
      )[0];
      if (!h?.face || !(h.object instanceof THREE.Mesh)) return null;
      return faceAt(h.object, h.point, h.face.normal);
    };
    const showAlign = (hover: AlignFace | null) => {
      for (const c of [...alignGroup.children]) {
        alignGroup.remove(c);
        c.traverse((o) => {
          if (o instanceof THREE.Mesh || o instanceof THREE.LineSegments) {
            o.geometry.dispose();
            (o.material as THREE.Material).dispose();
          }
        });
      }
      if (alignRef) alignGroup.add(faceHighlight(alignRef, true));
      if (hover && hover.tris.length) alignGroup.add(faceHighlight(hover, false));
    };
    const alignHover = (e: PointerEvent) => {
      const f = alignFaceAt(e);
      // After the reference, only faces that could move onto it light up.
      showAlign(f && (!alignRef || canAlign(alignRef, f)) ? f : null);
    };
    const alignClick = async (e: PointerEvent) => {
      const s = useAppStore.getState();
      const f = alignFaceAt(e);
      if (!f) return;
      if (!alignRef) {
        alignRef = f;
        showAlign(null);
        s.setPrompt(prompt3d("align", 1));
        return;
      }
      if (!canAlign(alignRef, f)) {
        s.setError(
          f.el === alignRef.el
            ? "Pick a face on another element: the one to move."
            : "Pick a face parallel to the reference.",
        );
        return;
      }
      const ref = alignRef;
      alignRef = null;
      showAlign(null);
      s.setPrompt(prompt3d("align", 0));
      await apply(() => ipc.align3d(f.el, alignDelta(ref, f)));
    };
    const onCancel = () => {
      const s = useAppStore.getState();
      if (s.tool === "sketch") {
        cancelSketch3d();
        return;
      }
      if (wallFrom) wallFrom = null;
      else if (moveFrom) moveFrom = null;
      else if (alignRef) {
        alignRef = null;
        showAlign(null);
        s.setPrompt(prompt3d("align", 0));
      } else if (s.tool !== "select") s.setTool("select");
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
        alignRef = null;
        showAlign(null);
        clearGhost();
        s.setPrompt(prompt3d(s.tool));
      }
    });

    // ---- D5's Grass Brush (ADR-065): a ring on the ground; dragging lays dabs ----
    const brushRing = new THREE.Mesh(
      new THREE.RingGeometry(0.93, 1, 64),
      new THREE.MeshBasicMaterial({
        color: 0x3ecff7,
        transparent: true,
        opacity: 0.95,
        depthTest: false,
      }),
    );
    brushRing.renderOrder = 10;
    brushRing.visible = false;
    scene.add(brushRing);
    const brushPaint = new THREE.Group();
    scene.add(brushPaint);
    let stroke: { dabs: [number, number, number, number][] } | null = null;
    const brushHit = (e: PointerEvent) => {
      const targets = [...group.children, ...terrainGroup.children, ground].filter(
        (c) =>
          c instanceof THREE.Mesh &&
          c.visible &&
          c.userData.category !== "Planting" &&
          c.userData.category !== "GrassPatch",
      );
      const h = rayAt(e).intersectObjects(targets, false)[0];
      return h?.point ?? planeHit(e)?.point ?? null;
    };
    const brushOptions = () => useAppStore.getState().options;
    const showRing = (p: THREE.Vector3) => {
      const o = brushOptions();
      brushRing.position.set(p.x, p.y, p.z + 15);
      brushRing.scale.setScalar(o.grassBrush);
      (brushRing.material as THREE.MeshBasicMaterial).color.setHex(
        o.grassErase ? 0xc0352b : 0x3ecff7,
      );
      brushRing.visible = true;
    };
    const addDab = (p: THREE.Vector3) => {
      if (!stroke) return;
      const o = brushOptions();
      stroke.dabs.push([p.x, p.y, p.z, o.grassBrush]);
      const disc = new THREE.Mesh(
        new THREE.CircleGeometry(o.grassBrush, 32),
        new THREE.MeshBasicMaterial({
          color: o.grassErase ? 0xc0352b : 0x5fae3c,
          transparent: true,
          opacity: 0.3,
          depthWrite: false,
        }),
      );
      disc.position.set(p.x, p.y, p.z + 10);
      brushPaint.add(disc);
    };
    const endStroke = async () => {
      const st = stroke;
      stroke = null;
      for (const c of [...brushPaint.children]) {
        brushPaint.remove(c);
        (c as THREE.Mesh).geometry.dispose();
      }
      controls.enabled = true;
      if (!st?.dabs.length) return;
      const s = useAppStore.getState();
      const o = s.options;
      if (o.grassErase) {
        await apply(() => ipc.eraseGrass(st.dabs));
        return;
      }
      const kinds = await (grassKinds ??= ipc.grassKinds());
      const spec = kinds.find((k) => k.kind === o.grassKind)?.spec;
      if (!spec) return;
      // The level at or below the stroke's start.
      const z = st.dabs[0]![2];
      const levels = s.app?.levels ?? [];
      const elevs = s.app?.levelElevations ?? [];
      let i = elevs.reduce(
        (best, e, k) => (Math.abs(e) < Math.abs(elevs[best] ?? Infinity) ? k : best),
        0,
      );
      elevs.forEach((e, k) => {
        if (e <= z + 1 && e > (elevs[i] ?? -Infinity)) i = k;
      });
      await apply(() =>
        ipc.paintGrass(levels[i]?.id ?? null, st.dabs, {
          ...spec,
          density: Math.max(0.2, Math.min(2, o.grassDensity / 100)),
        }),
      );
    };

    // Dragging a section box handle moves that face along its axis.
    let drag: { axis: number; end: "min" | "max"; at: THREE.Vector3 } | null = null;
    let down: [number, number] | null = null;
    const onDown = (e: PointerEvent) => {
      down = [e.clientX, e.clientY];
      if (e.button === 0 && useAppStore.getState().tool === "grassBrush") {
        const p = brushHit(e);
        if (!p) return;
        stroke = { dabs: [] };
        addDab(p);
        controls.enabled = false;
        renderer.domElement.setPointerCapture(e.pointerId);
        return;
      }
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
    // Revit's Tab selection in 3D (ADR-056): the elements the ray hits, nearest first, and
    // the candidate Tab has stepped to (until the cursor moves off).
    let lastPointer: PointerEvent | null = null;
    let cycle3d: { x: number; y: number; cands: PickCandidate[]; i: number } | null = null;
    const relight = (lit: Set<string> | undefined) => {
      const st = useAppStore.getState();
      applySelection(group, litOf(st), styleOf(st, view.id), lit);
    };
    const endCycle = () => {
      if (!cycle3d) return;
      cycle3d = null;
      relight(undefined);
      useAppStore.getState().setHoverLabel("");
    };
    const onSelectTab = async (e: Event) => {
      const at = lastPointer;
      if (!at || useAppStore.getState().tool !== "select") return;
      if (!cycle3d) {
        const els: string[] = [];
        for (const h of rayAt(at).intersectObjects(
          group.children.filter((c) => c instanceof THREE.Mesh && c.visible),
          false,
        )) {
          const el = h.object.userData.el as string;
          if (!els.includes(el)) els.push(el);
        }
        const cands = els.length ? ((await ipc.pickCandidates(els)) ?? []) : [];
        if (cands.length < 2 || lastPointer !== at) return;
        cycle3d = { x: at.clientX, y: at.clientY, cands, i: 0 };
      }
      const c = cycle3d;
      const n = c.cands.length;
      c.i = (c.i + ((e as CustomEvent<boolean>).detail === true ? n - 1 : 1)) % n;
      const cand = c.cands[c.i]!;
      relight(new Set(cand.ids));
      useAppStore.getState().setHoverLabel(`${cand.label}  (${c.i + 1} of ${n}, Tab for the next)`);
    };
    const onTabEvent = (e: Event) => void onSelectTab(e);
    window.addEventListener("select-tab", onTabEvent);
    const onLeave = () => {
      lastPointer = null;
      endCycle();
    };
    const onMove = (e: PointerEvent) => {
      lastPointer = e;
      if (useAppStore.getState().tool === "grassBrush") {
        const p = brushHit(e);
        if (!p) {
          brushRing.visible = false;
          return;
        }
        showRing(p);
        const last = stroke?.dabs.at(-1);
        if (stroke && (!last || Math.hypot(last[0] - p.x, last[1] - p.y) > last[3] * 0.35))
          addDab(p);
        return;
      }
      brushRing.visible = false;
      if (useAppStore.getState().tool === "align") {
        alignHover(e);
        return;
      }
      if (cycle3d && Math.hypot(e.clientX - cycle3d.x, e.clientY - cycle3d.y) > 4) endCycle();
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
        // The caps are for the box as it was; they come back when it's set.
        t.capGroup.visible = false;
        const planes = boxPlanes(next);
        for (const child of t.group.children) {
          const mats = (child as THREE.Mesh).material as THREE.Material;
          if (mats) mats.clippingPlanes = planes;
        }
        drawBox(t, next);
      }
    };
    const onUp = (e: PointerEvent) => {
      if (stroke) {
        void endStroke();
        return;
      }
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
      if (useAppStore.getState().tool === "align") {
        void alignClick(e);
        return;
      }
      if (useAppStore.getState().tool === "paint") {
        const hit = rayAt(e).intersectObjects(
          group.children.filter((c) => c instanceof THREE.Mesh && c.visible),
          false,
        )[0];
        // The face hit (ADR-096): the group is z-up model space, so the hit is in mm.
        const n = hit?.face?.normal.clone().transformDirection(hit.object.matrixWorld);
        void paintElement(
          (hit?.object.userData.el as string | undefined) ?? null,
          e.shiftKey,
          hit && n
            ? {
                kind: "3d",
                point: [hit.point.x, hit.point.y, hit.point.z],
                normal: [n.x, n.y, n.z],
              }
            : undefined,
        );
        return;
      }
      if (useAppStore.getState().tool !== "select") {
        void click3d(e).finally(() => useAppStore.getState().setSnapOverride(null));
        return;
      }
      // Click (without dragging) selects the element under the cursor, or Tab's candidate.
      if (cycle3d) {
        const ids = cycle3d.cands[cycle3d.i]!.ids;
        cycle3d = null;
        useAppStore.getState().setHoverLabel("");
        const s = useAppStore.getState();
        s.select(
          e.shiftKey ? [...s.selection, ...ids.filter((x) => !s.selection.includes(x))] : ids,
        );
        return;
      }
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
      else if (u && u.category === "Wall") {
        // A wall opening's reveal (ADR-058): Edit Sketch on the face toward the camera.
        void ipc.selectionCategories([u.el]).then((c) => {
          if (!c.includes("WallOpening")) return;
          const d = camera.getWorldDirection(new THREE.Vector3());
          void editWallOpening(u.el, { x: -d.x, y: -d.y }).then((ok) => ok && orientToSketch());
        });
      }
    };
    renderer.domElement.addEventListener("dblclick", onDouble);
    // Before OrbitControls' own handlers (capture), so they zoom and orbit about the new center.
    renderer.domElement.addEventListener("wheel", onWheelFirst, { capture: true, passive: true });
    renderer.domElement.addEventListener("pointerdown", onOrbitStart, { capture: true });
    renderer.domElement.addEventListener("pointerdown", onDown);
    renderer.domElement.addEventListener("pointermove", onMove);
    renderer.domElement.addEventListener("pointerleave", onLeave);
    renderer.domElement.addEventListener("pointerup", onUp);
    useAppStore.getState().setPrompt(prompt3d(useAppStore.getState().tool));

    return () => {
      window.removeEventListener("tool-cancel", onCancel);
      window.removeEventListener("select-tab", onTabEvent);
      window.removeEventListener("orient-to-sketch", onOrient);
      window.removeEventListener("keydown", onNavKey, { capture: true });
      window.removeEventListener("keyup", onNavKey, { capture: true });
      window.removeEventListener("blur", onBlur);
      renderer.domElement.removeEventListener("pointermove", onNavLook);
      renderer.domElement.removeEventListener("wheel", onNavWheel);
      three.current?.composer?.dispose();
      renderer.domElement.removeEventListener("pointerleave", onLeave);
      window.clearTimeout(saveTimer);
      controls.removeEventListener("change", onCameraChange);
      controls.removeEventListener("start", stopTurn);
      window.removeEventListener("view-fit", onFit);
      setCube(null);
      cube.dispose();
      renderer.domElement.removeEventListener("dblclick", onDouble);
      renderer.domElement.removeEventListener("wheel", onWheelFirst, { capture: true });
      renderer.domElement.removeEventListener("pointerdown", onOrbitStart, { capture: true });
      unsubTool();
      unsubSketch();
      cancelAnimationFrame(raf);
      ro.disconnect();
      controls.dispose();
      three.current?.env?.dispose();
      three.current?.sky?.dispose();
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
          // Its material comes from the visual style (applyDisplay).
          const info: MeshInfo = { mesh: m, base: meshColor(m), map: null };
          const mesh = new THREE.Mesh(geo, new THREE.MeshBasicMaterial({ visible: false }));
          mesh.userData = {
            el: m.el,
            category: m.category,
            level: m.level,
            info,
            planes,
            style: null,
          };
          t.group.add(mesh);
          // Walls bring their own lines, without the seams where they're split around
          // openings (ADR-038); the rest take theirs from the triangles.
          let lines: THREE.BufferGeometry;
          if (m.edges.length > 0) {
            lines = new THREE.BufferGeometry();
            lines.setAttribute("position", new THREE.Float32BufferAttribute(m.edges, 3));
          } else {
            lines = new THREE.EdgesGeometry(geo, 25);
          }
          const edges = new THREE.LineSegments(
            lines,
            new THREE.LineBasicMaterial({ color: EDGE_COLOR, clippingPlanes: planes }),
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
        const style = styleOf(st, view.id);
        applyImagery(t.group, imagery.current);
        applyDisplay(t.group, st.tempHide[view.id] ?? null, style, real.current?.map ?? null);
        applySelection(t.group, litOf(st), style);
        applyScene(t, style);
        setMeshRev((n) => n + 1);
      },
      (e) => useAppStore.getState().setError(errorMessage(e)),
    );
    return () => {
      live = false;
    };
  }, [revision, sectionBox, view.id]);

  // The section box's caps (ADR-044): fetched with the model and the box.
  useEffect(() => {
    const t = three.current;
    if (!t) return;
    if (!sectionBox) {
      clearCaps(t.capGroup);
      return;
    }
    let live = true;
    ipc.sectionCaps(view.id).then(
      (caps) => {
        const t = three.current;
        if (!live || !t) return;
        buildCaps(t, caps);
        const st = useAppStore.getState();
        applyCaps(t, styleOf(st, view.id), litOf(st));
      },
      () => {},
    );
    return () => {
      live = false;
    };
  }, [revision, sectionBox, view.id, meshRev]);

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
  const visualStyle = useAppStore((s) => styleOf(s, view.id));
  const setVisualStyle = useAppStore((s) => s.setVisualStyle);
  const sketchTarget = useAppStore((s) => s.app?.sketch?.target ?? null);
  useEffect(() => {
    const t = three.current;
    if (!t) return;
    applyDisplay(t.group, temp, visualStyle, real.current?.map ?? null);
    applySelection(t.group, litOf(useAppStore.getState()), visualStyle);
    applyScene(t, visualStyle);
    applyCaps(t, visualStyle, litOf(useAppStore.getState()));
  }, [temp, visualStyle, revision, sketchTarget, meshRev]);

  // Realistic's sky and sun from the project's Sun Settings and site (ADR-062): a physical
  // sky for the light and background, without its sun disk, which the shadow-casting sun
  // light carries at the sky model's sun-to-sky ratio.
  const sunKey = useAppStore(
    (s) => JSON.stringify(s.sunPreview ?? s.app?.sun ?? null) + (s.app?.site ? "site" : ""),
  );
  const previewing = useAppStore((s) => !!s.sunPreview);
  useEffect(() => {
    if (visualStyle !== "realistic") return;
    let live = true;
    void (async () => {
      const [{ physicalSky, sunColor }, { horizontalIrradiance, toYUp }] = await Promise.all([
        import("../render/sky"),
        import("../render/pathtrace"),
      ]);
      const st = useAppStore.getState();
      const settings = st.sunPreview ?? st.app?.sun ?? null;
      const sun = await (settings ? ipc.sunFor(settings) : ipc.sunNow()).catch(() => null);
      const t = three.current;
      if (!live || !t) return;
      const alt = sun ? sun.altitude : 35;
      const night = alt <= 0;
      const dirZ = sun && !night ? sun.dir : [0.45, -0.55, 0.7];
      // A coarser sky while the time is being dragged.
      const sky = physicalSky({
        sunDir: toYUp(dirZ),
        altitude: night ? -6 : Math.max(alt, 1),
        sunToSky: 0,
        width: previewing ? 512 : 1024,
        height: previewing ? 256 : 512,
        // D5's default sky: fair-weather clouds (ADR-065).
        clouds: night ? 0 : D5_CLOUDS,
      });
      sky.mapping = THREE.EquirectangularReflectionMapping;
      sky.needsUpdate = true;
      const pmrem = new THREE.PMREMGenerator(t.renderer);
      const env = pmrem.fromEquirectangular(sky).texture;
      pmrem.dispose();
      t.env?.dispose();
      if (t.sky && t.sky !== sky) t.sky.dispose();
      t.env = env;
      t.sky = sky;
      const irr = horizontalIrradiance(sky);
      const ratio = 6 * Math.min(1, Math.max(0.15, (Math.max(alt, 2) + 2) / 20));
      t.sunDir = new THREE.Vector3(dirZ[0], dirZ[1], dirZ[2]);
      t.sunIntensity = night ? 1e-6 : ratio * irr;
      t.night = night;
      // Auto exposure: a lit ground at any time of day reads like a photograph's, a white
      // surface in full sun just short of white. After dark, a fixed boost for the lights.
      const lux = irr * (1 + ratio * Math.max(0.2, Math.sin((Math.max(alt, 2) * Math.PI) / 180)));
      t.autoExposure = night ? 8 : AUTO_EXPOSURE / Math.max(lux, 1e-6);
      // After dark the building's fixtures light it (ADR-063).
      if (!t.fixtures) {
        t.fixtures = new THREE.Group();
        t.scene.add(t.fixtures);
      }
      for (const c of [...t.fixtures.children]) t.fixtures.remove(c);
      if (night) {
        const lights = await ipc.lights(view.id).catch(() => [] as LightInfo[]);
        if (!live) return;
        for (const l of lights.filter((l) => l.on && l.lumens > 0).slice(0, 48))
          t.fixtures.add(...fixtureLight3d(l));
      }
      const c = sunColor(Math.max(alt, 2));
      t.sunColor = new THREE.Color().setRGB(c[0], c[1], c[2], THREE.LinearSRGBColorSpace);
      applyScene(t, "realistic");
    })();
    return () => {
      live = false;
    };
  }, [visualStyle, sunKey, previewing, revision, view.id]);

  // Realistic: the project's materials with their textures, loaded when first shown.
  useEffect(() => {
    if (visualStyle !== "realistic") return;
    const t = three.current;
    if (!t || real.current?.rev === meshRev) return;
    let live = true;
    const meshes = t.group.children
      .filter((c): c is THREE.Mesh => c instanceof THREE.Mesh)
      .map((c) => (c.userData.info as MeshInfo).mesh);
    ipc
      .renderMaterials()
      .then((list) => realMaterials(meshes, list, (set, map) => ipc.materialTexture(set, map)))
      .then(
        (map) => {
          const t = three.current;
          if (!live || !t) return;
          real.current = { rev: meshRev, map };
          for (const c of t.group.children) if (c instanceof THREE.Mesh) c.userData.style = null;
          const st = useAppStore.getState();
          const style = styleOf(st, view.id);
          applyDisplay(t.group, st.tempHide[view.id] ?? null, style, map);
          applySelection(t.group, litOf(st), style);
        },
        () => {},
      );
    return () => {
      live = false;
    };
  }, [visualStyle, meshRev, view.id]);

  // Realistic: the full plant models and Enscape's grass (ADR-064), loaded when first shown
  // and after the model changes.
  const baseGround = useAppStore((s) => s.app?.ground ?? null);
  useEffect(() => {
    const t = three.current;
    if (!t) return;
    const clear = () => {
      for (const k of ["plants", "grass"] as const) {
        const g = t[k];
        if (!g) continue;
        t.scene.remove(g);
        g.traverse((o) => {
          if (o instanceof THREE.InstancedMesh) {
            (o.material as THREE.Material).dispose();
            o.dispose();
          }
        });
        t[k] = undefined;
      }
    };
    if (visualStyle !== "realistic") {
      clear();
      return;
    }
    let live = true;
    void (async () => {
      const [instances, mats, patches] = await Promise.all([
        ipc.plantInstances(view.id).catch(() => []),
        ipc.renderMaterials().catch(() => []),
        ipc.grassPatches(view.id).catch(() => []),
      ]);
      const patchSpec = new Map(patches.map((p) => [p.el, p.spec]));
      const entries = await loadPlantEntries(instances, plantLoader);
      const t = three.current;
      if (!live || !t) return;
      clear();
      const plants = plantGroup(entries);
      plants.traverse((o) => o.layers.set(1));
      t.scene.add(plants);
      t.plants = plants;
      // Grass on every surface in a Grass material, and on the ground without topography.
      const byId = new Map(mats.map((m) => [m.id, m]));
      const surfaces: GrassSurface[] = [];
      // What covers the ground (paving, slabs): no grass grows through it.
      const blockers: number[][] = [];
      let hasSite = false;
      for (const c of t.group.children) {
        if (!(c instanceof THREE.Mesh)) continue;
        const m = (c.userData.info as MeshInfo | undefined)?.mesh;
        if (m?.category === "Site") hasSite = true;
        const mat = m?.material ? byId.get(m.material) : undefined;
        const grows =
          !!mat && (!!mat.appearance.grass || mat.appearance.texture === "gen:pine-straw");
        // Painted grass (ADR-065): its own kind, colour and density.
        const painted = m ? patchSpec.get(m.el) : undefined;
        if (m && painted) {
          surfaces.push({
            positions: m.positions,
            grass: { height: painted.height, variation: painted.variation },
            color: painted.color,
            kind: painted.kind,
            density: painted.density,
          });
          continue;
        }
        if (
          m &&
          !grows &&
          m.category !== "Site" &&
          m.category !== "Planting" &&
          m.category !== "GrassPatch"
        )
          blockers.push(m.positions);
        if (!m || !mat) continue;
        const cones = mat.appearance.texture === "gen:pine-straw";
        if (mat.appearance.grass || cones)
          surfaces.push({
            positions: m.positions,
            grass: mat.appearance.grass ?? null,
            color: mat.color,
            cones,
          });
      }
      const box = new THREE.Box3().setFromObject(t.group);
      const center = box.isEmpty() ? new THREE.Vector3() : box.getCenter(new THREE.Vector3());
      const radius = box.isEmpty() ? 10_000 : box.getSize(new THREE.Vector3()).length() / 2;
      const base = baseGround ? byId.get(baseGround) : undefined;
      if (
        !hasSite &&
        base &&
        (base.appearance.grass || base.appearance.texture === "gen:pine-straw")
      ) {
        const z = Math.min(0, ...(useAppStore.getState().app?.levelElevations ?? [0])) - 2;
        const r = Math.max(radius * 3, 45_000);
        const [x0, y0, x1, y1] = [center.x - r, center.y - r, center.x + r, center.y + r];
        surfaces.push({
          positions: [x0, y0, z, x1, y0, z, x1, y1, z, x0, y0, z, x1, y1, z, x0, y1, z],
          grass: base.appearance.grass ?? null,
          color: base.color,
          cones: base.appearance.texture === "gen:pine-straw",
        });
      }
      if (surfaces.length) {
        const grass = grassGroup(surfaces, center, radius, blockers);
        grass.traverse((o) => o.layers.set(1));
        t.scene.add(grass);
        t.grass = grass;
      }
      // The base ground's material on Realistic's ground (without topography).
      const key = base ? `${base.id}:${JSON.stringify(base.appearance)}` : "";
      if (t.groundKey !== key) {
        t.groundKey = key;
        const old = t.ground.material as THREE.Material;
        if (base) {
          const [real] = [
            await realMaterials([{ material: base.id } as Mesh], [base], (set, map) =>
              ipc.materialTexture(set, map),
            ),
          ];
          const r = real.get(base.id);
          if (r && live) {
            const m = (r.material as THREE.MeshPhysicalMaterial).clone();
            // The disc is scaled up hugely: tile its texture at the material's real size.
            const k = (2 * t.ground.scale.x) / Math.max(r.scale, 1);
            for (const map of [m.map, m.normalMap, m.roughnessMap]) {
              if (!map) continue;
              const c = map.clone();
              c.repeat.set(k, k / (r.aspect || 1));
              c.needsUpdate = true;
              if (map === m.map) m.map = c;
              else if (map === m.normalMap) m.normalMap = c;
              else m.roughnessMap = c;
            }
            t.ground.material = m;
            old.dispose();
          }
        } else {
          t.ground.material = new THREE.MeshStandardMaterial({ color: 0x6f7d5c, roughness: 1 });
          old.dispose();
        }
      }
    })();
    return () => {
      live = false;
    };
  }, [visualStyle, meshRev, view.id, baseGround]);

  const highlight = useAppStore((s) => s.highlight);
  useEffect(() => {
    if (three.current) {
      const style = styleOf(useAppStore.getState(), view.id);
      const lit = litOf({ selection, highlight });
      applySelection(three.current.group, lit, style);
      applyCaps(three.current, style, lit);
    }
  }, [selection, highlight, view.id]);

  const satellite = useAppStore((s) => s.satellite);
  const hasSite = useAppStore((s) => !!s.app?.site);
  // The site's terrain (ADR-045): fetched with the model, shown per the terrain toolbar.
  const terrainUi = useAppStore((s) => s.terrain);
  const setTerrainUi = useAppStore((s) => s.setTerrain);
  const [fetched, setTerrain] = useState<Terrain | null>(null);
  const terrain = hasSite ? fetched : null;
  useEffect(() => {
    let live = true;
    if (!hasSite) return;
    ipc.siteTerrain().then(
      (t) => live && setTerrain(t),
      () => {},
    );
    return () => {
      live = false;
    };
  }, [revision, hasSite]);
  useEffect(() => {
    const t = three.current;
    if (!t) return;
    buildTerrain(t, terrain);
    applyTerrain(
      t,
      useAppStore.getState().terrain,
      styleOf(useAppStore.getState(), view.id),
      sectionBox,
    );
  }, [terrain, view.id, sectionBox]);
  useEffect(() => {
    const t = three.current;
    if (!t) return;
    applyTerrain(t, terrainUi, visualStyle, sectionBox);
    applyCaps(t, visualStyle, litOf(useAppStore.getState()));
  }, [terrainUi, visualStyle, sectionBox]);

  useEffect(() => {
    let live = true;
    const off = () => {
      imagery.current = null;
      const t = three.current;
      if (!t) return;
      applyImagery(t.group, null);
      const st = useAppStore.getState();
      applyDisplay(
        t.group,
        st.tempHide[view.id] ?? null,
        styleOf(st, view.id),
        real.current?.map ?? null,
      );
      applySelection(t.group, litOf(st), styleOf(st, view.id));
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
        applyDisplay(
          t.group,
          st.tempHide[view.id] ?? null,
          styleOf(st, view.id),
          real.current?.map ?? null,
        );
        applySelection(t.group, litOf(st), styleOf(st, view.id));
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
      <ViewCubeOverlay cube={cube} viewId={view.id} />
      {structuralOn && <StructuralLegend />}
      {structuralOn && stInfo && <StructuralInfoCard at={stInfo} onClose={() => setStInfo(null)} />}
      <div className="view3d-top">
        <SunPanel />
        <NavBar />
      </div>
      <VisualStyleToggle value={visualStyle} onChange={(s) => setVisualStyle(view.id, s)} />
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
      {hasSite && terrain && (
        <TerrainBar
          solid={terrainUi.solid}
          contours={terrainUi.contours}
          labels={terrainUi.labels}
          interval={terrain.interval}
          onSolid={(solid) => setTerrainUi({ solid })}
          onContours={(contours) => setTerrainUi({ contours })}
          onLabels={(labels) => setTerrainUi({ labels })}
          onInterval={(mm) =>
            void apply(() => ipc.setProperty(terrain.site, "contour", `${(mm / 25.4).toFixed(2)}"`))
          }
        />
      )}
    </div>
  );
}

/** Drapes the satellite image on the ground (Site meshes), or takes it off (ADR-026). */
function applyImagery(group: THREE.Group, im: Imagery | null) {
  for (const child of group.children) {
    if (!(child instanceof THREE.Mesh) || child.userData.category !== "Site") continue;
    const info = child.userData.info as MeshInfo;
    info.map?.dispose();
    info.map = null;
    info.base = meshColor(info.mesh);
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
      info.map = tex;
      info.base = 0xffffff;
    }
    // Restyled with (or without) the image.
    child.userData.style = null;
  }
}

/** Temporary Hide/Isolate and the visual style (ADR-024, ADR-038) on the shown meshes. */
function applyDisplay(
  group: THREE.Group,
  temp: { isolate: boolean; ids: string[]; categories: string[] } | null,
  style: VisualStyle,
  real: Map<string, RealMaterial> | null,
) {
  // The floor or ceiling whose boundary is being edited is hidden meanwhile.
  const editing = useAppStore.getState().app?.sketch?.target ?? null;
  const realistic = style === "realistic";
  let lastMesh: THREE.Mesh | null = null;
  for (const child of group.children) {
    if (child instanceof THREE.Mesh) {
      lastMesh = child;
      const u = child.userData as {
        el: string;
        category: string;
        info: MeshInfo;
        planes: THREE.Plane[];
        style: VisualStyle | null;
      };
      const hit = temp ? temp.ids.includes(u.el) || temp.categories.includes(u.category) : false;
      child.visible = (!temp || (temp.isolate ? hit : !hit)) && u.el !== editing;
      if (u.style !== style) {
        const r =
          realistic && u.info.mesh.material ? (real?.get(u.info.mesh.material) ?? null) : null;
        if (r?.textured && child.userData.uvScale !== r.scale) {
          realUv(child.geometry, r.scale, r.aspect);
          child.userData.uvScale = r.scale;
        }
        (child.material as THREE.Material).dispose();
        child.material = styleMaterial(style, u.info, u.planes, r);
        u.style = style;
      }
      if (realistic && u.category === "Planting")
        (child.material as THREE.Material).visible = false;
      // Painted grass shows as its blades in Realistic (its patch stays pickable).
      if (realistic && u.category === "GrassPatch")
        (child.material as THREE.Material).visible = false;
      // Ground regions lie just over the ground: drawn in front of it (ADR-064).
      if (u.category === "GroundRegion")
        Object.assign(child.material as THREE.Material, {
          polygonOffset: true,
          polygonOffsetFactor: -4,
          polygonOffsetUnits: -8,
        });
      const shadows =
        realistic && !isGlass(u.info.mesh) && u.category !== "Ceiling" && u.category !== "Planting";
      child.castShadow = shadows;
      child.receiveShadow = realistic;
    } else if (child instanceof THREE.LineSegments && lastMesh) {
      // Each mesh's lines follow it; Realistic shows none.
      child.visible = lastMesh.visible && !realistic;
    }
  }
  // Under the structural overlay the architecture stays ghosted (ADR-080).
  const st = useAppStore.getState();
  if (
    (st.structuralOverlay && st.app?.structuralLayer) ||
    st.mepOverlay.some((d) => st.app?.mepLayers.includes(d))
  )
    ghostArchitecture(group, true);
}

/** A fixture's light in the z-up 3D view (ADR-063), in the render's units: candela over
 * mm², the sky at 6,000 lux a unit. Area sources become cosine spots. */
function fixtureLight3d(l: LightInfo): THREE.Object3D[] {
  const color = new THREE.Color().setRGB(
    l.color[0] / 255,
    l.color[1] / 255,
    l.color[2] / 255,
    THREE.SRGBColorSpace,
  );
  const k = 1e6 / 6000;
  const at = new THREE.Vector3(...l.at);
  const dir = new THREE.Vector3(...l.dir).normalize();
  if (l.distribution === "Spherical" && l.shape !== "Rectangle" && l.shape !== "Line") {
    const p = new THREE.PointLight(color, (l.lumens / (4 * Math.PI)) * k, 0, 2);
    p.position.copy(at);
    return [p];
  }
  const spot = l.distribution === "Spot";
  const half = spot
    ? THREE.MathUtils.degToRad(Math.min(Math.max(l.beam, 2), 170)) / 2
    : (Math.PI / 2) * 0.999;
  const cd = spot ? l.lumens / (2 * Math.PI * (1 - Math.cos(half))) : l.lumens / Math.PI;
  const s = new THREE.SpotLight(color, cd * k, 0, half, spot ? 0.35 : 1, 2);
  s.position.copy(at);
  s.target.position.copy(at.clone().addScaledVector(dir, 1000));
  return [s, s.target];
}

/** Tab's candidate under the cursor, pre-highlighted lighter than the selection (ADR-056). */
const PRESELECTED = 0x9fe6f8;

function applySelection(
  group: THREE.Group,
  selection: string[],
  style: VisualStyle,
  preselected?: Set<string>,
) {
  const sel = new Set(selection);
  const pre = (el: string) => !sel.has(el) && (preselected?.has(el) ?? false);
  let lastMesh: THREE.Mesh | null = null;
  for (const child of group.children) {
    if (child instanceof THREE.Mesh) {
      lastMesh = child;
      const on = sel.has(child.userData.el);
      const info = child.userData.info as MeshInfo;
      const mat = child.material as THREE.Material & {
        color?: THREE.Color;
        emissive?: THREE.Color;
      };
      const lit = pre(child.userData.el as string);
      if (style === "realistic" && info.mesh.category === "Planting") {
        // The Enscape proxy shows only while selected or under the cursor.
        Object.assign(mat, {
          visible: on || lit,
          transparent: true,
          opacity: 0.3,
          depthWrite: false,
        });
        mat.emissive?.setHex(on ? 0x146e87 : 0x0b4455);
      } else if (style === "realistic") {
        mat.emissive?.setHex(on ? 0x146e87 : lit ? 0x0b4455 : 0x000000);
      } else if (mat.color) {
        mat.color.setHex(
          on ? SELECTED : lit ? PRESELECTED : style === "hiddenLine" ? 0xffffff : info.base,
        );
      }
    } else if (child instanceof THREE.LineSegments && lastMesh) {
      // Selected lines turn blue too (all that shows of an element in Wireframe).
      const on = sel.has(lastMesh.userData.el);
      const lit = pre(lastMesh.userData.el as string);
      (child.material as THREE.LineBasicMaterial).color.setHex(
        on ? SELECTED : lit ? PRESELECTED : style === "wireframe" ? 0x2b2e31 : EDGE_COLOR,
      );
    }
  }
}

/** The scene around the model in a visual style: Realistic tone-maps, lights with an
 * environment and a shadow-casting sun, shows a sky and a ground, and dims the grid. */
function applyScene(t: Three, style: VisualStyle) {
  const realistic = style === "realistic";
  t.renderer.toneMapping = realistic ? THREE.ACESFilmicToneMapping : THREE.NoToneMapping;
  if (realistic && !t.env) {
    const pmrem = new THREE.PMREMGenerator(t.renderer);
    t.env = pmrem.fromScene(new RoomEnvironment(), 0.04).texture;
    pmrem.dispose();
  }
  if (realistic && !t.sky) t.sky = skyTexture();
  // The physical sky (y-up) turned into this z-up world (ADR-062).
  const physical = !!t.sunIntensity;
  t.scene.environment = realistic ? t.env : null;
  t.scene.background = realistic ? t.sky : null;
  t.scene.environmentRotation.set(physical ? Math.PI / 2 : 0, 0, 0);
  t.scene.backgroundRotation.set(physical ? Math.PI / 2 : 0, 0, 0);
  t.scene.environmentIntensity = physical ? 1 : 0.45;
  t.hemi.intensity = realistic ? (physical ? 0 : 0.35) : 2.2;
  t.sun.intensity = realistic ? (t.sunIntensity ?? 3.2) : 1.4;
  t.sun.color.copy(realistic && t.sunColor ? t.sunColor : new THREE.Color(0xffffff));
  t.sun.castShadow = realistic;
  if (t.fixtures) t.fixtures.visible = realistic;
  const shadowSize = realistic ? 4096 : 2048;
  if (t.sun.shadow.mapSize.x !== shadowSize) {
    t.sun.shadow.mapSize.set(shadowSize, shadowSize);
    t.sun.shadow.map?.dispose();
    t.sun.shadow.map = null;
  }
  const box = new THREE.Box3().setFromObject(t.group);
  const hasSite = t.group.children.some((c) => c.userData.category === "Site");
  if (!box.isEmpty()) {
    const c = box.getCenter(new THREE.Vector3());
    const r = Math.max(box.getSize(new THREE.Vector3()).length() / 2, 1000);
    // The sun from the southeast, high, its shadows falling to the northwest across the
    // model and the ground (Shaded keeps its light from the southwest).
    const dir = (
      realistic
        ? (t.sunDir?.clone() ?? new THREE.Vector3(0.7, -0.9, 1.1))
        : new THREE.Vector3(-0.6, -1, 1.4)
    ).normalize();
    t.sun.position.copy(c).addScaledVector(dir, r * 3);
    t.sun.target.position.copy(c);
    t.sun.target.updateMatrixWorld();
    Object.assign(t.sun.shadow.camera, {
      left: -r * 1.3,
      right: r * 1.3,
      top: r * 1.3,
      bottom: -r * 1.3,
      near: r * 0.5,
      far: r * 6,
    });
    t.sun.shadow.camera.updateProjectionMatrix();
    const z0 = Math.min(0, ...(useAppStore.getState().app?.levelElevations ?? [0]));
    t.ground.position.set(c.x, c.y, z0 - 3);
    t.ground.scale.setScalar(r * 60);
  }
  t.ground.visible = realistic && !hasSite && !box.isEmpty();
  // The grid dims to a quarter in Realistic.
  for (const g of t.gridGroup.children) {
    for (const l of g instanceof THREE.LineSegments ? [g] : g.children) {
      if (!(l instanceof THREE.LineSegments)) continue;
      const m = l.material as THREE.LineBasicMaterial;
      m.userData.opacity ??= m.opacity;
      m.opacity = (m.userData.opacity as number) * (realistic ? 0.25 : 1);
    }
  }
}

/** A deep tone of an element's colour for its cut (in sRGB, as seen). */
export function cutTone(base: THREE.Color): THREE.Color {
  const hsl = { h: 0, s: 0, l: 0 };
  base.getHSL(hsl, THREE.SRGBColorSpace);
  return new THREE.Color().setHSL(
    hsl.h,
    hsl.s * 0.6,
    Math.min(hsl.l * 0.45, 0.32),
    THREE.SRGBColorSpace,
  );
}

function clearCaps(g: THREE.Group) {
  for (const c of [...g.children]) {
    g.remove(c);
    const o = c as THREE.Mesh;
    o.geometry?.dispose();
    (o.material as THREE.Material | undefined)?.dispose();
  }
}

/** Section box caps (ADR-044), as Revit draws them: each element's cut filled in its cut
 * colour, a heavy cut line round it, and a wall's layer boundaries inside. */
function buildCaps(t: Three, caps: Cap[]) {
  clearCaps(t.capGroup);
  const size = t.renderer.getSize(new THREE.Vector2());
  for (const c of caps) {
    if (c.positions.length) {
      const geo = new THREE.BufferGeometry();
      geo.setAttribute("position", new THREE.Float32BufferAttribute(c.positions, 3));
      geo.computeVertexNormals();
      const fill = new THREE.Mesh(geo, new THREE.MeshBasicMaterial());
      fill.userData = { el: c.el, cap: c, kind: "fill" };
      fill.renderOrder = 2;
      t.capGroup.add(fill);
    }
    if (c.inner.length) {
      const geo = new THREE.BufferGeometry();
      geo.setAttribute("position", new THREE.Float32BufferAttribute(c.inner, 3));
      const inner = new THREE.LineSegments(
        geo,
        new THREE.LineBasicMaterial({ color: 0x3a3d40, depthTest: true }),
      );
      inner.userData = { el: c.el, kind: "inner", cap: c };
      inner.renderOrder = 3;
      t.capGroup.add(inner);
    }
    if (c.outline.length) {
      const geo = new LineSegmentsGeometry().setPositions(c.outline);
      const mat = new LineMaterial({ color: 0x111111, linewidth: 2.2 });
      mat.resolution.set(size.x, size.y);
      const cut = new LineSegments2(geo, mat);
      cut.userData = { el: c.el, kind: "cut", cap: c };
      cut.renderOrder = 4;
      t.capGroup.add(cut);
    }
  }
  t.capGroup.visible = true;
}

/** Cut colours per visual style: Shaded and Realistic fill it in a deep tone of the element's colour
 * (Revit's cut faces read as poch&eacute;, much darker than the surfaces beside them); Consistent Colors shows it flat;
 * Hidden Line leaves the cut white; Wireframe shows only the cut lines. Hidden elements'
 * cuts hide with them; a selected element's cut turns blue. */
function applyCaps(t: Three, style: VisualStyle, selection: string[]) {
  const shown = new Map<string, boolean>();
  for (const c of t.group.children)
    if (c instanceof THREE.Mesh) shown.set(c.userData.el as string, c.visible);
  const sel = new Set(selection);
  const size = t.renderer.getSize(new THREE.Vector2());
  for (const c of t.capGroup.children) {
    const u = c.userData as { el: string; kind: string; cap?: Cap };
    // The ground caps only as a block of earth (ADR-045).
    const ground = u.cap?.category === "Site";
    c.visible = shown.get(u.el) !== false && !(ground && !useAppStore.getState().terrain.solid);
    if (u.kind === "fill") {
      const mat = (c as THREE.Mesh).material as THREE.MeshBasicMaterial;
      const base = new THREE.Color(meshColor(u.cap as unknown as Mesh));
      const color =
        style === "hiddenLine"
          ? new THREE.Color(0xffffff)
          : style === "consistent"
            ? base
            : cutTone(base);
      mat.color.copy(sel.has(u.el) ? new THREE.Color(SELECTED) : color);
      mat.side = THREE.DoubleSide;
      mat.polygonOffset = true;
      mat.polygonOffsetFactor = -1;
      mat.polygonOffsetUnits = -1;
      mat.toneMapped = false;
      c.visible = c.visible && style !== "wireframe";
    } else if (u.kind === "cut") {
      const mat = (c as LineSegments2).material;
      mat.resolution.set(size.x, size.y);
      mat.color.set(sel.has(u.el) ? SELECTED : 0x111111);
    }
  }
}

/** The earth texture of the terrain's sides: soil in faint strata. */
let earthTexture: THREE.Texture | null = null;
function earth(): THREE.Texture | null {
  if (earthTexture || typeof document === "undefined") return earthTexture;
  const c = document.createElement("canvas");
  c.width = 64;
  c.height = 256;
  const g = c.getContext("2d");
  if (!g) return null;
  const bands = ["#8a6a47", "#7d5f3f", "#94734e", "#735636", "#86663f"];
  let y = 0;
  let k = 0;
  while (y < 256) {
    const h = 18 + ((k * 37) % 29);
    g.fillStyle = bands[k % bands.length]!;
    g.fillRect(0, y, 64, h);
    y += h;
    k++;
  }
  for (let i = 0; i < 900; i++) {
    g.fillStyle = i % 2 ? "rgba(0,0,0,0.08)" : "rgba(255,255,255,0.06)";
    g.fillRect((i * 53) % 64, (i * 97) % 256, 1 + (i % 2), 1);
  }
  earthTexture = new THREE.CanvasTexture(c);
  earthTexture.colorSpace = THREE.SRGBColorSpace;
  earthTexture.wrapS = earthTexture.wrapT = THREE.RepeatWrapping;
  return earthTexture;
}

function contourLabel(text: string, major: boolean): THREE.Sprite {
  const c = document.createElement("canvas");
  const g = c.getContext("2d");
  const font = `${major ? 700 : 600} 44px "Barlow Condensed", Arial, sans-serif`;
  let w = 160;
  if (g) {
    g.font = font;
    w = Math.ceil(g.measureText(text).width) + 20;
  }
  c.width = w;
  c.height = 60;
  if (g) {
    g.font = font;
    g.fillStyle = "rgba(255,255,255,0.85)";
    g.fillRect(0, 6, w, 48);
    g.fillStyle = "#3b2f22";
    g.textAlign = "center";
    g.textBaseline = "middle";
    g.fillText(text, w / 2, 32);
  }
  const tex = new THREE.CanvasTexture(c);
  tex.colorSpace = THREE.SRGBColorSpace;
  const s = new THREE.Sprite(
    new THREE.SpriteMaterial({
      map: tex,
      depthTest: true,
      toneMapped: false,
      // A steady size on screen, like annotation text.
      sizeAttenuation: false,
    }),
  );
  // About 12 px tall on screen, sitting just above its contour (not sunk into a slope).
  const h = 0.018;
  s.center.set(0.5, 0);
  s.scale.set((h * w) / 60, h, 1);
  return s;
}

/** The site's terrain (ADR-045): its earth block (sides and bottom) with its outline, its
 * contours (minor thin, every fifth heavier) and their labels. */
function buildTerrain(t: Three, terrain: Terrain | null) {
  for (const c of [...t.terrainGroup.children]) {
    t.terrainGroup.remove(c);
    c.traverse((o) => {
      const m = o as THREE.Mesh;
      m.geometry?.dispose();
      const mat = m.material as (THREE.Material & { map?: THREE.Texture | null }) | undefined;
      if (mat && mat.map && mat.map !== earthTexture) mat.map.dispose();
      mat?.dispose();
    });
  }
  if (!terrain) return;
  if (terrain.skirt.length) {
    const geo = new THREE.BufferGeometry();
    geo.setAttribute("position", new THREE.Float32BufferAttribute(terrain.skirt, 3));
    // Soil strata run level: v up the side, u along it.
    const p = terrain.skirt;
    const uv = new Float32Array((p.length / 3) * 2);
    for (let i = 0, k = 0; i < p.length; i += 3, k += 2) {
      uv[k] = (p[i]! + p[i + 1]!) / 4000;
      uv[k + 1] = p[i + 2]! / 3048;
    }
    geo.setAttribute("uv", new THREE.BufferAttribute(uv, 2));
    geo.computeVertexNormals();
    const block = new THREE.Mesh(
      geo,
      new THREE.MeshLambertMaterial({ map: earth(), side: THREE.DoubleSide }),
    );
    block.userData.kind = "block";
    t.terrainGroup.add(block);
    const edges = new THREE.LineSegments(
      new THREE.EdgesGeometry(geo, 30),
      new THREE.LineBasicMaterial({ color: EDGE_COLOR }),
    );
    edges.userData.kind = "blockEdges";
    t.terrainGroup.add(edges);
  }
  for (const major of [false, true]) {
    const segs = terrain.contours.filter((c) => c.major === major).flatMap((c) => c.segments);
    if (!segs.length) continue;
    const geo = new THREE.BufferGeometry();
    geo.setAttribute("position", new THREE.Float32BufferAttribute(segs, 3));
    const lines = new THREE.LineSegments(
      geo,
      new THREE.LineBasicMaterial({
        color: major ? 0x3f3325 : 0x6d5d48,
        transparent: !major,
        opacity: major ? 1 : 0.7,
      }),
    );
    lines.userData.kind = "contours";
    t.terrainGroup.add(lines);
  }
  if (typeof document !== "undefined") {
    // Many contours: only the heavier ones carry labels.
    const all = terrain.labels.length <= 60;
    const labels = new THREE.Group();
    labels.userData.kind = "labels";
    for (const l of terrain.labels) {
      if (!all && !l.major) continue;
      const s = contourLabel(l.text, l.major);
      s.position.set(l.at[0], l.at[1], l.at[2] + 60);
      labels.add(s);
    }
    t.terrainGroup.add(labels);
  }
}

function applyTerrain(
  t: Three,
  ui: { solid: boolean; contours: boolean; labels: boolean },
  style: VisualStyle,
  box: SectionBox | null,
) {
  const planes = boxPlanes(box);
  for (const c of t.terrainGroup.children) {
    const kind = c.userData.kind as string;
    if (kind === "block") {
      c.visible = ui.solid && style !== "wireframe";
      const mat = (c as THREE.Mesh).material as THREE.MeshLambertMaterial;
      mat.color.setHex(style === "hiddenLine" ? 0xffffff : 0xffffff);
      mat.map = style === "hiddenLine" ? null : earth();
      mat.clippingPlanes = planes;
      mat.needsUpdate = true;
    } else if (kind === "blockEdges") {
      c.visible = ui.solid && style !== "realistic";
      ((c as THREE.LineSegments).material as THREE.Material).clippingPlanes = planes;
    } else if (kind === "contours") {
      c.visible = ui.contours;
      ((c as THREE.LineSegments).material as THREE.Material).clippingPlanes = planes;
    } else if (kind === "labels") {
      c.visible = ui.contours && ui.labels;
      if (box)
        for (const s of c.children)
          s.visible =
            s.position.x >= box.min[0]! &&
            s.position.x <= box.max[0]! &&
            s.position.y >= box.min[1]! &&
            s.position.y <= box.max[1]!;
    }
  }
}
