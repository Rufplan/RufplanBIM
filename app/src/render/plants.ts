// Plants in 3D (ADR-064), after Enscape's assets: each species' model (grown in Rust) with
// its foliage atlas and bark, instanced in the live Realistic view and merged into the
// path tracer's scene. Leaf cards are alpha-cut, lit with their crown's soft normals on
// both sides (so a crown reads as a volume, not a pile of cards) and sway a little in the
// wind in the live view.
import * as THREE from "three";
import { mergeGeometries } from "three/examples/jsm/utils/BufferGeometryUtils.js";
import type { PlantInstance } from "../bindings/PlantInstance";
import type { PlantMap } from "../bindings/PlantMap";
import type { PlantModel } from "../bindings/PlantModel";
import type { PlantPart } from "../bindings/PlantPart";
import type { PlantSource } from "../bindings/PlantSource";
import { imageTexture } from "./materials";

export interface PlantLoader {
  model: (source: PlantSource, variant: number) => Promise<PlantModel>;
  texture: (source: PlantSource, map: PlantMap, size: number | null) => Promise<ArrayBuffer>;
}

export interface PlantAssets {
  model: PlantModel;
  bark: THREE.BufferGeometry | null;
  leaves: THREE.BufferGeometry | null;
  solid: THREE.BufferGeometry | null;
  materials: PlantMaterials;
}

export interface PlantMaterials {
  bark: THREE.MeshStandardMaterial;
  leaves: THREE.MeshPhysicalMaterial;
  solid: THREE.MeshStandardMaterial;
}

export const sourceKey = (s: PlantSource) => ("Type" in s ? `t:${s.Type}` : `p:${s.Preset}`);

const srgbToLinear = (c: number) => (c <= 0.04045 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4));

/** A part's geometry; `srgb` colours (the solid parts') are made linear for three. */
export function partGeometry(p: PlantPart, srgb = false): THREE.BufferGeometry | null {
  if (p.indices.length === 0) return null;
  const g = new THREE.BufferGeometry();
  g.setAttribute("position", new THREE.Float32BufferAttribute(p.positions, 3));
  g.setAttribute("normal", new THREE.Float32BufferAttribute(p.normals, 3));
  g.setAttribute("uv", new THREE.Float32BufferAttribute(p.uvs, 2));
  const colors = srgb ? p.colors.map(srgbToLinear) : p.colors;
  g.setAttribute("color", new THREE.Float32BufferAttribute(colors, 3));
  g.setIndex(p.indices);
  g.computeBoundingSphere();
  return g;
}

/** Leaves keep their crown normal on their back faces too (three flips it for
 * double-sided materials), so a crown is lit as one soft volume. */
function unflipBackFaces(m: THREE.Material) {
  m.onBeforeCompile = (shader) => {
    const chunk = THREE.ShaderChunk.normal_fragment_begin.replace("normal *= faceDirection;", "");
    shader.fragmentShader = shader.fragmentShader.replace(
      "#include <normal_fragment_begin>",
      chunk,
    );
  };
  m.customProgramCacheKey = () => "plant-leaves";
}

/** Materials for a plant's parts. */
export function plantMaterials(
  atlas: THREE.Texture | null,
  bark: THREE.Texture | null,
  barkNormal: THREE.Texture | null,
): PlantMaterials {
  // Leaves reflect little: a waxy sheen, not the white rim full Fresnel gives cards seen
  // edge-on.
  const leaves = new THREE.MeshPhysicalMaterial({
    map: atlas,
    alphaTest: 0.5,
    side: THREE.DoubleSide,
    vertexColors: true,
    roughness: 0.82,
    metalness: 0,
    specularIntensity: 0.28,
  });
  unflipBackFaces(leaves);
  return {
    leaves,
    bark: new THREE.MeshStandardMaterial({
      map: bark,
      normalMap: barkNormal,
      normalScale: new THREE.Vector2(1.2, 1.2),
      vertexColors: true,
      roughness: 0.92,
      metalness: 0,
    }),
    solid: new THREE.MeshStandardMaterial({
      vertexColors: true,
      roughness: 0.62,
      metalness: 0,
      side: THREE.DoubleSide,
    }),
  };
}

/** Alpha-cut foliage keeps its coverage when it shrinks: each mipmap level's alpha is
 * scaled so the same share of it stays above the cut-off (Castaño's coverage-preserving
 * mipmaps). Otherwise thin needles and narrow leaves fade away in the distance. */
