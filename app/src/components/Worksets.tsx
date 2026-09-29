import { useEffect, useState } from "react";
import type { WorksetInfo } from "../bindings/WorksetInfo";
import { apply } from "../fileActions";
import { ipc } from "../ipc";
import { useAppStore } from "../store";

// Worksets (ADR-079), as Revit's Collaborate tab has them: the Worksets dialog (New,
// Rename, Delete, Visible in all views) and the Active Workset selector, which also sits in
// the status bar. The system worksets (Views, Families, Project Standards) are Revit's and
// read-only; they show under Show.

/** The Active Workset drop-down: where new elements go. */
export function ActiveWorkset({ compact = false }: { compact?: boolean }) {
  const worksets = useAppStore((s) => s.app?.worksets ?? []);
  const active = useAppStore((s) => s.app?.activeWorkset ?? "");
  if (worksets.length === 0) return null;
  return (
    <label className={compact ? "status-workset" : "rb-field"} title="Active Workset">
      {!compact && <span>Active Workset</span>}
      <select
        aria-label="Active Workset"
        value={active}
        onChange={(e) => void apply(() => ipc.setActiveWorkset(e.target.value))}
      >
        {worksets.map((w) => (
          <option key={w.id} value={w.id}>
            {w.name}
          </option>
        ))}
      </select>
    </label>
  );
}

type Show = "user" | "families" | "standards" | "views";

