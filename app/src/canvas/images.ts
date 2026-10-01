// Saved renderings drawn in views and on sheets (ADR-095): each image is fetched once
// by element id, and views redraw when one arrives.
import { ipc } from "../ipc";
import type { ElementId } from "../bindings/ElementId";

const cache = new Map<ElementId, HTMLImageElement | null>();
const listeners = new Set<() => void>();

/** The loaded image, or null while it loads (a redraw follows). */
export function renderImage(id: ElementId): HTMLImageElement | null {
  const hit = cache.get(id);
  if (hit !== undefined) return hit && hit.complete ? hit : null;
  cache.set(id, null);
  ipc
    .renderImage(id)
    .then((url) => {
      const img = new Image();
      img.onload = () => {
        cache.set(id, img);
        listeners.forEach((f) => f());
      };
      img.src = url;
    })
    .catch(() => cache.delete(id));
  return null;
}

/** Calls `f` whenever an image finishes loading; returns the unsubscribe. */
export function onRenderImage(f: () => void): () => void {
  listeners.add(f);
  return () => {
    listeners.delete(f);
  };
}
