import { useEffect, useState } from "react";
import type { TitleBlockFields } from "../bindings/TitleBlockFields";
import { apply } from "../fileActions";
import { errorMessage, ipc } from "../ipc";

// The title block opened from its sheet (ADR-111), as double-clicking a Revit title block
// edits its labels: the sheet, the project, the architect's firm, the license line and who
// drew and checked the sheet. The project and firm are Project Information, so every
// sheet shows the change; drawn and checked are this sheet's own.

type Key = Exclude<keyof TitleBlockFields, "license" | "signed">;

const SECTIONS: { title: string; note?: string; fields: [Key, string, string?][] }[] = [
  {
    title: "Sheet",
    fields: [
      ["sheetNumber", "Number", "A-101"],
      ["sheetName", "Name", "Floor Plan"],
    ],
  },
  {
    title: "Project",
    note: "Project Information — every sheet shows it.",
    fields: [
      ["projectName", "Name"],
      ["projectNumber", "Number"],
      ["client", "Owner"],
      ["street", "Street"],
      ["city", "City"],
      ["state", "State"],
      ["zip", "ZIP"],
    ],
  },
  {
    title: "Architect",
    fields: [
      ["firm", "Firm"],
      ["firmAddress", "Address"],
      ["firmPhone", "Phone"],
      ["firmEmail", "Email"],
      ["firmWebsite", "Website"],
    ],
  },
];

export function TitleBlockDialog({ sheet, onClose }: { sheet: string; onClose: () => void }) {
  const [f, setF] = useState<TitleBlockFields | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let live = true;
    ipc.titleBlockFields(sheet).then(
      (x) => live && setF(x),
      (e) => setError(errorMessage(e)),
    );
    return () => {
      live = false;
    };
  }, [sheet]);

  const save = async () => {
    if (!f) return;
    if (await apply(() => ipc.setTitleBlockFields(sheet, f))) onClose();
  };
  const input = (label: string, value: string, set: (v: string) => void, hint?: string) => (
    <label key={label} className="field">
      {label}
      <input
        aria-label={label}
        value={value}
        placeholder={hint}
        onChange={(e) => set(e.target.value)}
      />
    </label>
  );

  return (
    <div className="modal-backdrop" role="dialog" aria-label="Title Block">
      <div className="modal title-block-dialog">
        <h2>Title Block</h2>
        {error && <p className="error">{error}</p>}
        {f && (
          <div className="tb-grid">
            {SECTIONS.map((s) => (
              <section key={s.title}>
                <h3>{s.title}</h3>
                {s.note && <p className="muted">{s.note}</p>}
                {s.fields.map(([k, label, hint]) =>
                  input(`${s.title} ${label}`, f[k], (v) => setF({ ...f, [k]: v }), hint),
                )}
              </section>
            ))}
            <section>
              <h3>Stamp</h3>
              <p className="muted">
                California asks for the license number and renewal on every sheet.
              </p>
              {input(
                "License No.",
                f.license.number,
                (v) => setF({ ...f, license: { ...f.license, number: v } }),
                "C-12345",
              )}
              {input(
                "Renews",
                f.license.renews,
                (v) => setF({ ...f, license: { ...f.license, renews: v } }),
                "06/30/2028",
              )}
              <h3>This sheet</h3>
              {input(
                "Drawn By",
                f.signed.drawn,
                (v) => setF({ ...f, signed: { ...f.signed, drawn: v } }),
                "Initials",
              )}
              {input(
                "Checked By",
                f.signed.checked,
                (v) => setF({ ...f, signed: { ...f.signed, checked: v } }),
                "Initials",
              )}
            </section>
          </div>
        )}
        <div className="modal-actions">
          <button className="btn-cyan" disabled={!f} onClick={() => void save()}>
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
