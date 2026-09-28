// The Realistic 3D view's path-traced scene (ADR-062): the model with its project
// materials, the site's sky and sun from Sun Settings, and at night its lit fixtures; the
// same pipeline as Render, without the render's backdrop and ground photo options.
import type * as THREE from "three";
import { ipc } from "../ipc";
import { useAppStore } from "../store";

export interface RealScene {
  scene: THREE.Scene;
  /** Tone-mapping exposure: daylight 1, night (lit by the fixtures) brighter. */
  exposure: number;
}

export async function realScene(viewId: string): Promise<RealScene> {
  const [pt, sky] = await Promise.all([import("./pathtrace"), import("./sky")]);
  const [meshes, sun, materials] = await Promise.all([
    ipc.meshes(viewId),
    ipc.sunNow().catch(() => null),
    ipc.renderMaterials(),
  ]);
  const up = sun && sun.altitude > 0 ? sun : null;
  const env = sky.physicalSky({
    sunDir: up ? pt.toYUp(up.dir) : pt.toYUp([0, -1, -0.2]),
    altitude: up ? up.altitude : -6,
  });
  const materialOf = await pt.prepareMaterials(meshes, materials, (set, map) =>
    ipc.materialTexture(set, map),
  );
  // After dark the building's own lights carry the scene.
  const lights = up ? [] : (await ipc.lights(viewId)).filter((l) => l.on && l.lumens > 0);
  const levels = useAppStore.getState().app?.levelElevations ?? [0];
  const scene = pt.buildScene(meshes, {
    materialOf,
    environment: env,
    environmentIntensity: 1,
    rotation: 0,
    ground: "grass",
    projection: null,
    imagery: null,
    groundZ: Math.min(0, ...levels),
    lights,
  });
  // The sky shows behind the model, as it lights it.
  scene.background = env;
  return { scene, exposure: up ? 0.85 : 12 };
}
