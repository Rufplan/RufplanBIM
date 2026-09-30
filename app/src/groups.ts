import type { AppState } from "./bindings/AppState";
import type { ElementId } from "./bindings/ElementId";

// Selection helpers for groups (ADR-087), kept free of the store (which uses them).

/** The group an element belongs to, unless that group is being edited. */
export function groupOf(app: AppState | null, id: ElementId): ElementId | null {
  if (!app) return null;
  const g = app.groups.find((x) => x.members.includes(id));
  return g && g.id !== app.editingGroup ? g.id : null;
}

/** Revit selects the group when you click one of its members. */
export function toGroups(app: AppState | null, ids: ElementId[]): ElementId[] {
  if (!app || app.groups.length === 0) return ids;
  const out: ElementId[] = [];
  for (const id of ids) {
    const g = groupOf(app, id) ?? id;
    if (!out.includes(g)) out.push(g);
  }
  return out;
}

/** Selected groups light up their members. */
export function withMembers(app: AppState | null | undefined, ids: ElementId[]): ElementId[] {
  if (!app || app.groups.length === 0) return ids;
  const out = [...ids];
  for (const id of ids) {
    const g = app.groups.find((x) => x.id === id);
    if (g) out.push(...g.members);
  }
  return out;
}
