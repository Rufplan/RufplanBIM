import { useEffect, useState } from "react";
import type { NotesBuilding } from "../bindings/NotesBuilding";
import type { NotesDrawing } from "../bindings/NotesDrawing";
import type { NotesOptions } from "../bindings/NotesOptions";
import { apply } from "../fileActions";
import { ipc } from "../ipc";
import { activeViewInfo, useAppStore } from "../store";

// Annotate > General Notes (ADR-103): numbered notes preset by building type and by the
// drawing they go on, picked and edited, then placed as one text note.

export function GeneralNotesDialog({ onClose }: { onClose: () => void }) {
  const view = useAppStore((s) => activeViewInfo(s));
  const [opts, setOpts] = useState<NotesOptions | null>(null);
  const [building, setBuilding] = useState<NotesBuilding>("SingleFamily");
  const [drawing, setDrawing] = useState<NotesDrawing>("General");
  const [code, setCode] = useState("");
  const [heading, setHeading] = useState("GENERAL NOTES");
  const [notes, setNotes] = useState<{ text: string; on: boolean }[]>([]);
  const [extra, setExtra] = useState("");
  const [width, setWidth] = useState(160);

  useEffect(() => {
    if (!view) return;
    let live = true;
    ipc.generalNotesOptions(view.id).then(
      (o) => {
        if (!live || !o) return;
        setOpts(o);
        setBuilding(o.building);
        setDrawing(o.drawing);
        setCode(o.code);
      },
      () => {},
    );
    return () => {
      live = false;
    };
  }, [view]);

  // The presets for the choice; picking another building or drawing starts over.
  useEffect(() => {
    if (!opts) return;
    let live = true;
    ipc.generalNotesPreset(building, drawing, code).then(
      (r) => {
        if (!live || !r) return;
        const [h, list] = r;
        setHeading(h);
        setNotes(list.map((text) => ({ text, on: true })));
      },
      () => {},
    );
    return () => {
      live = false;
    };
  }, [opts, building, drawing, code]);

  const chosen = [
    ...notes.filter((n) => n.on).map((n) => n.text),
    ...extra
      .split("\n")
      .map((l) => l.trim())
      .filter(Boolean),
  ];
  const place = async () => {
    if (!view) return;
    if (await apply(() => ipc.placeGeneralNotes(view.id, heading, chosen, width))) onClose();
  };

  return (
    <div className="modal-backdrop" role="dialog" aria-label="General Notes">
      <div className="modal general-notes">
        <h2>General Notes</h2>
        {!view ? (
          <p>Open a plan, elevation, section, drafting view or sheet to place notes in.</p>
        ) : (
          <>
            <div className="row">
              <label className="field grow">
                Building type
                <select
                  aria-label="Building type"
                  value={building}
                  onChange={(e) => setBuilding(e.target.value as NotesBuilding)}
                >
                  {opts?.buildings.map(([b, label]) => (
                    <option key={b} value={b}>
                      {label}
                    </option>
                  ))}
                </select>
              </label>
              <label className="field grow">
                Drawing
                <select
                  aria-label="Drawing"
                  value={drawing}
                  onChange={(e) => setDrawing(e.target.value as NotesDrawing)}
                >
                  {opts?.drawings.map(([d, label]) => (
                    <option key={d} value={d}>
                      {label}
                    </option>
                  ))}
                </select>
              </label>
            </div>
            <div className="row">
              <label className="field grow">
                Heading
                <input
                  aria-label="Heading"
                  value={heading}
                  onChange={(e) => setHeading(e.target.value)}
                />
              </label>
              <label className="field">
                Code
                <input
                  aria-label="Code"
                  placeholder={
                    building === "SingleFamily" || building === "DuplexTownhouse"
                      ? "2021 IRC"
                      : "2021 IBC"
                  }
                  value={code}
                  onChange={(e) => setCode(e.target.value)}
                />
              </label>
              <label className="field small">
                Width (mm)
                <input
                  aria-label="Width"
                  type="number"
                  min={60}
                  max={400}
                  value={width}
                  onChange={(e) => setWidth(Number(e.target.value) || 160)}
                />
              </label>
            </div>
            <ol className="gn-list">
              {notes.map((n, i) => (
                <li key={i}>
                  <label>
                    <input
                      type="checkbox"
                      checked={n.on}
                      onChange={(e) =>
                        setNotes((cur) =>
                          cur.map((x, k) => (k === i ? { ...x, on: e.target.checked } : x)),
                        )
                      }
                    />
                    <span>{n.text}</span>
                  </label>
                </li>
              ))}
            </ol>
            <label className="field">
              Your own notes (one per line, added at the end)
              <textarea
                aria-label="Your own notes"
                rows={3}
                value={extra}
                onChange={(e) => setExtra(e.target.value)}
              />
            </label>
          </>
        )}
        <div className="modal-actions">
          <button
            className="btn-cyan"
            disabled={!view || chosen.length === 0}
            onClick={() => void place()}
          >
            Place {chosen.length} Notes
          </button>
          <button className="btn-ghost" onClick={onClose}>
            Cancel
          </button>
        </div>
        <p className="muted">
          Placed in {view?.name ?? "the view"} beside the drawing (on a sheet, beside the title
          block) as one text note you can move, stretch and edit.
        </p>
      </div>
    </div>
  );
}