export function coverageMips(t: THREE.Texture, cutoff = 0.5): THREE.Texture {
  const img = t.image as CanvasImageSource & { width: number; height: number };
  if (typeof document === "undefined" || !img?.width) return t;
  const levels: HTMLCanvasElement[] = [];
  let w = img.width;
  let h = img.height;
  const first = document.createElement("canvas");
  first.width = w;
  first.height = h;
  const fc = first.getContext("2d", { willReadFrequently: true });
  if (!fc) return t;
  fc.drawImage(img, 0, 0);
  const cut = cutoff * 255;
  const covered = (d: Uint8ClampedArray, k: number) => {
    let n = 0;
    for (let i = 3; i < d.length; i += 4) if (d[i]! * k > cut) n++;
    return n / (d.length / 4);
  };
  const target = covered(fc.getImageData(0, 0, w, h).data, 1);
  levels.push(first);
  let prev = first;
  while (w > 1 || h > 1) {
    w = Math.max(1, w >> 1);
    h = Math.max(1, h >> 1);
    const c = document.createElement("canvas");
    c.width = w;
    c.height = h;
    const ctx = c.getContext("2d", { willReadFrequently: true });
    if (!ctx) return t;
    ctx.imageSmoothingQuality = "high";
    ctx.drawImage(prev, 0, 0, w, h);
    const data = ctx.getImageData(0, 0, w, h);
    let lo = 0.5;
    let hi = 8;
    for (let k = 0; k < 12; k++) {
      const mid = (lo + hi) / 2;
      if (covered(data.data, mid) < target) lo = mid;
      else hi = mid;
    }
    const s = (lo + hi) / 2;
    for (let i = 3; i < data.data.length; i += 4) data.data[i] = Math.min(255, data.data[i]! * s);
    ctx.putImageData(data, 0, 0);
    levels.push(c);
    prev = c;
  }
  t.image = levels[0];
  t.mipmaps = levels as unknown as THREE.Texture["mipmaps"];
  t.generateMipmaps = false;
  t.minFilter = THREE.LinearMipmapLinearFilter;
  t.needsUpdate = true;
  return t;
}

const assets = new Map<string, Promise<PlantAssets>>();

/** A plant's model and textures, loaded once per source, variant and texture size. */
export function plantAssets(
  source: PlantSource,
  variant: number,
  load: PlantLoader,
  size: number | null = null,
  stamp = "",
): Promise<PlantAssets> {
  const key = `${sourceKey(source)}:${stamp}:${variant}:${size ?? "full"}`;
  let p = assets.get(key);
  if (!p) {
    p = (async () => {
      const tex = (map: PlantMap, srgb: boolean) =>
        load
          .texture(source, map, size)
          .then((b) => imageTexture(b, srgb))
          .catch(() => null);
      const [model, atlas, bark, normal] = await Promise.all([
        load.model(source, variant),
        tex("Foliage", true),
        tex("Bark", true),
        tex("BarkNormal", false),
      ]);
      if (atlas) {
        coverageMips(atlas);
        // One atlas: no wrapping bleed between cells.
        atlas.wrapS = THREE.ClampToEdgeWrapping;
        atlas.wrapT = THREE.ClampToEdgeWrapping;
      }
      return {
        model,
        bark: partGeometry(model.bark),
        leaves: partGeometry(model.leaves),
        solid: partGeometry(model.solid, true),
        materials: plantMaterials(atlas, bark, normal),
      };
    })();
    p.catch(() => assets.delete(key));
    assets.set(key, p);
  }
  return p;
}

/** Forgets cached plants of a type (its spec changed). */
export function forgetPlants(prefix: string) {
  for (const k of [...assets.keys()]) if (k.startsWith(prefix)) assets.delete(k);
}

/** An instance's placement (z up). */
export function instanceMatrix(i: Pick<PlantInstance, "at" | "rotation" | "scale">) {
  const m = new THREE.Matrix4().makeRotationZ(i.rotation);
  m.scale(new THREE.Vector3(i.scale, i.scale, i.scale));
  m.setPosition(i.at[0], i.at[1], i.at[2]);
  return m;
}

/** Wind (Enscape's gentle sway): leaves and twigs move more the higher and farther out
 * they are. One clock for every plant material. */
export const wind = { time: { value: 0 } };

