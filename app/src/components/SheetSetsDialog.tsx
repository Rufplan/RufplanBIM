import { useEffect, useState } from "react";
import type { BuildingType } from "../bindings/BuildingType";
import type { BuildingTypeOption } from "../bindings/BuildingTypeOption";
import type { SetOptions } from "../bindings/SetOptions";
import type { SetPlan } from "../bindings/SetPlan";
import type { SheetSize } from "../bindings/SheetSize";
import { dialogs, errorMessage, ipc } from "../ipc";
import { useAppStore } from "../store";

// Sheet Sets (ADR-032): each design phase's deliverables and their sheets, numbered to the
// US National CAD Standard for the building type, created and exported from here.

export const PHASES = ["PD", "SD", "DD", "CD", "BN", "CA"] as const;
const TYPE_KEY = "rufplan.sheetSets.type";

function savedType(): BuildingType {
  try {
    return (localStorage.getItem(TYPE_KEY) as BuildingType | null) ?? "SingleFamily";
  } catch {
    return "SingleFamily";
  }
}

export function SheetSetsDialog({ onClose }: { onClose: () => void }) {
  const app = useAppStore((s) => s.app);
  const [types, setTypes] = useState<BuildingTypeOption[]>([]);
  const [buildingType, setBuildingType] = useState<BuildingType>(savedType);
  const [phases, setPhases] = useState<Set<string>>(new Set(PHASES));
  const [size, setSize] = useState<SheetSize>("ArchD");
  const [plan, setPlan] = useState<SetPlan | null>(null);
  const [shown, setShown] = useState<string | null>(null);
  const [record, setRecord] = useState(false);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const revision = app?.revision ?? 0;

  useEffect(() => {
    let live = true;
    ipc.buildingTypes().then(
      (t) => live && setTypes(t),
      () => {},
    );
    return () => {
      live = false;
    };
  }, []);

  const options: SetOptions = {
    buildingType,
    phases: PHASES.filter((p) => phases.has(p)),
    size,
  };
  const optionsKey = JSON.stringify(options);
  useEffect(() => {
    let live = true;
    ipc.sheetSetPlan(JSON.parse(optionsKey) as SetOptions).then(
      (p) => live && setPlan(p),
      (e) => useAppStore.getState().setError(errorMessage(e)),
    );
    return () => {
      live = false;
    };
  }, [optionsKey, revision]);

  const stageName = (abbr: string) =>
    app?.stages.find((s) => s.abbreviation === abbr)?.name ?? abbr;
  const toggle = (p: string) => {
    const next = new Set(phases);
    if (next.has(p)) next.delete(p);
    else next.add(p);
    setPhases(next);
  };
  const deliverable = plan?.deliverables.find((d) => `${d.phase}:${d.id}` === shown);
  const rows = (plan?.sheets ?? []).filter(
    (s) => !deliverable || deliverable.sheets.includes(s.number),
  );
  const existing = plan?.sheets.filter((s) => s.exists).length ?? 0;

  const create = async () => {
    setBusy(true);
    setMessage(null);
    try {
      try {
        localStorage.setItem(TYPE_KEY, buildingType);
      } catch {
        /* per-computer convenience only */
      }
      const r = await ipc.createSheetSets(options);
      useAppStore.getState().setApp(r.state);
      const rep = r.report;
      setMessage(
        [
          `${rep.created} sheets created`,
          rep.updated ? `${rep.updated} updated` : "",
          `${rep.views} views placed`,
          rep.sections ? `${rep.sections} building sections made` : "",
          `${rep.placeholders} placeholders`,
        ]
          .filter(Boolean)
          .join(" · ") + (rep.warnings.length ? `. ${rep.warnings.join(". ")}.` : "."),
      );
    } catch (e) {
      useAppStore.getState().setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  const exportPdfs = async () => {
    const folder = await dialogs.pickFolder();
    if (!folder) return;
    setBusy(true);
    setMessage(null);
    try {
      const r = await ipc.exportSheetSets(options.phases, folder, record);
      useAppStore.getState().setApp(r.state);
      setMessage(
        `Exported ${r.files.length} deliverable${r.files.length === 1 ? "" : "s"} to ${folder}: ` +
          r.files.map((f) => `${f.name} (${f.sheets} sheets)`).join(", ") +
          (record ? ". Each is recorded as issued." : "."),
      );
    } catch (e) {
      useAppStore.getState().setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="modal-backdrop" role="dialog" aria-label="Sheet Sets">
      <div className="modal material-browser sheet-sets">
        <div className="mb-head">
          <h2>Sheet Sets</h2>
          <span className="muted">
            Each phase&apos;s deliverables and their sheets, numbered to the US National CAD
            Standard.
          </span>
          <button className="btn-ghost" onClick={onClose} aria-label="Close">
            ×
          </button>
        </div>
        <div className="mb-body">
          <nav className="mb-filters ss-options" aria-label="Options">
            <h3>Building type</h3>
            <select
              aria-label="Building type"
              value={buildingType}
              onChange={(e) => setBuildingType(e.target.value as BuildingType)}
            >
              {types.map((t) => (
                <option key={t.id} value={t.id}>
                  {t.label}
                </option>
              ))}
            </select>
            <h3>Phases</h3>
            {PHASES.map((p) => (
              <label key={p} className="ob-check">
                <input type="checkbox" checked={phases.has(p)} onChange={() => toggle(p)} />
                {p} · {stageName(p)}
              </label>
            ))}
            <h3>Sheet size</h3>
            {(["ArchD", "Tabloid"] as const).map((s) => (
              <label key={s} className="ob-check">
                <input
                  type="radio"
                  name="ss-size"
                  checked={size === s}
                  onChange={() => setSize(s)}
                />
                {s === "ArchD" ? 'ARCH D 36" x 24"' : 'Tabloid 17" x 11"'}
              </label>
            ))}
            <div className="ss-actions">
              <button
                className="btn-cyan"
                disabled={busy || options.phases.length === 0}
                onClick={() => void create()}
              >
                {existing ? "Create / Update Sheets" : "Create Sheets"}
              </button>
              <button
                className="btn-outline"
                disabled={busy || options.phases.length === 0}
                onClick={() => void exportPdfs()}
              >
                Export PDFs…
              </button>
              <label className="ob-check">
                <input
                  type="checkbox"
                  checked={record}
                  onChange={(e) => setRecord(e.target.checked)}
                />
                Record as issued
              </label>
            </div>
          </nav>
          <div className="ss-main">
            <div className="ss-deliverables" role="list" aria-label="Deliverables">
              <button
                role="listitem"
                className={`ss-deliverable${shown === null ? " active" : ""}`}
                onClick={() => setShown(null)}
              >
                <span className="ss-phase">All</span>
                <span>Sheet index</span>
                <span className="muted">{plan?.sheets.length ?? 0} sheets</span>
              </button>
              {plan?.deliverables.map((d) => {
                const k = `${d.phase}:${d.id}`;
                return (
                  <button
                    key={k}
                    role="listitem"
                    className={`ss-deliverable${shown === k ? " active" : ""}`}
                    onClick={() => setShown(k)}
                    title={`${d.stage}: files in Rufplan as "${d.id}"`}
                  >
                    <span className="ss-phase">{d.phase}</span>
                    <span>{d.name}</span>
                    <span className="muted">{d.sheets.length} sheets</span>
                  </button>
                );
              })}
            </div>
            {message && (
              <p className="ss-message" role="status">
                {message}
              </p>
            )}
            {plan?.warnings.map((w) => (
              <p key={w} className="ss-warning">
                {w}
              </p>
            ))}
            <div className="ss-table-wrap">
              <table className="ss-table" aria-label="Sheets">
                <thead>
                  <tr>
                    <th>Number</th>
                    <th>Sheet name</th>
                    <th>Contents</th>
                    {PHASES.filter((p) => phases.has(p)).map((p) => (
                      <th key={p} className="ss-dot">
                        {p}
                      </th>
                    ))}
                    <th />
                  </tr>
                </thead>
                <tbody>
                  {rows.map((s) => (
                    <tr key={s.number} className={s.placeholder ? "ss-placeholder" : ""}>
                      <td className="ss-number">{s.number}</td>
                      <td>{s.name}</td>
                      <td className="ss-contents">{s.contents}</td>
                      {PHASES.filter((p) => phases.has(p)).map((p) => (
                        <td
                          key={p}
                          className="ss-dot"
                          aria-label={`${p} ${s.phases.includes(p) ? "yes" : "no"}`}
                        >
                          {s.phases.includes(p) ? "●" : ""}
                        </td>
                      ))}
                      <td className="muted">{s.exists ? "Update" : "New"}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
