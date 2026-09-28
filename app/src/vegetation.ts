// The Vegetation tab's actions (ADR-064): placing plants the way Enscape's Asset Library
// does (random rotation and size), the season of every deciduous tree at once, and the
// base ground.
import type { PlantAt } from "./bindings/PlantAt";
import type { Season } from "./bindings/Season";
import { apply } from "./fileActions";
import { ipc, type ElementId, type Pt } from "./ipc";
import { useAppStore } from "./store";

/** A plant at `at`, turned and sized per the options bar. */
export function plantPlacement(at: Pt, rand: () => number = Math.random): PlantAt {
  const o = useAppStore.getState().options;
  const v = Math.max(0, Math.min(50, o.plantSizeVariation)) / 100;
  return {
    at,
    rotation: o.plantRandomRotation ? rand() * Math.PI * 2 : 0,
    scale: 1 + (rand() * 2 - 1) * v,
  };
}

/** Places the current plant at `at` (on the plan's level, or `level` from 3D). */
export async function placePlant(view: ElementId, at: Pt, level: ElementId | null = null) {
  const s = useAppStore.getState();
  const type = s.toolTypes.plant;
  if (!type) {
    s.setError("Pick a plant in the Asset Library first.");
    return false;
  }
  return apply(() => ipc.createPlants(view, type, [plantPlacement(at)], level));
}

/** Sets the season of every planting type that changes with them (Enscape's season
 * variants for the whole site at once). */
export async function setSeason(season: Season) {
  const types = useAppStore.getState().app?.plantingTypes ?? [];
  for (const t of types) {
    const sheet = await ipc.properties(t.id).catch(() => null);
    const prop = sheet?.properties.find((p) => p.key === "season");
    if (prop && prop.value !== season) {
      if (!(await apply(() => ipc.setProperty(t.id, "season", season)))) return;
    }
  }
}
