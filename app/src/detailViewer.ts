// The Detail Viewer's zoom and pan (the enlarged detail before inserting): 100% is the
// drawing fitted to the canvas, zoom about a point keeps that point still.

export const ZOOM_MIN = 0.5;
export const ZOOM_MAX = 8;

export interface ViewState {
  zoom: number;
  /** Pan, in screen px, from the canvas's centre. */
  tx: number;
  ty: number;
}

export const FIT: ViewState = { zoom: 1, tx: 0, ty: 0 };

/** Zooms to `nz` (clamped) about the point (`px`, `py`), screen px from the canvas centre. */
export function zoomAt(v: ViewState, nz: number, px = 0, py = 0): ViewState {
  const zoom = Math.min(ZOOM_MAX, Math.max(ZOOM_MIN, nz));
  const k = zoom / v.zoom;
  return { zoom, tx: px - (px - v.tx) * k, ty: py - (py - v.ty) * k };
}

/** The wheel's zoom: `deltaY` of a wheel event as a factor. */
export const wheelFactor = (deltaY: number) => Math.exp(-deltaY * 0.0015);

/** A drawing `w` × `h` fitted inside `boxW` × `boxH` (px). */
export function fitSize(w: number, h: number, boxW: number, boxH: number): [number, number] {
  if (!(w > 0 && h > 0 && boxW > 0 && boxH > 0)) return [0, 0];
  const k = Math.min(boxW / w, boxH / h);
  return [w * k, h * k];
}

/** The id `d` places from `current` in `ids` (wrapping), or the first if `current` isn't
 * in it; null for none. */
export function step(ids: string[], current: string | null, d: number): string | null {
  if (!ids.length) return null;
  const i = current ? ids.indexOf(current) : -1;
  if (i < 0) return ids[0]!;
  return ids[(((i + d) % ids.length) + ids.length) % ids.length]!;
}
