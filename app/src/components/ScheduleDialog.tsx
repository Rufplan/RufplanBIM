import { useEffect, useState } from "react";
import type { IndexRow } from "../bindings/IndexRow";
import type { ScheduleStyle } from "../bindings/ScheduleStyle";
import type { TextFont } from "../bindings/TextFont";
import { apply } from "../fileActions";
import { errorMessage, ipc, type Table } from "../ipc";
import { useAppStore } from "../store";
import { SheetIndexEditor } from "./SheetIndexEditor";

// A schedule opened from its sheet (ADR-110), as Revit opens one on double-click: its
// appearance (font, title, header and body text, row height) with a preview, and, for the
// sheet index, its rows: the sheets to renumber, rename, add and order, and placeholders
// (ADR-113).

/** Text heights offered, inches → paper mm (3/32" is the floor). */
const HEIGHTS: [number, string][] = [
  [(3 / 32) * 25.4, `3/32"`],
  [(1 / 8) * 25.4, `1/8"`],
  [(5 / 32) * 25.4, `5/32"`],
  [(3 / 16) * 25.4, `3/16"`],
  [(1 / 4) * 25.4, `1/4"`],
  [(3 / 8) * 25.4, `3/8"`],
  [(1 / 2) * 25.4, `1/2"`],
];

const FAMILY: Record<TextFont, string> = {
  Drafting: `"Barlow Condensed", "Barlow", sans-serif`,
  Sans: `Carlito, Calibri, Arial, sans-serif`,
  Serif: `Tinos, "Times New Roman", serif`,
};

/** The offered height nearest `mm`. */
function nearest(mm: number): number {
  return HEIGHTS.reduce((a, b) => (Math.abs(b[0] - mm) < Math.abs(a[0] - mm) ? b : a))[0];
}

export function ScheduleDialog({ view, onClose }: { view: string; onClose: () => void }) {
  const revision = useAppStore((s) => s.app?.revision ?? 0);
  const [table, setTable] = useState<Table | null>(null);
  const [style, setStyle] = useState<ScheduleStyle | null>(null);
  const [fonts, setFonts] = useState<[TextFont, string][]>([]);
  const [rows, setRows] = useState<IndexRow[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let live = true;
    ipc.scheduleTable(view).then(
      (t) => live && setTable(t),
      (e) => setError(errorMessage(e)),
    );
    return () => {
      live = false;
    };
  }, [view, revision]);
  useEffect(() => {
    let live = true;
    void Promise.all([ipc.scheduleStyle(view), ipc.textFonts(), ipc.sheetIndexRows()]).then(
      ([s, f, r]) => {
        if (!live) return;
        setStyle(s);
        setFonts(f);
        setRows(r);
      },
      (e) => setError(errorMessage(e)),
    );
    return () => {
      live = false;
    };
  }, [view]);

  const sheetIndex = table?.columns[0] === "Sheet Number";
  const set = (patch: Partial<ScheduleStyle>) => style && setStyle({ ...style, ...patch });
  const save = async () => {
    if (!style) return;
    const ok = await apply(() => ipc.setScheduleStyle(view, style));
    if (!ok) return;
    if (sheetIndex && rows) {
      const kept = rows.filter((r) => r.sheet !== null || r.number.trim() || r.name.trim());
      if (!(await apply(() => ipc.setSheetIndexRows(kept)))) return;
    }
    onClose();
  };
  // The preview at about 3.6 px per paper mm.
  const px = (mm: number) => `${(mm * 3.6).toFixed(1)}px`;

  return (
    <div className="modal-backdrop" role="dialog" aria-label="Schedule">
      <div className="modal schedule-dialog">
        <h2>{table?.title ?? "Schedule"}</h2>
        {error && <p className="error">{error}</p>}
        {style && (
          <div className="row">
            <label className="field grow">
              Font
              <select
                aria-label="Font"
                value={style.font}
                onChange={(e) => set({ font: e.target.value as TextFont })}
              >
                {fonts.map(([f, label]) => (
                  <option key={f} value={f}>
                    {label}
                  </option>
                ))}
              </select>
            </label>
            {(["title", "header", "body"] as const).map((k) => (
              <label key={k} className="field small">
                {k === "title" ? "Title" : k === "header" ? "Headers" : "Body"}
                <select
                  aria-label={`${k} text`}
                  value={nearest(style[k])}
                  onChange={(e) => set({ [k]: Number(e.target.value) })}
                >
                  {HEIGHTS.map(([mm, label]) => (
                    <option key={label} value={mm}>
                      {label}
                    </option>
                  ))}
                </select>
              </label>
            ))}
            <label className="field small">
              Row (mm)
              <input
                aria-label="Row height"
                type="number"
                min={4}
                max={40}
                step={0.5}
                value={style.row}
                onChange={(e) => set({ row: Number(e.target.value) || style.row })}
              />
            </label>
          </div>
        )}
        {table && style && (
          <div className="schedule-preview" style={{ fontFamily: FAMILY[style.font] }}>
            <div className="sp-title" style={{ fontSize: px(style.title) }}>
              {table.title}
            </div>
            <table>
              <thead>
                <tr>
                  {table.columns.map((c) => (
                    <th key={c} style={{ fontSize: px(style.header), height: px(style.row) }}>
                      {c.toUpperCase()}
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {(sheetIndex && rows ? rows.map((r) => [r.number, r.name]) : table.rows)
                  .slice(0, 40)
                  .map((r, i) => (
                    <tr key={i} style={{ height: px(style.row) }}>
                      {r.map((c, j) => (
                        <td key={j} style={{ fontSize: px(style.body) }}>
                          {c}
                        </td>
                      ))}
                    </tr>
                  ))}
              </tbody>
            </table>
          </div>
        )}
        {sheetIndex && rows && <SheetIndexEditor rows={rows} onChange={setRows} />}
        <div className="modal-actions">
          <button className="btn-cyan" disabled={!style} onClick={() => void save()}>
            OK
          </button>
          <button className="btn-ghost" onClick={onClose}>
            Cancel
          </button>
        </div>
      </div>
    </div>
  );
}
