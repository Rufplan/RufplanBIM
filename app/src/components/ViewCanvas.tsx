import { siteImagery, type Imagery } from "../imagery";
import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import {
  errorMessage,
  ipc,
  type DisplayList,
  type ElementId,
  type Handles,
  type OpeningPreview,
  type Pt,
  type RefLine,
  type SnapResult,
  type ViewInfo,
} from "../ipc";
import { apply } from "../fileActions";
import { drawOptions, editBoundary, filletRadius } from "../sketch";
import { SELECTION_TOOLS, useAppStore } from "../store";
import {
  THEME,
  draw,
  drawGrips,
  drawOverlay,
  drawPreview,
  drawRefLine,
  drawSketch,
  drawTempDims,
  drawTempFrame,
  drawImageUnder,
  drawZoomBox,
  fit,
  tempDimBox,
  toModel,
  toScreen,
  zoomAt,
  type Camera,
} from "../canvas/render";
import {
  closesSketch,
  pointAtLength,
  promptFor,
  samePt,
  startsTypedValue,
  sweep,
  toolAllowed,
} from "../tools";

// Cameras survive tab switches.
const cameras = new Map<string, Camera>();

/** Runs async work one at a time, keeping only the most recent pending request. */
function useLatest<A extends unknown[]>(fn: (...args: A) => Promise<void>) {
  const busy = useRef(false);
  const pending = useRef<A | null>(null);
  const fnRef = useRef(fn);
  useLayoutEffect(() => {
    fnRef.current = fn;
  });
  return useCallback((...args: A) => {
    if (busy.current) {
      pending.current = args;
      return;
    }
    busy.current = true;
    const run = async (a: A) => {
      try {
        await fnRef.current(...a);
      } finally {
        const next = pending.current;
        pending.current = null;
        if (next) void run(next);
        else busy.current = false;
      }
    };
    void run(args);
  }, []);
}

/** Tools whose next point can be typed as a distance (or, for Rotate, an angle). */
const TYPED_TOOLS = [
  "wall",
  "grid",
  "move",
  "copy",
  "array",
  "rotate",
  "stair",
  "beam",
  "railing",
  "roomSeparator",
];

interface Editor {
  /** Typing a length (or angle) for the next point, or a temporary dimension's value. */
  kind: "typed" | "dim";
  /** The tool it was opened for; switching tools hides it. */
  tool: string;
  text: string;
  x: number;
  y: number;
  /** For temporary dimensions: what the value sets. */
  id?: ElementId;
  key?: string;
}

