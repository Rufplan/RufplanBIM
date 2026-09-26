import * as THREE from "three";
import type { RenderMaterial } from "../bindings/RenderMaterial";
import type { Mesh } from "../ipc";
import { boxUv, physicalMaterial, texturesFor, type TextureLoader } from "./materials";

// 3D view visual styles (ADR-038), after Revit's: Wireframe, Hidden Line, Shaded,
// Consistent Colors and Realistic. Each mesh gets the style's material; the scene around
// it (tone mapping, light, sky, shadows, ground) is set by the view.

export type VisualStyle = "wireframe" | "hiddenLine" | "shaded" | "consistent" | "realistic";

export const VISUAL_STYLES: { id: VisualStyle; label: string }[] = [
  { id: "wireframe", label: "Wireframe" },
  { id: "hiddenLine", label: "Hidden Line" },
  { id: "shaded", label: "Shaded" },
  { id: "consistent", label: "Consistent Colors" },
  { id: "realistic", label: "Realistic" },
];

export const EDGE_COLOR = 0x1b1d1f;
export const SELECTED = 0x3ecff7;

/** What a mesh in the view carries to be restyled. */
export interface MeshInfo {
  mesh: Mesh;
  /** Its colour in Shaded and Consistent Colors (the satellite image makes it white). */
  base: number;
  /** A draped satellite image (Site meshes, ADR-026). */
  map: THREE.Texture | null;
}

/** A project material ready for Realistic: its physical material and texture scale. */
export interface RealMaterial {
  material: THREE.MeshPhysicalMaterial;
  scale: number;
  aspect: number;
  textured: boolean;
}

export function isGlass(m: Pick<Mesh, "category" | "color">): boolean {
  return (m.category === "Window" || m.category === "Door") && !m.color;
}

function opacityOf(m: Mesh): number {
  return m.category === "Ceiling" ? 0.55 : isGlass(m) ? 0.45 : 1;
}

const flat = { polygonOffset: true, polygonOffsetFactor: 1, polygonOffsetUnits: 1 };

/** A category's surface in Realistic when it has no project material. */
function fallback(info: MeshInfo, planes: THREE.Plane[]): THREE.MeshStandardMaterial {
  const m = info.mesh;
  if (isGlass(m))
    return new THREE.MeshStandardMaterial({
      color: 0x4f6a78,
      metalness: 0.9,
      roughness: 0.05,
      transparent: true,
      opacity: 0.55,
      envMapIntensity: 1.6,
      clippingPlanes: planes,
      ...flat,
    });
  const rough: Record<string, number> = {
    Roof: 0.8,
    Floor: 0.6,
    Stair: 0.6,
    Site: 1,
    Beam: 0.4,
    Railing: 0.35,
    Door: 0.45,
    Window: 0.4,
  };
  const metal = m.category === "Beam" || m.category === "Railing" ? 0.7 : 0;
  return new THREE.MeshStandardMaterial({
    color: info.base,
    map: info.map,
    roughness: rough[m.category] ?? 0.85,
    metalness: metal,
    transparent: m.category === "Ceiling",
    opacity: opacityOf(m),
    clippingPlanes: planes,
    ...flat,
  });
}

/** The material a mesh shows in a visual style. Wireframe draws no faces, but the
 * invisible material still lets the mesh be picked. */
export function styleMaterial(
  style: VisualStyle,
  info: MeshInfo,
  planes: THREE.Plane[],
  real: RealMaterial | null,
): THREE.Material {
  const m = info.mesh;
  const common = {
    transparent: opacityOf(m) < 1,
    opacity: opacityOf(m),
    clippingPlanes: planes,
    toneMapped: false,
    ...flat,
  };
  switch (style) {
    case "wireframe":
      return new THREE.MeshBasicMaterial({ visible: false, clippingPlanes: planes });
    case "hiddenLine":
      return new THREE.MeshBasicMaterial({ color: 0xffffff, ...common });
    case "consistent":
      return new THREE.MeshBasicMaterial({ color: info.base, map: info.map, ...common });
    case "realistic": {
      if (real && m.category !== "Site") {
        const mat = real.material.clone();
        Object.assign(mat, { clippingPlanes: planes, ...flat });
        return mat;
      }
      return fallback(info, planes);
    }
    default:
      return new THREE.MeshLambertMaterial({ color: info.base, map: info.map, ...common });
  }
}

/** Real-world texture coordinates for a z-up mesh, from Rendering's box mapping (which
 * works y-up). */
export function realUv(geo: THREE.BufferGeometry, scale: number, aspect: number) {
  const pos = geo.getAttribute("position");
  const yUp = new Float32Array(pos.count * 3);
  for (let i = 0; i < pos.count; i++) {
    yUp[i * 3] = pos.getX(i);
    yUp[i * 3 + 1] = pos.getZ(i);
    yUp[i * 3 + 2] = -pos.getY(i);
  }
  const tmp = new THREE.BufferGeometry();
  tmp.setAttribute("position", new THREE.BufferAttribute(yUp, 3));
  boxUv(tmp, scale, aspect);
  geo.setAttribute("uv", tmp.getAttribute("uv"));
  // Tangents back to z-up: (x, y, z) y-up is (x, -z, y) here.
  const t = tmp.getAttribute("tangent");
  const tan = new Float32Array(t.count * 4);
  for (let i = 0; i < t.count; i++) {
    tan.set([t.getX(i), -t.getZ(i), t.getY(i), 1], i * 4);
  }
  geo.setAttribute("tangent", new THREE.BufferAttribute(tan, 4));
  tmp.dispose();
}

/** The project's materials used by `meshes`, with their textures (photo sets download
 * once), for Realistic. */
export async function realMaterials(
  meshes: Mesh[],
  materials: RenderMaterial[],
  load: TextureLoader,
): Promise<Map<string, RealMaterial>> {
  const used = new Set(meshes.map((m) => m.material).filter((x): x is string => !!x));
  const out = new Map<string, RealMaterial>();
  for (const m of materials.filter((x) => used.has(x.id))) {
    // A texture that can't download shows as the material's colour and gloss alone.
    const tex = await texturesFor(m.appearance, m.color, load).catch(() => null);
    out.set(m.id, {
      material: physicalMaterial(m.appearance, m.color, tex),
      scale: m.appearance.scale,
      aspect: tex?.aspect ?? 1,
      textured: !!tex,
    });
  }
  return out;
}

/** Realistic's sky: a soft blue gradient to a pale horizon. */
export function skyTexture(): THREE.Texture | null {
  if (typeof document === "undefined") return null;
  const c = document.createElement("canvas");
  c.width = 4;
  c.height = 256;
  const g = c.getContext("2d");
  if (!g) return null;
  const grad = g.createLinearGradient(0, 0, 0, 256);
  grad.addColorStop(0, "#9fc4dc");
  grad.addColorStop(0.55, "#d8e7ef");
  grad.addColorStop(1, "#eef2f1");
  g.fillStyle = grad;
  g.fillRect(0, 0, 4, 256);
  const t = new THREE.CanvasTexture(c);
  t.colorSpace = THREE.SRGBColorSpace;
  return t;
}
