import { useEffect, useState } from "react";
import { errorMessage, ipc, type Table, type ViewInfo } from "../ipc";
import { useAppStore } from "../store";

/** A schedule shown as a table; clicking a row selects its element. */
export function ScheduleView({ view }: { view: ViewInfo }) {
  const revision = useAppStore((s) => s.app?.revision ?? 0);
  const selection = useAppStore((s) => s.selection);
  const select = useAppStore((s) => s.select);
  const [table, setTable] = useState<Table | null>(null);

  useEffect(() => {
    let live = true;
    ipc.scheduleTable(view.id).then(
      (t) => live && setTable(t),
      (e) => useAppStore.getState().setError(errorMessage(e)),
    );
    useAppStore
      .getState()
      .setPrompt("Click a row to select that element and edit it in Properties.");
    return () => {
      live = false;
    };
  }, [view.id, revision]);

  if (!table) return <div className="view-empty">Loading schedule…</div>;
  return (
    <div className="schedule">
      <div className="schedule-title">{table.title}</div>
      <table>
        <thead>
          <tr>
            {table.columns.map((c) => (
              <th key={c}>{c}</th>
            ))}
          </tr>
        </thead>
        <tbody>
          {table.rows.map((row, i) => {
            const id = table.ids[i]!;
            return (
              <tr
                key={id}
                className={selection.includes(id) ? "selected" : ""}
                onClick={() => select([id])}
              >
                {row.map((cell, j) => (
                  <td key={j}>{cell}</td>
                ))}
              </tr>
            );
          })}
          {table.rows.length === 0 && (
            <tr>
              <td colSpan={table.columns.length} className="schedule-empty">
                Nothing to schedule yet.
              </td>
            </tr>
          )}
        </tbody>
      </table>
    </div>
  );
}