function addWind(m: THREE.Material, strength: number) {
  const prev = m.onBeforeCompile;
  m.onBeforeCompile = (shader, r) => {
    prev?.call(m, shader, r);
    shader.uniforms.uWind = wind.time;
    shader.vertexShader = shader.vertexShader
      .replace("#include <common>", "#include <common>\nuniform float uWind;")
      .replace(
        "#include <begin_vertex>",
        `#include <begin_vertex>
        {
          float h = max(transformed.z, 0.0) * 0.00025;
          float r = length(transformed.xy) * 0.0002;
          #ifdef USE_INSTANCING
            vec3 base = instanceMatrix[3].xyz;
          #else
            vec3 base = vec3(0.0);
          #endif
          float ph = base.x * 0.0003 + base.y * 0.0002;
          float s = sin(uWind * 1.3 + ph + transformed.z * 0.0004) * 0.6
                  + sin(uWind * 2.9 + ph * 1.7 + transformed.x * 0.002) * 0.25;
          transformed.xy += vec2(s, s * 0.6) * h * h * ${strength.toFixed(1)}
                          + vec2(s) * r * ${(strength * 0.4).toFixed(1)};
        }`,
      );
  };
  const key = m.customProgramCacheKey.bind(m);
  m.customProgramCacheKey = () => `${key()}-wind${strength}`;
}

/** The live view's plants: every placed instance of every species, instanced per part.
 * Instances whose assets aren't loaded yet are left out. */
export function plantGroup(
  entries: { assets: PlantAssets; instances: PlantInstance[] }[],
  swaying = true,
): THREE.Group {
  const g = new THREE.Group();
  g.name = "plants";
  for (const { assets: a, instances } of entries) {
    if (!instances.length) continue;
    const parts: [THREE.BufferGeometry | null, THREE.Material][] = [
      [a.bark, a.materials.bark],
      [a.leaves, a.materials.leaves],
      [a.solid, a.materials.solid],
    ];
    for (const [geo, mat] of parts) {
      if (!geo) continue;
      const m = swaying ? mat.clone() : mat;
      if (swaying) {
        if (mat === a.materials.leaves) {
          unflipBackFaces(m);
          addWind(m, 60);
        } else addWind(m, 12);
        (m as THREE.MeshStandardMaterial).alphaToCoverage = mat === a.materials.leaves;
      }
      const mesh = new THREE.InstancedMesh(geo, m, instances.length);
      instances.forEach((inst, k) => mesh.setMatrixAt(k, instanceMatrix(inst)));
      mesh.instanceMatrix.needsUpdate = true;
      mesh.computeBoundingSphere();
      mesh.castShadow = true;
      mesh.receiveShadow = true;
      mesh.userData.plants = instances.map((i) => i.el);
      g.add(mesh);
    }
  }
  return g;
}

/** Plants for the path tracer: each species' parts with every instance baked in and
 * merged, turned into the render's y-up world. */
export function plantMeshesYUp(
  entries: { assets: PlantAssets; instances: PlantInstance[] }[],
): THREE.Mesh[] {
  const toYUp = new THREE.Matrix4().makeRotationX(-Math.PI / 2);
  const out: THREE.Mesh[] = [];
  for (const { assets: a, instances } of entries) {
    if (!instances.length) continue;
    const parts: [THREE.BufferGeometry | null, THREE.Material][] = [
      [a.bark, a.materials.bark],
      [a.leaves, a.materials.leaves],
      [a.solid, a.materials.solid],
    ];
    for (const [geo, mat] of parts) {
      if (!geo) continue;
      const pieces = instances.map((i) => {
        const g = geo.clone();
        g.applyMatrix4(toYUp.clone().multiply(instanceMatrix(i)));
        return g;
      });
      const merged = pieces.length === 1 ? pieces[0]! : mergeGeometries(pieces, false);
      if (pieces.length > 1) for (const p of pieces) p.dispose();
      if (!merged) continue;
      const mesh = new THREE.Mesh(merged, mat);
      mesh.userData.plant = true;
      out.push(mesh);
    }
  }
  return out;
}

/** Loads every instance's assets (grouped by type and variant) for a view or a render. */
export async function loadPlantEntries(
  instances: PlantInstance[],
  load: PlantLoader,
  size: number | null = null,
): Promise<{ assets: PlantAssets; instances: PlantInstance[] }[]> {
  const groups = new Map<string, PlantInstance[]>();
  for (const i of instances) {
    const k = `${i.type_id}:${i.spec_key}:${i.variant}`;
    const list = groups.get(k);
    if (list) list.push(i);
    else groups.set(k, [i]);
  }
  const out = await Promise.all(
    [...groups.values()].map(async (list) => {
      const first = list[0]!;
      const a = await plantAssets(
        { Type: first.type_id },
        first.variant,
        load,
        size,
        first.spec_key,
      ).catch(() => null);
      return a ? { assets: a, instances: list } : null;
    }),
  );
  return out.filter((x): x is { assets: PlantAssets; instances: PlantInstance[] } => !!x);
}
