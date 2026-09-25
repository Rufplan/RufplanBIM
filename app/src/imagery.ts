// The site's satellite overlay (ADR-026): one Google Static Maps image covering the
// topography, fetched through Rust and kept in memory only (Google's terms don't allow
// storing its imagery).
import type { ImageryFrame } from "./bindings/ImageryFrame";
import { ipc } from "./ipc";

export interface Imagery {
  frame: ImageryFrame;
  image: HTMLImageElement;
}

let cache: { key: string; p: Promise<{ image: HTMLImageElement; url: string }> } | null = null;

async function fetchImage(frame: ImageryFrame) {
  const bytes = await ipc.siteImagery(frame);
  const url = URL.createObjectURL(new Blob([bytes], { type: "image/jpeg" }));
  const image = new Image();
  image.src = url;
  await image.decode();
  return { image, url };
}

/** The satellite image for the site as it is now. The same image is reused while only the
 * site's placement (offset, angle) changes. */
export async function siteImagery(): Promise<Imagery> {
  const frame = await ipc.siteImageryFrame();
  const key = JSON.stringify([frame.lat, frame.lon, frame.zoom, frame.width, frame.height]);
  if (!cache || cache.key !== key) {
    const old = cache;
    const p = fetchImage(frame);
    cache = { key, p };
    p.catch(() => {
      if (cache?.p === p) cache = null;
    });
    void old?.p.then(
      (o) => URL.revokeObjectURL(o.url),
      () => {},
    );
  }
  const { image } = await cache.p;
  return { frame, image };
}

/** Texture coordinates of plan point (x, y) on the image: (0, 0) at its lower left, (1, 1)
 * at its upper right. */
export function uvAt(frame: ImageryFrame, x: number, y: number): [number, number] {
  const [ll, lr, , ul] = frame.corners;
  const ux = lr.x - ll.x;
  const uy = lr.y - ll.y;
  const vx = ul.x - ll.x;
  const vy = ul.y - ll.y;
  const dx = x - ll.x;
  const dy = y - ll.y;
  return [(dx * ux + dy * uy) / (ux * ux + uy * uy), (dx * vx + dy * vy) / (vx * vx + vy * vy)];
}

/** Forget the cached image (tests). */
export function resetImagery() {
  cache = null;
}