export function ViewCanvas({ view }: { view: ViewInfo }) {
  const revision = useAppStore((s) => s.app?.revision ?? 0);
  const tool = useAppStore((s) => s.tool);
  const selection = useAppStore((s) => s.selection);
  const wrapRef = useRef<HTMLDivElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [dl, setDl] = useState<DisplayList | null>(null);
  const [size, setSize] = useState({ w: 0, h: 0 });
  const [editor, setEditor] = useState<Editor | null>(null);
  const cam = useRef<Camera | null>(cameras.get(view.id) ?? null);
  const pts = useRef<Pt[]>([]);
  const snapRef = useRef<SnapResult | null>(null);
  // Placement preview from Rust (door, window, room, offset), drawn at the cursor.
  const preview = useRef<{
    items: OpeningPreview["items"];
    valid: boolean;
    label: string;
    at: Pt;
  } | null>(null);
  const hover = useRef<string | null>(null);
  const drag = useRef<{ x: number; y: number; moved: boolean; button: number } | null>(null);
  // Grips and temporary dimensions of the selection, and a grip being dragged.
  const handles = useRef<Handles | null>(null);
  const hoverGrip = useRef<number | null>(null);
  const gripDrag = useRef<{ index: number; to: Pt | null } | null>(null);
  // Align's reference line and Trim's first wall.
  const refLine = useRef<RefLine | null>(null);
  const firstPick = useRef<{ id: ElementId; at: Pt } | null>(null);
  // Sketch mode: the draw tool's preview, the first pick of Trim/Fillet, a dragged vertex.
  const sketchPreview = useRef<Pt[][]>([]);
  const sketchFirst = useRef<{ i: number; at: Pt } | null>(null);
  const vertexDrag = useRef<{ from: Pt; to: Pt | null } | null>(null);
  const sketchMode = useAppStore((s) => s.sketchUi.mode);
  // Temporary Hide/Isolate for this view, and each drawn element's category for it.
  const temp = useAppStore((s) => s.tempHide[view.id] ?? null);
  const thinLines = useAppStore((s) => s.thinLines);
  const [viewCats, setViewCats] = useState<Map<string, string>>(new Map());
  // Pan/zoom history (ZP) and a Zoom Region (ZR) drag in progress.
  const camHistory = useRef<Camera[]>([]);
  const lastPush = useRef(0);
  const zoomRegion = useRef<{
    armed: boolean;
    from: [number, number] | null;
    to: [number, number] | null;
  }>({
    armed: false,
    from: null,
    to: null,
  });
  // Match Type's source type.
  const matchSource = useRef<string | null>(null);
  // The satellite overlay on site plans (ADR-026).
  const satellite = useAppStore((s) => s.satellite && view.site && !!s.app?.site);
  const imagery = useRef<Imagery | null>(null);
  const frame = useRef(0);
  const redrawRef = useRef<() => void>(() => {});

  const redraw = useCallback(() => {
    cancelAnimationFrame(frame.current);
    frame.current = requestAnimationFrame(() => {
      const canvas = canvasRef.current;
      const ctx = canvas?.getContext("2d");
      if (!canvas || !ctx || !dl || !cam.current) return;
      const dpr = window.devicePixelRatio || 1;
      const { w, h } = size;
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      const s = useAppStore.getState();
      const sk = s.app?.sketch ?? null;
      const th = s.tempHide[view.id];
      const visible = th
        ? (el: string | null) => {
            if (el === null) return true;
            const hit = th.ids.includes(el) || th.categories.includes(viewCats.get(el) ?? "");
            return th.isolate ? hit : !hit;
          }
        : undefined;
      draw(ctx, dl, cam.current, w, h, {
        selected: new Set(s.selection),
        hover: hover.current,
        faded: sk !== null,
        hidden: sk?.target ?? null,
        visible,
        thin: s.thinLines,
        underlay: imagery.current
          ? (c, S) =>
              drawImageUnder(
                c,
                S,
                imagery.current!.image,
                imagery.current!.frame.corners,
                0.7,
                "Imagery ©Google",
              )
          : undefined,
      });
      if (th) drawTempFrame(ctx, w, h);
      const zr = zoomRegion.current;
      if (zr.from && zr.to) drawZoomBox(ctx, zr.from, zr.to);
      if (sk && sk.view === view.id) {
        const sel = new Set(s.sketchUi.sel);
        const grips: Pt[] = [];
        sk.curves.forEach((c, i) => {
          if (sel.has(i) && c.isLine) grips.push(c.pts[0]!, c.pts[c.pts.length - 1]!);
        });
        const vd = vertexDrag.current;
        const shown = vd?.to
          ? sk.curves.map((c) => ({
              ...c,
              pts: c.pts.map((q) =>
                Math.hypot(q.x - vd.from.x, q.y - vd.from.y) < 1 ? vd.to! : q,
              ),
            }))
          : sk.curves;
        drawSketch(
          ctx,
          cam.current,
          w,
          h,
          shown,
          sel,
          new Set(sk.bad),
          sketchPreview.current,
          grips,
        );
      }
      const hd = handles.current;
      if (s.tool === "select" && hd) {
        drawTempDims(ctx, cam.current, w, h, gripDrag.current ? [] : hd.dims);
        drawGrips(ctx, cam.current, w, h, hd.grips, hoverGrip.current);
        const g = gripDrag.current;
        const grip = g ? hd.grips[g.index] : undefined;
        if (g?.to && grip) {
          drawOverlay(
            ctx,
            cam.current,
            w,
            h,
            grip.anchor ? [grip.anchor] : [],
            g.to,
            snapRef.current?.kind ?? null,
            snapRef.current?.label ?? null,
            false,
          );
        }
      }
      if (refLine.current) {
        drawRefLine(ctx, cam.current, w, h, refLine.current.a, refLine.current.b);
      }
      // Placing with a Rust-computed preview: openings, rooms, offsets, a dimension's line.
      const placing =
        s.tool === "door" ||
        s.tool === "window" ||
        s.tool === "room" ||
        s.tool === "offset" ||
        (s.tool === "dimension" && pts.current.length === 2);
      if (placing && preview.current) {
        const { items, valid, label, at } = preview.current;
        drawPreview(ctx, cam.current, w, h, items, valid ? THEME.cyan : "#c0352b", label, at);
      }
      const drawing = s.tool !== "select" && !placing && toolAllowed(s.tool, view.viewType);
      if (drawing) {
        const sn = snapRef.current;
        // A rubber band from the placed points (Rotate: center and reference ray).
        const a = pts.current[0];
        const c = sn?.pt;
        // A callout previews as its rectangle.
        const rect =
          s.tool === "callout" && a && c ? [a, { x: c.x, y: a.y }, c, { x: a.x, y: c.y }] : null;
        drawOverlay(
          ctx,
          cam.current,
          w,
          h,
          s.tool === "sketch" ? [] : (rect ?? pts.current),
          rect ? null : (sn?.pt ?? null),
          sn?.kind ?? null,
          sn?.label ?? null,
          rect !== null || s.tool === "floor" || s.tool === "ceiling",
        );
      }
    });
  }, [dl, size, view.viewType, view.id, viewCats]);

  // Categories of drawn elements, when hiding or isolating by category.
  useEffect(() => {
    let live = true;
    if (!temp || temp.categories.length === 0) return;
    ipc.viewCategories(view.id).then(
      (list) => live && setViewCats(new Map(list)),
      () => {},
    );
    return () => {
      live = false;
    };
  }, [temp, view.id, revision]);
  useEffect(() => redrawRef.current(), [temp, thinLines]);

  // Fetch the satellite image when the overlay is on (again when the topography changes).
  useEffect(() => {
    let live = true;
    if (!satellite) {
      imagery.current = null;
      redrawRef.current();
      return;
    }
    siteImagery().then(
      (im) => {
        if (!live) return;
        imagery.current = im;
        redrawRef.current();
      },
      (e) => {
        if (!live) return;
        useAppStore.getState().setError(errorMessage(e));
        useAppStore.getState().setSatellite(false);
      },
    );
    return () => {
      live = false;
    };
  }, [satellite, revision]);
  useLayoutEffect(() => {
    redrawRef.current = redraw;
  });

  // Fetch the display list whenever the model or view changes.
  useEffect(() => {
    let live = true;
    ipc.displayList(view.id).then(
      (d) => live && setDl(d),
      (e) => useAppStore.getState().setError(errorMessage(e)),
    );
    return () => {
      live = false;
    };
  }, [view.id, revision]);

  // Grips and temporary dimensions follow the selection (and the model).
  useEffect(() => {
    let live = true;
    if (tool !== "select" || selection.length === 0) {
      handles.current = null;
      redrawRef.current();
      return;
    }
    ipc.handles(view.id, selection).then(
      (h) => {
        if (!live) return;
        handles.current = h;
        redrawRef.current();
      },
      () => {},
    );
    return () => {
      live = false;
    };
  }, [view.id, selection, revision, tool]);

  // Track canvas size.
  useEffect(() => {
    const el = wrapRef.current;
    if (!el) return;
    const ro = new ResizeObserver(() => {
      const r = el.getBoundingClientRect();
      setSize({ w: Math.floor(r.width), h: Math.floor(r.height) });
    });
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || size.w === 0) return;
    const dpr = window.devicePixelRatio || 1;
    canvas.width = size.w * dpr;
    canvas.height = size.h * dpr;
    if (dl && !cam.current) {
      cam.current = fit(dl.bounds, size.w, size.h);
      cameras.set(view.id, cam.current);
    }
    redraw();
  }, [size, dl, redraw, view.id]);

  useEffect(redraw, [selection, redraw]);

  const resetRefs = useCallback(() => {
    matchSource.current = null;
    pts.current = [];
    snapRef.current = null;
    preview.current = null;
    refLine.current = null;
    firstPick.current = null;
  }, []);
  const resetTool = useCallback(() => {
    resetRefs();
    setEditor(null);
  }, [resetRefs]);

  // Tool changes reset the sketch and prompt. (Not on redraw changes: new display lists
  // arrive after every edit, and a wall chain must survive them.)
  // (An open typed-value box belongs to the tool that opened it; see `editor.tool`.)
  useEffect(() => {
    resetRefs();
    sketchPreview.current = [];
    sketchFirst.current = null;
    useAppStore.getState().setPrompt(promptFor(tool, 0, view.viewType));
    redrawRef.current();
  }, [tool, view.viewType, resetRefs, sketchMode]);

  // A new sketch state from Rust redraws the sketch.
  const sketchState = useAppStore((s) => s.app?.sketch);
  useEffect(() => redrawRef.current(), [sketchState]);

  /** Remembers the view before a zoom, for Previous Pan/Zoom (ZP). */
  const pushCam = (force = false) => {
    const now = Date.now();
    if (cam.current && (force || now - lastPush.current > 800)) {
      camHistory.current.push({ ...cam.current });
      if (camHistory.current.length > 30) camHistory.current.shift();
    }
    lastPush.current = now;
  };
  const setCam = (c: Camera) => {
    cam.current = c;
    cameras.set(view.id, c);
    redraw();
  };

  const local = (e: { clientX: number; clientY: number }): [number, number] => {
    const r = canvasRef.current?.getBoundingClientRect();
    return r ? [e.clientX - r.left, e.clientY - r.top] : [0, 0];
  };

  const modelAt = (sx: number, sy: number) =>
    cam.current ? toModel(cam.current, size.w, size.h, sx, sy) : { x: 0, y: 0 };

  /** Index of the grip under a screen point, if any. */
  const gripAt = (sx: number, sy: number): number | null => {
    const g = handles.current?.grips ?? [];
    if (!cam.current || useAppStore.getState().tool !== "select") return null;
    for (let i = 0; i < g.length; i++) {
      const [gx, gy] = toScreen(cam.current, size.w, size.h, g[i]!.at.x, g[i]!.at.y);
      if (Math.abs(gx - sx) <= 7 && Math.abs(gy - sy) <= 7) return i;
    }
    return null;
  };

  /** The temporary dimension whose value box is under a screen point, if any. */
  const tempDimAt = (sx: number, sy: number) => {
    const ctx = canvasRef.current?.getContext("2d");
    if (!ctx || !cam.current) return null;
    for (const d of handles.current?.dims ?? []) {
      const [bx, by, bw, bh] = tempDimBox(ctx, cam.current, size.w, size.h, d.labelAt, d.value);
      if (sx >= bx && sx <= bx + bw && sy >= by && sy <= by + bh) return { d, bx, by };
    }
    return null;
  };

  const hoverPick = useLatest(async (p: Pt, tol: number) => {
    const id = await ipc.pick(view.id, p, tol);
    if (id !== hover.current) {
      hover.current = id;
      redraw();
    }
  });

  const snapAt = useLatest(async (p: Pt, tol: number) => {
    const g = gripDrag.current;
    const grip = g ? handles.current?.grips[g.index] : undefined;
    const from = grip ? (grip.anchor ?? null) : (pts.current[pts.current.length - 1] ?? null);
    snapRef.current = await ipc.snap(view.id, p, from, tol);
    if (g) g.to = snapRef.current.pt;
    redraw();
  });

  /** What the sketch tool would add for the cursor (computed in Rust). */
  const sketchPreviewAt = useLatest(async (cursor: Pt, tol: number) => {
    const s = useAppStore.getState();
    const ui = s.sketchUi;
    const m = ui.mode;
    const draws = !["Modify", "Trim", "FilletArc"].includes(m);
    if (!draws || (!m.startsWith("Pick") && pts.current.length === 0)) {
      if (sketchPreview.current.length) {
        sketchPreview.current = [];
        redraw();
      }
      return;
    }
    const options = await drawOptions();
    sketchPreview.current = await ipc.sketchPreview(
      m,
      pts.current,
      cursor,
      options,
      tol,
      ui.tab,
      ui.core,
    );
    redraw();
  });

  /** A vertex grip (end of a selected sketch line) under a screen point. */
  const sketchGripAt = (sx: number, sy: number): Pt | null => {
    const s = useAppStore.getState();
    const sk = s.app?.sketch;
    if (!sk || !cam.current || s.sketchUi.mode !== "Modify") return null;
    for (const i of s.sketchUi.sel) {
      const c = sk.curves[i];
      if (!c?.isLine) continue;
      for (const q of [c.pts[0]!, c.pts[c.pts.length - 1]!]) {
        const [gx, gy] = toScreen(cam.current, size.w, size.h, q.x, q.y);
        if (Math.abs(gx - sx) <= 6 && Math.abs(gy - sy) <= 6) return q;
      }
    }
    return null;
  };

  /** A click in sketch mode, by the active boundary line tool. */
  async function sketchClick(raw: Pt, shift: boolean) {
    const s = useAppStore.getState();
    const ui = s.sketchUi;
    const zoom = cam.current?.zoom ?? 1;
    const pickTol = 8 / zoom;
    const m = ui.mode;
    const offset = async () => (await drawOptions()).offset;
    if (m === "PickWalls") {
      await apply(async () => ipc.sketchPickWalls(raw, pickTol, ui.tab, ui.core, await offset()));
      s.setSketchUi({ tab: false });
    } else if (m === "PickLines") {
      await apply(async () => ipc.sketchPickLine(raw, pickTol, await offset(), ui.lock));
    } else if (m === "Modify") {
      const i = await ipc.sketchHit(raw, pickTol);
      if (i === null) s.setSketchUi({ sel: shift ? ui.sel : [] });
      else if (shift)
        s.setSketchUi({ sel: ui.sel.includes(i) ? ui.sel.filter((x) => x !== i) : [...ui.sel, i] });
      else s.setSketchUi({ sel: [i] });
    } else if (m === "Trim" || m === "FilletArc") {
      const i = await ipc.sketchHit(raw, pickTol);
      if (i === null) {
        s.setError("Click a boundary line.");
      } else if (!sketchFirst.current) {
        sketchFirst.current = { i, at: raw };
      } else {
        const a = sketchFirst.current;
        sketchFirst.current = null;
        if (m === "Trim") await apply(() => ipc.sketchTrim(a.i, a.at, i, raw));
        else {
          const r = await filletRadius();
          await apply(() => ipc.sketchFillet(a.i, i, r));
        }
      }
    } else {
      const from = pts.current[pts.current.length - 1] ?? null;
      const p = (await ipc.snap(view.id, raw, from, 12 / zoom)).pt;
      await sketchPoint(p);
      return;
    }
    s.setPrompt(promptFor("sketch", sketchFirst.current ? 1 : 0, view.viewType));
    redraw();
  }

  /** The next point of a draw tool (clicked, snapped or typed). */
  async function sketchPoint(p: Pt) {
    const s = useAppStore.getState();
    const m = s.sketchUi.mode;
    const need = m === "StartEndRadiusArc" || m === "CenterEndsArc" ? 3 : 2;
    const from = pts.current[pts.current.length - 1];
    if (from && samePt(from, p)) return;
    const all = [...pts.current, p];
    if (all.length < need) {
      pts.current = all;
    } else {
      const options = await drawOptions();
      const ok = await apply(() => ipc.sketchDraw(m as never, all, options));
      // Chained lines continue from the end of the last one.
      pts.current = ok && m === "Line" && s.sketchUi.chain ? [p] : ok ? [] : pts.current;
      sketchPreview.current = [];
    }
    s.setPrompt(promptFor("sketch", pts.current.length, view.viewType));
    redraw();
  }

  const dimensionAt = useLatest(async (p: Pt) => {
    const [a, b] = pts.current;
    if (!a || !b) return;
    const d = await ipc.dimensionPreview(view.id, a, b, p);
    preview.current = d ? { items: d.items, valid: true, label: "", at: p } : null;
    redraw();
  });

  const previewAt = useLatest(async (p: Pt, tol: number) => {
    const s = useAppStore.getState();
    if (s.tool === "room") {
      const r = await ipc.roomPreview(view.id, p);
      preview.current = r ? { ...r, at: p } : null;
    } else if (s.tool === "offset") {
      const o = await ipc.offsetPreview(view.id, p, tol, s.options.offsetDistance);
      preview.current = o
        ? { items: o.items, valid: true, label: s.options.offsetDistance, at: p }
        : null;
    } else {
      const typeId = s.tool === "door" ? s.toolTypes.door : s.toolTypes.window;
      const o = typeId ? await ipc.openingPreview(view.id, typeId, p, tol) : null;
      preview.current = o
        ? {
            items: o.items,
            valid: o.valid,
            label: o.valid ? o.label : "Overlaps another opening",
            at: p,
          }
        : null;
    }
    redraw();
  });

  const finishSketch = useCallback(async () => {
    const s = useAppStore.getState();
    const ring = pts.current;
    pts.current = [];
    snapRef.current = null;
    if (s.tool === "railing" && ring.length >= 2) {
      await apply(() => ipc.createRailing(view.id, s.toolTypes.railing, ring));
    }
    if (ring.length >= 3) {
      if (s.tool === "floor" && s.toolTypes.floor)
        await apply(() => ipc.createFloor(view.id, s.toolTypes.floor!, ring));
      if (s.tool === "ceiling" && s.toolTypes.ceiling)
        await apply(() => ipc.createCeiling(view.id, s.toolTypes.ceiling!, ring, null));
    }
    s.setPrompt(promptFor(s.tool, 0, view.viewType));
    redraw();
  }, [view.id, view.viewType, redraw]);

  const cancelSketch = useCallback(() => {
    const s = useAppStore.getState();
    if (pts.current.length === 0 && !refLine.current && !firstPick.current) {
      if (s.tool !== "select") s.setTool("select");
      else s.select([]);
    }
    resetTool();
    s.setPrompt(promptFor(useAppStore.getState().tool, 0, view.viewType));
    redraw();
  }, [view.viewType, redraw, resetTool]);

  // Keyboard events from App (Esc / Enter / typed values) reach the active canvas
  // through window events.
  useEffect(() => {
    const onCancel = () => {
      const s = useAppStore.getState();
      if (s.tool === "sketch") {
        // Esc ends the current line chain, then returns to Modify; it never leaves sketch mode.
        if (pts.current.length || sketchFirst.current) {
          pts.current = [];
          sketchFirst.current = null;
          sketchPreview.current = [];
        } else if (s.sketchUi.mode !== "Modify") {
          s.setSketchUi({ mode: "Modify" });
        } else {
          s.setSketchUi({ sel: [] });
        }
        s.setPrompt(promptFor("sketch", 0, view.viewType));
        redraw();
        return;
      }
      if ((s.tool === "floor" || s.tool === "ceiling") && pts.current.length >= 3) {
        pts.current = [];
        redraw();
        return;
      }
      cancelSketch();
    };
    const onFinish = () => void finishSketch();
    const onFit = () => {
      if (!dl || !size.w) return;
      pushCam(true);
      setCam(fit(dl.bounds, size.w, size.h));
    };
    const onZoomOut = () => {
      if (!cam.current) return;
      pushCam(true);
      setCam({ ...cam.current, zoom: cam.current.zoom / 2 });
    };
    const onZoomPrev = () => {
      const prev = camHistory.current.pop();
      if (prev) setCam(prev);
    };
    const onZoomRegion = () => {
      zoomRegion.current = { armed: true, from: null, to: null };
      useAppStore.getState().setPrompt("Drag a rectangle to zoom into.");
    };
    const onTyped = (e: Event) => {
      const key = (e as CustomEvent<string>).detail;
      const s = useAppStore.getState();
      const hasBase = pts.current.length > 0 && (s.tool !== "rotate" || pts.current.length === 2);
      const typedTool =
        TYPED_TOOLS.includes(s.tool) || (s.tool === "sketch" && s.sketchUi.mode === "Line");
      if (!typedTool || !hasBase || !startsTypedValue(key)) return;
      const sn = snapRef.current?.pt;
      const at = sn && cam.current ? toScreen(cam.current, size.w, size.h, sn.x, sn.y) : null;
      setEditor({
        kind: "typed",
        tool: s.tool,
        text: key,
        x: at ? at[0] + 16 : size.w / 2,
        y: at ? at[1] - 34 : size.h / 2,
      });
    };
    window.addEventListener("tool-cancel", onCancel);
    window.addEventListener("tool-finish", onFinish);
    window.addEventListener("view-fit", onFit);
    window.addEventListener("view-zoom-out", onZoomOut);
    window.addEventListener("view-zoom-previous", onZoomPrev);
    window.addEventListener("view-zoom-region", onZoomRegion);
    window.addEventListener("typed-value", onTyped);
    return () => {
      window.removeEventListener("tool-cancel", onCancel);
      window.removeEventListener("tool-finish", onFinish);
      window.removeEventListener("view-fit", onFit);
      window.removeEventListener("view-zoom-out", onZoomOut);
      window.removeEventListener("view-zoom-previous", onZoomPrev);
      window.removeEventListener("view-zoom-region", onZoomRegion);
      window.removeEventListener("typed-value", onTyped);
    };
  });

  /** Places the tool's next point at `p` (already snapped or typed). */
  async function placePoint(p: Pt, raw: Pt) {
    const s = useAppStore.getState();
    const from = pts.current[pts.current.length - 1] ?? null;
    const selection = s.selection;
    const needSelection = () => {
      s.setError(
        `Select what to ${s.tool} first, then choose ${s.tool[0]!.toUpperCase()}${s.tool.slice(1)}.`,
      );
    };
    const done = (ok: boolean, keepTool = false) => {
      pts.current = [];
      snapRef.current = null;
      if (ok && !keepTool) s.setTool("select");
    };
    switch (s.tool) {
      case "move":
      case "copy":
      case "array": {
        if (selection.length === 0) needSelection();
        else if (!from) pts.current = [p];
        else if (!samePt(from, p)) {
          const delta = { x: p.x - from.x, y: p.y - from.y };
          if (s.tool === "move") {
            done(await apply(() => ipc.moveElements(selection, delta)));
          } else if (s.tool === "array") {
            const n = Math.max(2, Math.round(s.options.arrayCount));
            done(await apply(() => ipc.copyElements(selection, delta, n - 1)));
          } else {
            const ok = await apply(() => ipc.copyElements(selection, delta, 1));
            if (s.options.copyMultiple) pts.current = [from];
            else done(ok);
          }
        }
        break;
      }
      case "rotate": {
        if (selection.length === 0) needSelection();
        else if (pts.current.length < 2) {
          if (!from || !samePt(from, p)) pts.current = [...pts.current, p];
        } else {
          const [c, r] = pts.current as [Pt, Pt];
          const angle = sweep(c, r, p);
          done(await apply(() => ipc.rotateElements(selection, c, angle, s.options.rotateCopy)));
        }
        break;
      }
      case "mirror": {
        if (selection.length === 0) needSelection();
        else if (!from) pts.current = [p];
        else if (!samePt(from, p)) {
          done(await apply(() => ipc.mirrorElements(selection, from, p, s.options.mirrorCopy)));
        }
        break;
      }
      case "stair": {
        if (!from) pts.current = [p];
        else if (!samePt(from, p)) {
          pts.current = [];
          const shape = s.options.stairShape;
          await apply(() => ipc.createStair(view.id, from, p, shape === "straight" ? null : shape));
        }
        break;
      }
      case "wall": {
        if (from && !samePt(from, p) && s.toolTypes.wall) {
          const wt = s.toolTypes.wall;
          const loc = s.options.wallLocation;
          const ok = await apply(() =>
            loc === "Centerline"
              ? ipc.createWall(view.id, wt, from, p)
              : ipc.createWallLocated(view.id, wt, from, p, loc),
          );
          pts.current = ok ? [p] : pts.current;
        } else if (!from) {
          pts.current = [p];
        }
        break;
      }
      case "grid": {
        if (from && !samePt(from, p)) {
          await apply(() => ipc.createGrid(from, p));
          pts.current = [];
        } else {
          pts.current = [p];
        }
        break;
      }
      case "roomSeparator": {
        // Chained like walls: each click continues from the last point.
        if (from && !samePt(from, p)) {
          const ok = await apply(() => ipc.createRoomSeparator(view.id, from, p));
          pts.current = ok ? [p] : pts.current;
        } else if (!from) {
          pts.current = [p];
        }
        break;
      }
      case "callout": {
        if (from && Math.abs(from.x - p.x) > 1 && Math.abs(from.y - p.y) > 1) {
          pts.current = [];
          if (await apply(() => ipc.createCallout(view.id, from, p))) s.setTool("select");
        } else if (!from) {
          pts.current = [p];
        }
        break;
      }
      case "beam": {
        if (from && !samePt(from, p)) {
          await apply(() => ipc.createBeam(view.id, s.toolTypes.beam, from, p));
          pts.current = [];
        } else {
          pts.current = [p];
        }
        break;
      }
      case "railing": {
        if (!from || !samePt(from, p)) pts.current = [...pts.current, p];
        break;
      }
      default:
        void raw;
    }
    s.setPrompt(promptFor(useAppStore.getState().tool, pts.current.length, view.viewType));
    redraw();
  }

  /** Commits the typed length (or Rotate angle) as the next point. */
  async function commitTyped(text: string) {
    setEditor(null);
    const s = useAppStore.getState();
    const from = pts.current[pts.current.length - 1];
    const toward = snapRef.current?.pt;
    if (!from || !toward) return;
    if (s.tool === "rotate" && pts.current.length === 2) {
      const deg = Number(text.replace("°", ""));
      if (!Number.isFinite(deg)) {
        s.setError(`"${text}" is not an angle in degrees.`);
        return;
      }
      const [c] = pts.current as [Pt, Pt];
      const angle = (deg * Math.PI) / 180;
      pts.current = [];
      if (await apply(() => ipc.rotateElements(s.selection, c, angle, s.options.rotateCopy)))
        s.setTool("select");
      return;
    }
    const mm = await ipc.parseLength(text);
    if (mm === null) {
      s.setError(`"${text}" is not a length (try 12'-6" or 3.5').`);
      return;
    }
    const p = pointAtLength(from, toward, mm);
    if (p && s.tool === "sketch") await sketchPoint(p);
    else if (p) await placePoint(p, p);
  }

  async function commitTempDim(e: Editor) {
    setEditor(null);
    if (e.id && e.key) await apply(() => ipc.setTempDimension(e.id!, e.key!, e.text));
  }

  async function click(sx: number, sy: number, shift: boolean) {
    const s = useAppStore.getState();
    if (!cam.current) return;
    const raw = modelAt(sx, sy);
    const tol = 12 / cam.current.zoom;
    if (s.tool === "select") {
      const td = tempDimAt(sx, sy);
      if (td) {
        setEditor({
          kind: "dim",
          tool: s.tool,
          text: td.d.value,
          x: td.bx,
          y: td.by,
          id: td.d.id,
          key: td.d.key,
        });
        return;
      }
      const id = await ipc.pick(view.id, raw, 6 / cam.current.zoom);
      if (!id) s.select(shift ? s.selection : []);
      else if (shift)
        s.select(
          s.selection.includes(id) ? s.selection.filter((x) => x !== id) : [...s.selection, id],
        );
      else s.select([id]);
      return;
    }
    if (!toolAllowed(s.tool, view.viewType)) return;
    if (s.tool === "sketch") {
      await sketchClick(raw, shift);
      return;
    }
    if (s.tool === "elevation") {
      const p = (await ipc.snap(view.id, raw, null, tol)).pt;
      const markType = s.elevationType ?? s.app?.elevationMarkerTypes[0]?.id ?? null;
      if (await apply(() => ipc.createElevationMarker(view.id, p, true, markType)))
        s.setTool("select");
      return;
    }
    if (s.tool === "door" || s.tool === "window") {
      const typeId = s.tool === "door" ? s.toolTypes.door : s.toolTypes.window;
      const pv = typeId ? await ipc.openingPreview(view.id, typeId, raw, tol) : null;
      if (typeId && pv?.valid) {
        await apply(() => ipc.createOpening(typeId, pv.host, pv.offset, pv.flipFacing));
      } else if (pv) {
        s.setError("That spot overlaps another door or window in this wall.");
      }
      preview.current = null;
      redraw();
      return;
    }
    if (s.tool === "room") {
      const r = await ipc.roomPreview(view.id, raw);
      if (r?.valid) await apply(() => ipc.createRoom(view.id, raw));
      else if (r) s.setError(r.label);
      preview.current = null;
      redraw();
      return;
    }
    if (s.tool === "offset") {
      await apply(() => ipc.offsetElement(view.id, raw, tol, s.options.offsetDistance));
      preview.current = null;
      redraw();
      return;
    }
    if (s.tool === "roof") {
      await apply(() => ipc.createRoof(view.id, s.toolTypes.roof));
      return;
    }
    if (s.tool === "tag") {
      const id = await ipc.pick(view.id, raw, 6 / cam.current.zoom);
      if (!id) s.setError("Click a door, window, room, column or beam.");
      else await apply(() => ipc.tagElement(view.id, id));
      return;
    }
    if (s.tool === "matchType") {
      const id = await ipc.pick(view.id, raw, 6 / cam.current.zoom);
      if (!id) return;
      const sheet = await ipc.properties(id);
      if (!matchSource.current) {
        if (!sheet.typeId) {
          s.setError("That element has no type to match.");
          return;
        }
        matchSource.current = sheet.typeId;
        s.setPrompt(promptFor("matchType", 1, view.viewType));
      } else {
        const src = matchSource.current;
        await apply(() => ipc.setProperty(id, "type", src));
      }
      return;
    }
    if (s.tool === "mirrorPick") {
      if (s.selection.length === 0) {
        s.setError("Select what to mirror first, then Mirror - Pick Axis (MM).");
        return;
      }
      const line = await ipc.refLine(view.id, raw, tol, null);
      if (!line) {
        s.setError("Pick a wall face, a wall centerline or a grid as the axis.");
        return;
      }
      if (await apply(() => ipc.mirrorElements(s.selection, line.a, line.b, s.options.mirrorCopy)))
        s.setTool("select");
      return;
    }
    if (s.tool === "align") {
      if (!refLine.current) {
        refLine.current = await ipc.refLine(view.id, raw, tol, null);
        if (!refLine.current) s.setError("Click on a wall face, a wall centerline or a grid.");
      } else {
        const reference = refLine.current;
        const refPt = {
          x: (reference.a.x + reference.b.x) / 2,
          y: (reference.a.y + reference.b.y) / 2,
        };
        refLine.current = null;
        await apply(() => ipc.align(view.id, refPt, raw, tol));
      }
      s.setPrompt(promptFor(s.tool, refLine.current ? 1 : 0, view.viewType));
      redraw();
      return;
    }
    if (s.tool === "trim" || s.tool === "split") {
      const id = await ipc.pick(view.id, raw, 6 / cam.current.zoom);
      if (!id) {
        s.setError("Click on a wall.");
        return;
      }
      if (s.tool === "split") {
        const p = (await ipc.snap(view.id, raw, null, tol)).pt;
        await apply(() => ipc.splitWall(id, p));
      } else if (!firstPick.current) {
        firstPick.current = { id, at: raw };
      } else {
        const a = firstPick.current;
        firstPick.current = null;
        await apply(() => ipc.trimExtend(a.id, a.at, id, raw));
      }
      s.setPrompt(promptFor(s.tool, firstPick.current ? 1 : 0, view.viewType));
      redraw();
      return;
    }
    const from = pts.current[pts.current.length - 1] ?? null;
    const p = (await ipc.snap(view.id, raw, from, tol)).pt;
    if (s.tool === "dimension") {
      const [a, b] = pts.current;
      if (a && b) {
        const pv = await ipc.dimensionPreview(view.id, a, b, raw);
        pts.current = [];
        preview.current = null;
        if (pv) await apply(() => ipc.createDimension(view.id, a, b, pv.offset));
      } else if (!from || !samePt(from, p)) {
        pts.current = [...pts.current, p];
      }
      s.setPrompt(promptFor(s.tool, pts.current.length, view.viewType));
      redraw();
      return;
    }
    if (s.tool === "text") {
      const text = window.prompt("Text note", "");
      if (text !== null) await apply(() => ipc.createText(view.id, raw, text));
      return;
    }
    if (s.tool === "section") {
      if (!from) {
        pts.current = [p];
      } else if (!samePt(from, p)) {
        pts.current = [];
        await apply(() => ipc.createSection(from, p));
      }
      s.setPrompt(promptFor(s.tool, pts.current.length, view.viewType));
      redraw();
      return;
    }
    if (s.tool === "column") {
      await apply(() => ipc.createColumn(view.id, s.toolTypes.column, p));
      return;
    }
    if (
      SELECTION_TOOLS.includes(s.tool) ||
      ["wall", "grid", "stair", "beam", "railing", "roomSeparator", "callout"].includes(s.tool)
    ) {
      await placePoint(p, raw);
      return;
    }
    switch (s.tool) {
      case "floor":
      case "ceiling": {
        const first = pts.current[0];
        if (
          first &&
          closesSketch(
            toScreen(cam.current, size.w, size.h, first.x, first.y),
            [sx, sy],
            pts.current.length,
          )
        ) {
          await finishSketch();
          return;
        }
        if (!from || !samePt(from, p)) pts.current = [...pts.current, p];
        break;
      }
      case "floorAuto":
        if (s.toolTypes.floor) await apply(() => ipc.createFloor(view.id, s.toolTypes.floor!, []));
        break;
      case "ceilingAuto":
        if (s.toolTypes.ceiling)
          await apply(() => ipc.createCeiling(view.id, s.toolTypes.ceiling!, [], raw));
        break;
      case "level":
        await apply(() => ipc.createLevel(p.y));
        break;
    }
    s.setPrompt(promptFor(s.tool, pts.current.length, view.viewType));
    redraw();
  }

  async function doubleClick(sx: number, sy: number) {
    const s = useAppStore.getState();
    if (s.tool !== "select" || !cam.current || !s.app) return;
    const id = await ipc.pick(view.id, modelAt(sx, sy), 6 / cam.current.zoom);
    if (!id) return;
    const asView = s.app.views.find((v) => v.id === id);
    const levelPlan = s.app.views.find((v) => v.level === id && v.viewType === "Plan");
    const target = asView ?? levelPlan;
    if (target && target.id !== view.id) s.openView(target.id);
    else if (!target) {
      // Double-clicking a floor or ceiling edits its boundary, as in Revit.
      const sheet = await ipc.properties(id).catch(() => null);
      if (sheet && (sheet.category === "Floor" || sheet.category === "Ceiling"))
        await editBoundary(id);
    }
  }

  return (
    <div
      ref={wrapRef}
      className="canvas-wrap"
      data-tool={tool}
      onContextMenu={(e) => e.preventDefault()}
    >
      <canvas
        ref={canvasRef}
        style={{ width: size.w, height: size.h }}
        onWheel={(e) => {
          if (!cam.current) return;
          const [sx, sy] = local(e);
          pushCam();
          setCam(zoomAt(cam.current, size.w, size.h, sx, sy, Math.exp(-e.deltaY * 0.0015)));
        }}
        onMouseDown={(e) => {
          const [x, y] = local(e);
          if (zoomRegion.current.armed && e.button === 0) {
            zoomRegion.current = { armed: true, from: [x, y], to: [x, y] };
            return;
          }
          drag.current = { x, y, moved: false, button: e.button };
          if (e.button === 0) {
            const g = gripAt(x, y);
            if (g !== null) gripDrag.current = { index: g, to: null };
            const v = sketchGripAt(x, y);
            if (v) vertexDrag.current = { from: v, to: null };
          }
        }}
        onMouseMove={(e) => {
          const [sx, sy] = local(e);
          const zr = zoomRegion.current;
          if (zr.armed && zr.from) {
            zr.to = [sx, sy];
            redraw();
            return;
          }
          const d = drag.current;
          if (d && (d.button === 1 || d.button === 2) && cam.current) {
            const dx = sx - d.x;
            const dy = sy - d.y;
            if (Math.abs(dx) + Math.abs(dy) > 2) d.moved = true;
            if (d.moved) {
              setCam({
                ...cam.current,
                cx: cam.current.cx - dx / cam.current.zoom,
                cy: cam.current.cy + dy / cam.current.zoom,
              });
              drag.current = { ...d, x: sx, y: sy };
            }
            return;
          }
          if (!cam.current) return;
          const p = modelAt(sx, sy);
          const ft = (mm: number) => (mm / 304.8).toFixed(2);
          const inch = (mm: number) => (mm / 25.4).toFixed(2);
          const vertical = view.viewType === "Elevation" || view.viewType === "Section" ? "Z" : "Y";
          useAppStore
            .getState()
            .setCursor(
              view.viewType === "Sheet"
                ? `PAPER X ${inch(p.x)}"   Y ${inch(p.y)}"`
                : `X ${ft(p.x)}'   ${vertical} ${ft(p.y)}'`,
            );
          const s = useAppStore.getState();
          if (vertexDrag.current) {
            const vd = vertexDrag.current;
            if (d && Math.abs(sx - d.x) + Math.abs(sy - d.y) > 2) d.moved = true;
            void ipc.snap(view.id, p, null, 12 / cam.current.zoom).then((r) => {
              if (vertexDrag.current === vd) {
                vd.to = r.pt;
                redraw();
              }
            });
            return;
          }
          if (s.tool === "sketch") {
            const m = s.sketchUi.mode;
            if (m === "PickWalls" || m === "PickLines") {
              sketchPreviewAt(p, 8 / cam.current.zoom);
            } else if (m !== "Modify" && m !== "Trim" && m !== "FilletArc") {
              snapAt(p, 12 / cam.current.zoom);
              sketchPreviewAt(snapRef.current?.pt ?? p, 8 / cam.current.zoom);
            }
            return;
          }
          if (gripDrag.current) {
            if (d && Math.abs(sx - d.x) + Math.abs(sy - d.y) > 2) d.moved = true;
            snapAt(p, 12 / cam.current.zoom);
            return;
          }
          if (s.tool === "select") {
            const g = gripAt(sx, sy);
            if (g !== hoverGrip.current) {
              hoverGrip.current = g;
              redraw();
            }
            hoverPick(p, 6 / cam.current.zoom);
          } else if (
            s.tool === "door" ||
            s.tool === "window" ||
            s.tool === "room" ||
            s.tool === "offset"
          )
            previewAt(p, 12 / cam.current.zoom);
          else if (s.tool === "dimension" && pts.current.length === 2) dimensionAt(p);
          else if (toolAllowed(s.tool, view.viewType)) snapAt(p, 12 / cam.current.zoom);
        }}
        onMouseUp={(e) => {
          const zr = zoomRegion.current;
          if (zr.armed && zr.from && cam.current) {
            const [sx, sy] = local(e);
            const a = modelAt(zr.from[0], zr.from[1]);
            const b = modelAt(sx, sy);
            zoomRegion.current = { armed: false, from: null, to: null };
            if (Math.abs(a.x - b.x) > 1 && Math.abs(a.y - b.y) > 1) {
              pushCam(true);
              setCam(
                fit(
                  [Math.min(a.x, b.x), Math.min(a.y, b.y), Math.max(a.x, b.x), Math.max(a.y, b.y)],
                  size.w,
                  size.h,
                ),
              );
            } else redraw();
            useAppStore
              .getState()
              .setPrompt(promptFor(useAppStore.getState().tool, 0, view.viewType));
            return;
          }
          const d = drag.current;
          drag.current = null;
          const vd = vertexDrag.current;
          if (vd) {
            vertexDrag.current = null;
            if (vd.to && d?.moved) void apply(() => ipc.sketchMoveVertex(vd.from, vd.to!));
            else redraw();
            if (d?.moved) return;
          }
          const g = gripDrag.current;
          if (g) {
            gripDrag.current = null;
            const grip = handles.current?.grips[g.index];
            snapRef.current = null;
            if (grip && g.to && d?.moved) {
              void apply(() => ipc.dragHandle(grip.id, grip.key, g.to!));
            }
            redraw();
            return;
          }
          if (!d || d.moved) return;
          const [sx, sy] = local(e);
          // A snap override (SE, SM…) lasts for one pick.
          if (e.button === 0)
            void click(sx, sy, e.shiftKey).finally(() =>
              useAppStore.getState().setSnapOverride(null),
            );
          if (e.button === 2 && useAppStore.getState().tool === "sketch") {
            window.dispatchEvent(new Event("tool-cancel"));
            return;
          }
          if (e.button === 2) {
            const s = useAppStore.getState();
            if ((s.tool === "floor" || s.tool === "ceiling") && pts.current.length >= 3)
              void finishSketch();
            else cancelSketch();
          }
        }}
        onDoubleClick={(e) => {
          const [sx, sy] = local(e);
          void doubleClick(sx, sy);
        }}
        onMouseLeave={() => {
          hover.current = null;
          hoverGrip.current = null;
          if (!gripDrag.current) snapRef.current = null;
          preview.current = null;
          redraw();
        }}
      />
      {editor && editor.tool === tool && (
        <input
          className={`canvas-input ${editor.kind}`}
          style={{ left: editor.x, top: editor.y }}
          aria-label={editor.kind === "dim" ? "Dimension value" : "Typed length"}
          autoFocus
          value={editor.text}
          onFocus={(e) => editor.kind === "dim" && e.target.select()}
          onChange={(e) => setEditor({ ...editor, text: e.target.value })}
          onKeyDown={(e) => {
            e.stopPropagation();
            if (e.key === "Enter") {
              if (editor.kind === "dim") void commitTempDim(editor);
              else void commitTyped(editor.text);
            } else if (e.key === "Escape") {
              setEditor(null);
            }
          }}
          onBlur={() => setEditor(null)}
        />
      )}
      {!dl && <div className="canvas-loading">Generating view…</div>}
    </div>
  );
}
