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
  let body: React.ReactNode = null;
  if (tool === "copy") body = check("copyMultiple", "Multiple");
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
