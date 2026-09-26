import { useAppStore } from "../store";

// What an IFC import brought in, and how to go back and forth with Revit (ADR-035).

export function IfcReport() {
  const r = useAppStore((s) => s.ifcReport);
  if (!r) return null;
  const close = () => useAppStore.getState().setIfcReport(null);
  const rows: [string, number][] = [
    ["Levels", r.levels],
    ["Walls", r.walls],
    ["Doors", r.doors],
    ["Windows", r.windows],
    ["Floors", r.floors],
    ["Roofs", r.roofs],
    ["Rooms", r.rooms],
    ["Grids", r.grids],
    ["Columns", r.columns],
  ];
  return (
    <div className="modal-backdrop" role="dialog" aria-label="IFC Import">
      <div className="modal ifc-report">
        <h2>IFC Import</h2>
        <p className="muted">
          {r.application || "Unknown application"} · {r.schema}
          {r.project ? ` · ${r.project}` : ""}
        </p>
        <dl className="mb-settings">
          {rows
            .filter(([, n]) => n > 0)
            .map(([k, n]) => (
              <div key={k}>
                <dt>{k}</dt>
                <dd>{n}</dd>
              </div>
            ))}
        </dl>
        {r.skipped.length > 0 && (
          <p className="muted">
            Not brought in: {r.skipped.map(([k, n]) => `${n} ${k}`).join(", ")}.
          </p>
        )}
        {r.warnings.map((w) => (
          <p key={w} className="ss-warning">
            {w}
          </p>
        ))}
        <details>
          <summary>Working with Revit</summary>
          <ol className="ifc-steps">
            <li>
              In Revit: File › Export › IFC, choose IFC4 Reference View (or IFC2x3 Coordination View
              2.0), and export.
            </li>
            <li>
              Here: Open IFC… — walls, doors, windows, floors, roofs, rooms and grids arrive as
              editable Studio elements.
            </li>
            <li>
              Back to Revit: View › Export IFC here, then in Revit File › Open › IFC (or Insert ›
              Link IFC to coordinate against it).
            </li>
          </ol>
        </details>
        <div className="modal-actions">
          <button className="btn-cyan" onClick={close}>
            OK
          </button>
        </div>
      </div>
    </div>
  );
}
