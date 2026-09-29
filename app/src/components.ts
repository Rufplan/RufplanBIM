// Detail components (ADR-071): Revit's Detail Component, Repeating Detail and Insulation
// tools. The families and their geometry are Rust's; this keeps the type list and the
// tool's settings.
import type { ComponentTypeInfo } from "./bindings/ComponentTypeInfo";
import { ipc } from "./ipc";
import { useAppStore } from "./store";

let types: Promise<ComponentTypeInfo[]> | null = null;

/** The component types, fetched once. */
export function componentTypes(): Promise<ComponentTypeInfo[]> {
  types ??= ipc.detailComponentTypes();
  types.catch(() => (types = null));
  return types;
}

/** Starts placing a component: the given type (Repeating Detail: brick; Insulation: a
 * batt), else the last one used. */
export function startComponent(key?: string) {
  const s = useAppStore.getState();
  if (key) s.setOption("componentKey", key);
  s.setTool("component");
}

/** A point-based component's direction for a rotation in degrees. */
export function rotationDir(deg: number) {
  const a = (deg * Math.PI) / 180;
  return { x: Math.cos(a), y: Math.sin(a) };
}
