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
    // A sheet map (ADR-107) answers with what to fetch: the imagery comes from Google now,
    // held only here, never saved.
    .then((url) => (url.startsWith("map:") ? ipc.mapImage(JSON.parse(url.slice(4))) : url))
    .then((url) => {
      const img = new Image();
      img.onload = () => {
        cache.set(id, img);
        listeners.forEach((f) => f());
      };
      img.src = url;
    })
    // Not retried on every redraw (a missing Maps key would ask Google each frame).
    .catch(() => cache.set(id, null));
  return null;
}

/** Calls `f` whenever an image finishes loading; returns the unsubscribe. */
export function onRenderImage(f: () => void): () => void {
  listeners.add(f);
  return () => {
    listeners.delete(f);
  };
}
