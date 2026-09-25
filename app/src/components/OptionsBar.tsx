import { useEffect, useState } from "react";
import { ipc } from "../ipc";
import { TOOL_LABELS, useAppStore } from "../store";

type Choices = [[string, string][], [string, string][]];

/** Location lines and stair shapes, from Rust (so labels match the properties panel). */
function useDrawingOptions(): Choices | null {
  const [choices, setChoices] = useState<Choices | null>(null);
  useEffect(() => {
    let live = true;
    ipc.drawingOptions().then(
      (c) => live && setChoices(c),
      () => {},
    );
    return () => {
      live = false;
    };
  }, []);
  return choices;
}

// Revit's options bar: settings of the active modify tool, under the ribbon.

export function OptionsBar() {
  const tool = useAppStore((s) => s.tool);
  const o = useAppStore((s) => s.options);
  const set = useAppStore((s) => s.setOption);
  const choices = useDrawingOptions();
  const pick = (key: "wallLocation" | "stairShape", label: string, list: [string, string][]) => (
    <label className="ob-field">
      {label}
      <select aria-label={label} value={o[key]} onChange={(e) => set(key, e.target.value)}>
        {list.map(([id, text]) => (
          <option key={id} value={id}>
            {text}
          </option>
        ))}
      </select>
    </label>
  );
  const typed = <span className="ob-hint">Type a length after the first click, then Enter</span>;
  const check = (key: "copyMultiple" | "rotateCopy" | "mirrorCopy", label: string) => (
    <label className="ob-check">
      <input type="checkbox" checked={o[key]} onChange={(e) => set(key, e.target.checked)} />
      {label}
    </label>
  );
  const ui = useAppStore((s) => s.sketchUi);
  const setUi = useAppStore((s) => s.setSketchUi);
  const markTypes = useAppStore((s) => s.app?.elevationMarkerTypes);
  const elevationType = useAppStore((s) => s.elevationType);
  const setElevationType = useAppStore((s) => s.setElevationType);
  const levels = useAppStore((s) => s.app?.levels);
  const level3d = useAppStore((s) => s.level3d);
  const setLevel3d = useAppStore((s) => s.setLevel3d);
  const in3d = useAppStore(
    (s) => s.app?.views.find((v) => v.id === s.activeView)?.viewType === "ThreeD",
  );
  let body: React.ReactNode = null;
  const sketchCheck = (key: "chain" | "radiusOn" | "core" | "lock", label: string) => (
    <label className="ob-check">
      <input
        type="checkbox"
        checked={ui[key]}
        onChange={(e) => setUi({ [key]: e.target.checked })}
      />
      {label}
    </label>
  );
  const sketchText = (key: "offset" | "radius", label: string, disabled = false) => (
    <label className="ob-field">
      {label}
      <input
        aria-label={label}
        value={ui[key]}
        disabled={disabled}
        onChange={(e) => setUi({ [key]: e.target.value })}
      />
    </label>
  );
  if (tool === "sketch") {
    const m = ui.mode;
    const radius = (
      <>
        {sketchCheck("radiusOn", "Radius")}
        {sketchText("radius", "Radius value", !ui.radiusOn)}
      </>
    );
    body =
      m === "Line" ? (
        <>
          {sketchCheck("chain", "Chain")}
          {sketchText("offset", "Offset")}
          {radius}
        </>
      ) : m === "Rectangle" ? (
        <>
          {sketchText("offset", "Offset")}
          {radius}
        </>
      ) : m === "InscribedPolygon" || m === "CircumscribedPolygon" ? (
        <>
          <label className="ob-field">
            Sides
            <input
              type="number"
              min={3}
              max={64}
              aria-label="Sides"
              value={ui.sides || ""}
              onChange={(e) => setUi({ sides: Number(e.target.value) || 0 })}
            />
          </label>
          {sketchText("offset", "Offset")}
        </>
      ) : m === "Circle" ? (
        sketchText("offset", "Offset")
      ) : m === "FilletArc" ? (
        sketchText("radius", "Radius")
      ) : m === "PickWalls" ? (
        <>
          {sketchText("offset", "Offset")}
          {sketchCheck("core", "Extend into wall (to core)")}
          <span className="ob-hint">
            {ui.tab ? "Chain: all connected walls" : "Tab picks a chain of walls"}
          </span>
        </>
      ) : m === "PickLines" ? (
        <>
          {sketchText("offset", "Offset")}
          {sketchCheck("lock", "Lock")}
        </>
      ) : (
        <span className="ob-hint">
          {m === "Trim"
            ? "Click the parts to keep"
            : "Select lines; drag ends; Space flips; Del deletes"}
        </span>
      );
  } else if (tool === "elevation")
    body = (
      <label className="ob-field">
        Type
        <select
          aria-label="Elevation type"
          value={elevationType ?? markTypes?.[0]?.id ?? ""}
          onChange={(e) => setElevationType(e.target.value)}
        >
          {(markTypes ?? []).map((t) => (
            <option key={t.id} value={t.id}>
              {t.name}
            </option>
          ))}
        </select>
      </label>
    );
  else if (in3d && (tool === "wall" || tool === "column"))
    body = (
      <label className="ob-field">
        Level
        <select
          aria-label="Placement level"
          value={level3d ?? levels?.[0]?.id ?? ""}
          onChange={(e) => setLevel3d(e.target.value)}
        >
          {(levels ?? []).map((l) => (
            <option key={l.id} value={l.id}>
              {l.name}
            </option>
          ))}
        </select>
      </label>
    );
  else if (tool === "copy") body = check("copyMultiple", "Multiple");
  else if (tool === "rotate") body = check("rotateCopy", "Copy");
  else if (tool === "mirror") body = check("mirrorCopy", "Copy");
  else if (tool === "array")
    body = (
      <label className="ob-field">
        Number
        <input
          type="number"
          min={2}
          max={200}
          aria-label="Number of items"
          value={o.arrayCount || ""}
          // Clamped when the array is made, so the field can be cleared while typing.
          onChange={(e) => set("arrayCount", Math.min(200, Number(e.target.value) || 0))}
        />
      </label>
    );
  else if (tool === "camera")
    body = (
      <>
        <label className="ob-check">
          <input type="checkbox" checked readOnly aria-label="Perspective" />
          Perspective
        </label>
        <label className="ob-field">
          Offset
          <input
            aria-label="Camera eye height"
            title="Eye height above the plan's level"
            value={o.cameraHeight}
            onChange={(e) => set("cameraHeight", e.target.value)}
          />
        </label>
      </>
    );
  else if (tool === "offset")
    body = (
      <label className="ob-field">
        Offset
        <input
          aria-label="Offset distance"
          value={o.offsetDistance}
          onChange={(e) => set("offsetDistance", e.target.value)}
        />
      </label>
    );
  else if (tool === "wall")
    body = (
      <>
        {choices && pick("wallLocation", "Location Line", choices[0])}
        {typed}
      </>
    );
  else if (tool === "stair")
    body = (
      <>
        {choices && pick("stairShape", "Shape", choices[1])}
        {typed}
      </>
    );
  else if (["grid", "move", "beam"].includes(tool)) body = typed;
  else if (tool === "railing")
    body = <span className="ob-hint">Click the path's points; Enter finishes</span>;
  if (!body) return null;
  return (
    <div className="options-bar" role="group" aria-label="Tool options">
      <span className="ob-tool">{TOOL_LABELS[tool]}</span>
      {body}
    </div>
  );
}