export function WorksetsDialog({ onClose }: { onClose: () => void }) {
  const revision = useAppStore((s) => s.app?.revision);
  const views = useAppStore((s) => s.app?.views ?? []);
  const [list, setList] = useState<WorksetInfo[]>([]);
  const [chosen, setChosen] = useState<string | null>(null);
  const [show, setShow] = useState<Set<Show>>(new Set(["user"]));
  const [naming, setNaming] = useState<{ mode: "new" | "rename"; name: string } | null>(null);
  const [deleting, setDeleting] = useState<string | null>(null);
  useEffect(() => {
    let live = true;
    ipc.worksetsList().then(
      (l) => live && setList(l),
      () => {},
    );
    return () => {
      live = false;
    };
  }, [revision]);
  const sel = list.find((w) => w.id === chosen) ?? null;
  const toggle = (k: Show) =>
    setShow((s) => {
      const n = new Set(s);
      if (n.has(k)) n.delete(k);
      else n.add(k);
      return n;
    });
  const saveName = async () => {
    if (!naming) return;
    const ok =
      naming.mode === "new"
        ? await apply(() => ipc.createWorkset(naming.name, true))
        : sel && (await apply(() => ipc.renameWorkset(sel.id, naming.name)));
    if (ok) setNaming(null);
  };
  const role = (w: WorksetInfo) =>
    w.role === "Default" ? " (default)" : w.role === "Structural" ? " (structural layer)" : "";
  return (
    <div className="modal-backdrop" role="dialog" aria-label="Worksets">
      <div className="modal worksets-dialog">
        <h2>Worksets</h2>
        <div className="ws-active">
          <ActiveWorkset />
        </div>
        <table className="ws-table">
          <thead>
            <tr>
              <th>Name</th>
              <th>Editable</th>
              <th>Owner</th>
              <th>Opened</th>
              <th>Visible in all views</th>
              <th>Elements</th>
            </tr>
          </thead>
          <tbody>
            {show.has("user") &&
              list.map((w) => (
                <tr
                  key={w.id}
                  className={w.id === chosen ? "on" : ""}
                  onClick={() => setChosen(w.id)}
                  aria-selected={w.id === chosen}
                >
                  <td>
                    {w.name}
                    <span className="muted">{role(w)}</span>
                  </td>
                  <td>Yes</td>
                  <td>You</td>
                  <td>Yes</td>
                  <td>
                    <input
                      type="checkbox"
                      aria-label={`${w.name} visible in all views`}
                      checked={w.visibleInAllViews}
                      onChange={(e) =>
                        void apply(() => ipc.setWorksetVisibleInAllViews(w.id, e.target.checked))
                      }
                    />
                  </td>
                  <td>{w.count}</td>
                </tr>
              ))}
            {show.has("views") &&
              views.map((v) => (
                <tr key={v.id} className="system">
                  <td>
                    {v.viewType === "Sheet" ? "Sheet" : "View"}: {v.name}
                  </td>
                  <td colSpan={5} className="muted">
                    System workset
                  </td>
                </tr>
              ))}
            {show.has("standards") && (
              <tr className="system">
                <td>Project Standards</td>
                <td colSpan={5} className="muted">
                  Types, materials, line styles and settings
                </td>
              </tr>
            )}
            {show.has("families") && (
              <tr className="system">
                <td>Families</td>
                <td colSpan={5} className="muted">
                  Door, window, component and fixture families
                </td>
              </tr>
            )}
          </tbody>
        </table>
        <div className="ws-row">
          <fieldset className="ws-show">
            <legend>Show</legend>
            {(
              [
                ["user", "User-Created"],
                ["families", "Families"],
                ["standards", "Project Standards"],
                ["views", "Views"],
              ] as [Show, string][]
            ).map(([k, label]) => (
              <label key={k} className="ob-check">
                <input type="checkbox" checked={show.has(k)} onChange={() => toggle(k)} />
                {label}
              </label>
            ))}
          </fieldset>
          <div className="ws-buttons">
            <button className="btn-outline" onClick={() => setNaming({ mode: "new", name: "" })}>
              New…
            </button>
            <button
              className="btn-outline"
              disabled={!sel}
              onClick={() => sel && setNaming({ mode: "rename", name: sel.name })}
            >
              Rename…
            </button>
            <button
              className="btn-outline"
              disabled={!sel || sel.role === "Default"}
              title={sel?.role === "Default" ? "The default workset can't be deleted" : undefined}
              onClick={() => sel && setDeleting(list.find((w) => w.role === "Default")?.id ?? null)}
            >
              Delete…
            </button>
          </div>
        </div>
        {naming && (
          <div
            className="ws-inline"
            role="group"
            aria-label={naming.mode === "new" ? "New Workset" : "Rename Workset"}
          >
            <label className="inplace-name">
              {naming.mode === "new" ? "New workset name" : "Rename to"}
              <input
                aria-label="Workset name"
                autoFocus
                value={naming.name}
                onChange={(e) => setNaming({ ...naming, name: e.target.value })}
                onKeyDown={(e) => {
                  if (e.key === "Enter") void saveName();
                  if (e.key === "Escape") setNaming(null);
                }}
              />
            </label>
            <button className="btn-cyan" onClick={() => void saveName()}>
              OK
            </button>
            <button className="btn-outline" onClick={() => setNaming(null)}>
              Cancel
            </button>
          </div>
        )}
        {deleting && sel && (
          <div className="ws-inline" role="group" aria-label="Delete Workset">
            <label className="inplace-name">
              Move the {sel.count} elements on {sel.name} to
              <select
                aria-label="Move elements to"
                value={deleting}
                onChange={(e) => setDeleting(e.target.value)}
              >
                {list
                  .filter((w) => w.id !== sel.id)
                  .map((w) => (
                    <option key={w.id} value={w.id}>
                      {w.name}
                    </option>
                  ))}
              </select>
            </label>
            <button
              className="btn-cyan"
              onClick={async () => {
                if (await apply(() => ipc.deleteWorkset(sel.id, deleting))) {
                  setDeleting(null);
                  setChosen(null);
                }
              }}
            >
              Delete
            </button>
            <button className="btn-outline" onClick={() => setDeleting(null)}>
              Cancel
            </button>
          </div>
        )}
        <div className="modal-actions">
          <button className="btn-cyan" onClick={onClose}>
            OK
          </button>
        </div>
      </div>
    </div>
  );
}

/** Visibility/Graphics > Worksets: each workset shown or hidden in this view. */
export function VisibilityWorksets({ view }: { view: { id: string; hiddenWorksets: string[] } }) {
  const worksets = useAppStore((s) => s.app?.worksets ?? []);
  return (
    <div className="vv-list" role="group" aria-label="Worksets">
      {worksets.map((w) => (
        <label key={w.id} className="ob-check">
          <input
            type="checkbox"
            checked={!view.hiddenWorksets.includes(w.id)}
            onChange={(e) =>
              void apply(() => ipc.setWorksetVisibleInView(view.id, w.id, e.target.checked))
            }
          />
          {w.name}
        </label>
      ))}
    </div>
  );
}
