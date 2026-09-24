import { TOOL_LABELS, useAppStore } from "../store";

// Revit's options bar: settings of the active modify tool, under the ribbon.

export function OptionsBar() {
  const tool = useAppStore((s) => s.tool);
  const o = useAppStore((s) => s.options);
  const set = useAppStore((s) => s.setOption);
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
  else if (["wall", "grid", "move", "stair"].includes(tool))
    body = <span className="ob-hint">Type a length after the first click, then Enter</span>;
  if (!body) return null;
  return (
    <div className="options-bar" role="group" aria-label="Tool options">
      <span className="ob-tool">{TOOL_LABELS[tool]}</span>
      {body}
    </div>
  );
}
