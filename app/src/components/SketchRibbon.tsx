import type { ReactNode } from "react";
import { apply } from "../fileActions";
import { ipc } from "../ipc";
import {
  cancelSketchMode,
  deleteSketchSelection,
  finishSketchMode,
  flipSketchSelection,
} from "../sketch";
import { useAppStore, type SketchMode } from "../store";
import { Icons } from "./Icons";

// Revit's contextual tab in sketch mode (ADR-021): Mode (Finish / Cancel), Draw (the
// boundary line tools) and Modify.

const I = ({ children }: { children: ReactNode }) => (
  <svg
    viewBox="0 0 24 24"
    width="22"
    height="22"
    fill="none"
    stroke="currentColor"
    strokeWidth="1.6"
    aria-hidden
  >
    {children}
  </svg>
);

const DRAW: [SketchMode, string, ReactNode][] = [
  [
    "Line",
    "Line",
    <I key="l">
      <path d="M4 19L20 5" />
      <circle cx="4" cy="19" r="1.5" fill="currentColor" />
      <circle cx="20" cy="5" r="1.5" fill="currentColor" />
    </I>,
  ],
  [
    "Rectangle",
    "Rectangle",
    <I key="r">
      <rect x="4" y="6" width="16" height="12" />
    </I>,
  ],
  [
    "InscribedPolygon",
    "Inscribed Polygon",
    <I key="ip">
      <circle cx="12" cy="12" r="8" strokeWidth="0.9" strokeDasharray="2 1.5" />
      <path d="M12 4l7 4v8l-7 4-7-4V8z" />
    </I>,
  ],
  [
    "CircumscribedPolygon",
    "Circumscribed Polygon",
    <I key="cp">
      <circle cx="12" cy="12" r="6.5" strokeWidth="0.9" strokeDasharray="2 1.5" />
      <path d="M12 3.5l7.5 4.3v8.5L12 20.6l-7.5-4.3V7.8z" />
    </I>,
  ],
  [
    "Circle",
    "Circle",
    <I key="c">
      <circle cx="12" cy="12" r="8" />
      <path d="M12 12h8" strokeWidth="1" />
    </I>,
  ],
  [
    "StartEndRadiusArc",
    "Start-End-Radius Arc",
    <I key="a3">
      <path d="M4 17a9 9 0 0116 0" />
      <circle cx="4" cy="17" r="1.4" fill="currentColor" />
      <circle cx="20" cy="17" r="1.4" fill="currentColor" />
      <circle cx="12" cy="8" r="1.4" fill="currentColor" />
    </I>,
  ],
  [
    "CenterEndsArc",
    "Center-ends Arc",
    <I key="ac">
      <path d="M4 16a8 8 0 0116 0" />
      <circle cx="12" cy="16" r="1.4" fill="currentColor" />
      <path d="M12 16L4 16" strokeWidth="0.9" strokeDasharray="1.5 1.5" />
    </I>,
  ],
  [
    "FilletArc",
    "Fillet Arc",
    <I key="f">
      <path d="M4 20V12a8 8 0 018-8h8" />
    </I>,
  ],
  [
    "PickLines",
    "Pick Lines",
    <I key="pl">
      <path d="M4 18L18 4" strokeDasharray="2 2" />
      <path d="M9 20l2-6 3 4z" fill="currentColor" />
    </I>,
  ],
  [
    "PickWalls",
    "Pick Walls",
    <I key="pw">
      <path d="M3 8h18v5H3z" fill="currentColor" fillOpacity="0.25" />
      <path d="M3 15h18" strokeWidth="2" />
      <path d="M14 22l2-6 3 4z" fill="currentColor" />
    </I>,
  ],
];

const TRIM = (
  <I>
    <path d="M4 12h9M13 4v16" />
    <path d="M13 12h7" strokeDasharray="1.5 1.5" />
  </I>
);
const CHECK = (
  <I>
    <path d="M4 12l5 5L20 6" strokeWidth="3" stroke="#2f9a3a" />
  </I>
);
const CROSS = (
  <I>
    <path d="M5 5l14 14M19 5L5 19" strokeWidth="3" stroke="#c0352b" />
  </I>
);
const UNDO = (
  <I>
    <path d="M8 7L4 11l4 4" />
    <path d="M4 11h10a6 6 0 010 12h-2" />
  </I>
);

export function SketchRibbon() {
  const sketch = useAppStore((s) => s.app?.sketch ?? null);
  const ui = useAppStore((s) => s.sketchUi);
  const setUi = useAppStore((s) => s.setSketchUi);
  if (!sketch) return null;
  const what = sketch.kind === "Floor" ? "Floor" : "Ceiling";
  const title = sketch.target
    ? `Modify | ${what}s > Edit Boundary`
    : `Modify | Create ${what} Boundary`;
  const mode = (m: SketchMode, label: string, icon: ReactNode) => (
    <button
      key={m}
      className={`rb-btn rb-small${ui.mode === m ? " active" : ""}`}
      aria-pressed={ui.mode === m}
      onClick={() => setUi({ mode: m, tab: false })}
      title={label}
    >
      {icon}
      <span>{label}</span>
    </button>
  );
  return (
    <div className="ribbon sketch-ribbon" role="toolbar" aria-label="Tools">
      <div className="rb-tabs" role="tablist" aria-label="Ribbon tabs">
        <button role="tab" aria-selected className="rb-tab active contextual">
          {title}
        </button>
      </div>
      <div className="rb-body">
        <div className="rb-group">
          <div className="rb-items">
            <button
              className="rb-btn"
              onClick={() => void finishSketchMode()}
              title="Finish Edit Mode"
            >
              {CHECK}
              <span>Finish</span>
            </button>
            <button
              className="rb-btn"
              onClick={() => void cancelSketchMode()}
              title="Cancel Edit Mode"
            >
              {CROSS}
              <span>Cancel</span>
            </button>
          </div>
          <div className="rb-title">Mode</div>
        </div>
        <div className="rb-group">
          <div className="rb-items">
            {mode("Modify", "Modify", Icons.select)}
            <button
              className="rb-btn"
              onClick={() => void apply(() => ipc.sketchUndo(false))}
              disabled={!sketch.canUndo}
              title="Undo the last sketch change (Ctrl+Z)"
            >
              {UNDO}
              <span>Undo</span>
            </button>
          </div>
          <div className="rb-title">Select</div>
        </div>
        <div className="rb-group">
          <div className="rb-items rb-draw">
            {DRAW.map(([m, label, icon]) => mode(m, label, icon))}
          </div>
          <div className="rb-title">Draw — Boundary Line</div>
        </div>
        <div className="rb-group">
          <div className="rb-items">
            {mode("Trim", "Trim/Extend to Corner", TRIM)}
            <button
              className="rb-btn"
              onClick={flipSketchSelection}
              disabled={ui.sel.length === 0}
              title="Flip picked wall lines to the other face (Space)"
            >
              {Icons.flip}
              <span>Flip</span>
            </button>
            <button
              className="rb-btn"
              onClick={deleteSketchSelection}
              disabled={ui.sel.length === 0}
              title="Delete the selected boundary lines (Del)"
            >
              {Icons.del}
              <span>Delete</span>
            </button>
          </div>
          <div className="rb-title">Modify</div>
        </div>
      </div>
    </div>
  );
}
