import { useState } from "react";
import type { ViewInfo } from "../bindings/ViewInfo";
import { apply } from "../fileActions";
import { ipc } from "../ipc";

// Rename (ADR-074): Revit's Rename View, or for a sheet its Sheet Title (number and name).

export function RenameDialog({ view, onClose }: { view: ViewInfo; onClose: () => void }) {
  const sheet = view.viewType === "Sheet";
  // A sheet's browser label is "A101 - Floor Plan".
  const [num0, name0] = sheet ? splitSheet(view.name) : ["", view.name];
  const [name, setName] = useState(name0);
  const [number, setNumber] = useState(num0);
  const ok = async () => {
    let done = true;
    if (sheet && number.trim() && number !== num0)
      done = await apply(() => ipc.setProperty(view.id, "number", number.trim()));
    if (done && name.trim() && name !== name0)
      done = await apply(() => ipc.setProperty(view.id, "name", name.trim()));
    if (done) onClose();
  };
  const key = (e: React.KeyboardEvent) => {
    if (e.key === "Enter") void ok();
    if (e.key === "Escape") onClose();
  };
  return (
    <div
      className="modal-backdrop"
      role="dialog"
      aria-label={sheet ? "Sheet Title" : "Rename View"}
    >
      <div className="modal drafting-dialog">
        <h2>{sheet ? "Sheet Title" : "Rename View"}</h2>
        {sheet && (
          <label className="inplace-name">
            Number
            <input
              aria-label="Number"
              value={number}
              onChange={(e) => setNumber(e.target.value)}
              onKeyDown={key}
            />
          </label>
        )}
        <label className="inplace-name">
          Name
          <input
            aria-label="Name"
            autoFocus
            value={name}
            onChange={(e) => setName(e.target.value)}
            onKeyDown={key}
          />
        </label>
        <div className="mb-actions">
          <button className="btn-cyan" onClick={() => void ok()}>
            OK
          </button>
          <button className="btn-outline" onClick={onClose}>
            Cancel
          </button>
        </div>
      </div>
    </div>
  );
}

function splitSheet(label: string): [string, string] {
  const i = label.indexOf(" - ");
  return i < 0 ? ["", label] : [label.slice(0, i), label.slice(i + 3)];
}
