import { useEffect, useLayoutEffect, useRef, useState } from "react";
import type { DetailInfo } from "../bindings/DetailInfo";
import { FIT, fitSize, step, wheelFactor, zoomAt, type ViewState } from "../detailViewer";
import { DetailThumb } from "./DetailThumb";
import { useDetailPreview } from "./DetailPreview";

// The Detail Library's enlarged viewer (the design handoff "Detail Library — Enlarged Detail
// Viewer"): the detail's drawing and notes large enough to read before inserting. Wheel
// zooms toward the cursor, drag pans, double-click zooms, ← → browse the library's
// filtered list (sharing its selection), Esc closes.

export function DetailViewer({
  details,
  current,
  onPick,
  onClose,
  onInsert,
}: {
  /** The library's filtered list, in its order: what ← → browse. */
  details: DetailInfo[];
  current: string;
  onPick: (id: string) => void;
  onClose: () => void;
  onInsert: (id: string) => void;
}) {
  const d = details.find((x) => x.id === current) ?? null;
  const dl = useDetailPreview(current);
  // Zoom and pan belong to the detail shown: another opens fitted.
  const [held, setHeld] = useState<{ id: string; v: ViewState }>({ id: current, v: FIT });
  const view = held.id === current ? held.v : FIT;
  const setView = (u: ViewState | ((v: ViewState) => ViewState)) =>
    setHeld((s) => {
      const base = s.id === current ? s.v : FIT;
      return { id: current, v: typeof u === "function" ? u(base) : u };
    });
  const [drag, setDrag] = useState<{ x: number; y: number; tx: number; ty: number } | null>(null);
  const canvas = useRef<HTMLDivElement>(null);
  const layer = useRef<HTMLDivElement>(null);
  const [box, setBox] = useState<[number, number]>([0, 0]);

  // The drawing's layer size, to fit the sheet to it.
  useLayoutEffect(() => {
    const el = layer.current;
    if (!el) return;
    const measure = () => setBox([el.clientWidth, el.clientHeight]);
    measure();
    const ro = new ResizeObserver(measure);
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  const ids = details.map((x) => x.id);
  const browse = (k: number) => {
    const next = step(ids, current, k);
    if (next) onPick(next);
  };

  // A point (screen px) from the canvas's centre.
  const local = (e: { clientX: number; clientY: number }): [number, number] => {
    const r = canvas.current?.getBoundingClientRect();
    if (!r) return [0, 0];
    return [e.clientX - r.left - r.width / 2, e.clientY - r.top - r.height / 2];
  };

  // The wheel needs a non-passive listener to keep the page from scrolling.
  useEffect(() => {
    const el = canvas.current;
    if (!el) return;
    const onWheel = (e: WheelEvent) => {
      e.preventDefault();
      const [px, py] = local(e);
      setView((v) => zoomAt(v, v.zoom * wheelFactor(e.deltaY), px, py));
    };
    el.addEventListener("wheel", onWheel, { passive: false });
    return () => el.removeEventListener("wheel", onWheel);
  });

  // Keys: the viewer's own, ahead of the library's and the app's.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const t = e.target as HTMLElement | null;
      if (t && (t.tagName === "INPUT" || t.tagName === "TEXTAREA" || t.isContentEditable)) return;
      let handled = true;
      if (e.key === "Escape") onClose();
      else if (e.key === "ArrowLeft") browse(-1);
      else if (e.key === "ArrowRight") browse(1);
      else if (e.key === "+" || e.key === "=") setView((v) => zoomAt(v, v.zoom * 1.4));
      else if (e.key === "-") setView((v) => zoomAt(v, v.zoom / 1.4));
      else if (e.key === "0") setView(FIT);
      else handled = false;
      if (handled) {
        e.preventDefault();
        e.stopPropagation();
      }
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  });

  const [x0, y0, x1, y1] = dl?.bounds ?? [0, 0, 0, 0];
  const size = dl ? fitSize(x1 - x0, y1 - y0, box[0], box[1]) : null;
  const pos = ids.indexOf(current);

  return (
    <div className="dv-backdrop" role="dialog" aria-label="Detail viewer" onClick={onClose}>
      <div className="dv-panel" onClick={(e) => e.stopPropagation()}>
        <div className="dv-toolbar">
          <div className="dv-title">
            <span className="dv-name">{d?.name ?? ""}</span>
            <span className="dv-scale">{d?.scaleLabel ?? ""}</span>
            <span className="dv-counter">{pos >= 0 ? `${pos + 1} of ${ids.length}` : ""}</span>
          </div>
          <div className="dv-zoom" role="group" aria-label="Zoom">
            <button
              onClick={() => setView((v) => zoomAt(v, v.zoom / 1.4))}
              title="Zoom out (−)"
              aria-label="Zoom out"
            >
              −
            </button>
            <span aria-label="Zoom level">{Math.round(view.zoom * 100)}%</span>
            <button
              onClick={() => setView((v) => zoomAt(v, v.zoom * 1.4))}
              title="Zoom in (+)"
              aria-label="Zoom in"
            >
              +
            </button>
          </div>
          <div className="dv-presets">
            <button className="dv-btn" onClick={() => setView(FIT)} title="Fit (0)">
              Fit
            </button>
            <button
              className="dv-btn"
              onClick={() => setView((v) => zoomAt(v, 3))}
              title="Zoom 300%"
            >
              300%
            </button>
          </div>
          <button
            className="dv-close"
            onClick={onClose}
            title="Close (Esc)"
            aria-label="Close viewer"
          >
            ×
          </button>
        </div>
        <div
          ref={canvas}
          className={`dv-canvas${drag ? " dragging" : ""}`}
          onPointerDown={(e) => {
            if (e.button !== 0) return;
            (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
            setDrag({ x: e.clientX, y: e.clientY, tx: view.tx, ty: view.ty });
          }}
          onPointerMove={(e) => {
            if (!drag) return;
            setView((v) => ({
              ...v,
              tx: drag.tx + e.clientX - drag.x,
              ty: drag.ty + e.clientY - drag.y,
            }));
          }}
          onPointerUp={() => setDrag(null)}
          onPointerCancel={() => setDrag(null)}
          onDoubleClick={(e) => {
            if (view.zoom > 1.05) setView(FIT);
            else {
              const [px, py] = local(e);
              setView((v) => zoomAt(v, 2.5, px, py));
            }
          }}
        >
          <div
            ref={layer}
            className="dv-layer"
            style={{
              transform: `translate(${view.tx}px, ${view.ty}px) scale(${view.zoom})`,
              transition: drag ? "none" : undefined,
            }}
          >
            {dl && size ? (
              <DetailThumb
                dl={dl}
                label={`${d?.name ?? "Detail"} enlarged`}
                text
                className="dv-sheet"
                size={size}
              />
            ) : (
              <div className="dv-sheet dl-loading" role="img" aria-label="Loading the detail" />
            )}
          </div>
          <button
            className="dv-nav dv-prev"
            onClick={() => browse(-1)}
            onPointerDown={(e) => e.stopPropagation()}
            onDoubleClick={(e) => e.stopPropagation()}
            title="Previous (←)"
            aria-label="Previous detail"
          >
            <svg
              width="14"
              height="14"
              viewBox="0 0 14 14"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.8"
            >
              <path d="M9 2L4 7l5 5" />
            </svg>
          </button>
          <button
            className="dv-nav dv-next"
            onClick={() => browse(1)}
            onPointerDown={(e) => e.stopPropagation()}
            onDoubleClick={(e) => e.stopPropagation()}
            title="Next (→)"
            aria-label="Next detail"
          >
            <svg
              width="14"
              height="14"
              viewBox="0 0 14 14"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.8"
            >
              <path d="M5 2l5 5-5 5" />
            </svg>
          </button>
        </div>
        <div className="dv-footer">
          <div className="dv-about">
            <div className="dv-desc">{d?.description ?? ""}</div>
            <div className="dv-hint">
              Scroll to zoom · Drag to pan · Double-click to zoom · ← → to browse
            </div>
          </div>
          <button className="btn-cyan" onClick={() => onInsert(current)} disabled={!d}>
            Insert Detail
          </button>
        </div>
      </div>
    </div>
  );
}
