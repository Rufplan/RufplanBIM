import { useEffect, useMemo, useState } from "react";
import type { Assignable } from "../bindings/Assignable";
import type { Keynote } from "../bindings/Keynote";
import type { KeynoteTableInfo } from "../bindings/KeynoteTableInfo";
import { apply } from "../fileActions";
import { dialogs, errorMessage, ipc } from "../ipc";
import { useAppStore } from "../store";
import { KeynotePicker, KeynoteTree } from "./KeynotePicker";

// The Keynote Manager (ADR-081): Revit's keynote file, Keynote Settings and every type's
// Keynote parameter in one place. Browse and search the table, edit keynotes in place (a
// renamed key follows through to its tags and assignments), load or save Revit keynote
// files, choose the numbering, and assign keynotes to types and materials in bulk.

type Tab = "keynotes" | "assign";

const CATEGORY_LABELS: Record<string, string> = {
  WallType: "Walls",
  FloorType: "Floors",
  CeilingType: "Ceilings",
  RoofType: "Roofs",
  DoorType: "Doors",
  WindowType: "Windows",
  ColumnType: "Columns",
  BeamType: "Beams",
  RailingType: "Railings",
  LightingFixtureType: "Lighting",
  PlantingType: "Planting",
  Material: "Materials",
};

function useTable(): KeynoteTableInfo | null {
  const revision = useAppStore((s) => s.app?.revision);
  const [t, setT] = useState<KeynoteTableInfo | null>(null);
  useEffect(() => {
    let live = true;
    ipc.keynoteTable().then(
      (x) => live && setT(x),
      () => {},
    );
    return () => {
      live = false;
    };
  }, [revision]);
  return t;
}

function Editor({
  table,
  selected,
  onSelect,
}: {
  table: KeynoteTableInfo;
  selected: string | null;
  onSelect: (key: string | null) => void;
}) {
  const current = table.entries.find((k) => k.key === selected) ?? null;
  const [draft, setDraft] = useState<Keynote & { isNew: boolean }>(
    current ? { ...current, isNew: false } : { key: "", text: "", isNew: true },
  );
  const [confirm, setConfirm] = useState(false);
  const use = table.usage.find((u) => u.key === selected);
  const children = table.entries.filter((k) => k.parent === selected).length;
  const byKey = new Map(table.entries.map((k) => [k.key, k]));
  const trail: Keynote[] = [];
  for (
    let p = current?.parent ? byKey.get(current.parent) : undefined;
    p;
    p = p.parent ? byKey.get(p.parent) : undefined
  )
    trail.unshift(p);
  const save = async () => {
    const ok = await apply(() =>
      ipc.keynoteSave(draft.isNew ? null : (current?.key ?? null), {
        key: draft.key,
        text: draft.text,
        parent: draft.parent || undefined,
      }),
    );
    if (ok) onSelect(draft.key.trim());
  };
  const addChild = () => {
    const kids = table.entries.filter((k) => k.parent === current?.key);
    const base = current?.key ?? "";
    // Revit's pattern: the section, then .A1, .A2…
    const next = current?.parent ? `${base}.A${kids.length + 1}` : `${base} `;
    setDraft({ key: next, text: "", parent: current?.key, isNew: true });
    setConfirm(false);
  };
  return (
    <div className="kn-editor" aria-label="Keynote editor">
      {!draft.isNew && current && (
        <nav className="kn-trail" aria-label="Filed under">
          {trail.map((t) => (
            <button key={t.key} className="btn-ghost" onClick={() => onSelect(t.key)}>
              {t.key} {t.text}
            </button>
          ))}
        </nav>
      )}
      <h3>{draft.isNew ? "New keynote" : "Keynote"}</h3>
      <label className="kn-field">
        Key
        <input
          aria-label="Key"
          value={draft.key}
          onChange={(e) => setDraft({ ...draft, key: e.target.value })}
        />
      </label>
      <label className="kn-field">
        Text
        <textarea
          aria-label="Text"
          rows={3}
          value={draft.text}
          onChange={(e) => setDraft({ ...draft, text: e.target.value })}
        />
      </label>
      <label className="kn-field">
        Filed under
        <select
          aria-label="Filed under"
          value={draft.parent ?? ""}
          onChange={(e) => setDraft({ ...draft, parent: e.target.value || undefined })}
        >
          <option value="">(top level: a division)</option>
          {table.entries
            .filter((k) => k.key !== current?.key || draft.isNew)
            .map((k) => (
              <option key={k.key} value={k.key}>
                {k.key} — {k.text}
              </option>
            ))}
        </select>
      </label>
      <div className="kn-actions">
        <button
          className="btn-cyan"
          disabled={!draft.key.trim() || !draft.text.trim()}
          onClick={() => void save()}
        >
          {draft.isNew ? "Add Keynote" : "Save Changes"}
        </button>
        {!draft.isNew && current && (
          <button className="btn-outline" onClick={addChild}>
            Add Keynote Under This
          </button>
        )}
        {draft.isNew && (
          <button
            className="btn-ghost"
            onClick={() =>
              setDraft(current ? { ...current, isNew: false } : { key: "", text: "", isNew: true })
            }
          >
            Cancel
          </button>
        )}
      </div>
      {!draft.isNew && current && (
        <div className="kn-usage">
          <p>
            {use?.tags
              ? `${use.tags} tag${use.tags === 1 ? "" : "s"} in views`
              : "Not tagged in any view"}
            {use?.assigned.length ? ` · assigned to ${use.assigned.join(", ")}` : ""}
            {children ? ` · ${children} keynote${children === 1 ? "" : "s"} under it` : ""}
          </p>
          {confirm ? (
            <div className="kn-confirm" role="alert">
              Delete {current.key}
              {children ? " and everything under it" : ""}? Its user tags go too and assignments are
              cleared (undo brings them back).
              <button
                className="btn-danger"
                onClick={async () => {
                  if (await apply(() => ipc.keynoteDelete(current.key)))
                    onSelect(current.parent ?? null);
                }}
              >
                Delete
              </button>
              <button className="btn-ghost" onClick={() => setConfirm(false)}>
                Keep
              </button>
            </div>
          ) : (
            <button className="btn-ghost kn-delete" onClick={() => setConfirm(true)}>
              Delete…
            </button>
          )}
        </div>
      )}
    </div>
  );
}

function KeynotesTab({ table }: { table: KeynoteTableInfo }) {
  const [query, setQuery] = useState("");
  const [sel, setSel] = useState<string | null>(null);
  const [importing, setImporting] = useState(false);
  const usage = useMemo(
    () => new Map(table.usage.map((u) => [u.key, u.tags + u.assigned.length])),
    [table.usage],
  );
  const load = async (replace: boolean) => {
    setImporting(false);
    const path = await dialogs.pickKeynoteFile();
    if (!path) return;
    const s = useAppStore.getState();
    try {
      const [n, state] = await ipc.keynoteImport(path, replace);
      if (state) s.setApp(state);
      s.setPrompt(`Loaded ${n} keynotes from ${path}`);
    } catch (e) {
      s.setError(errorMessage(e));
    }
  };
  const save = async () => {
    const path = await dialogs.pickKeynoteSaveLocation("Keynotes");
    if (!path) return;
    const s = useAppStore.getState();
    try {
      s.setPrompt(`Saved the keynote file: ${await ipc.keynoteExport(path)}`);
    } catch (e) {
      s.setError(errorMessage(e));
    }
  };
  return (
    <div className="kn-tab">
      <div className="kn-toolbar">
        <input
          className="kn-search"
          aria-label="Search keynotes"
          placeholder={`Search ${table.entries.length} keynotes`}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
        <div className="kn-seg" role="radiogroup" aria-label="Numbering">
          {(
            [
              ["ByKeynote", "By keynote"],
              ["BySheet", "By sheet"],
            ] as const
          ).map(([v, label]) => (
            <button
              key={v}
              role="radio"
              aria-checked={table.numbering === v}
              className={table.numbering === v ? "on" : ""}
              title={
                v === "BySheet"
                  ? "Tags read 1, 2, 3… on each sheet"
                  : "Tags read their key, e.g. 09 29 00.A1"
              }
              onClick={() => void apply(() => ipc.keynoteSetNumbering(v))}
            >
              {label}
            </button>
          ))}
        </div>
        <div className="kn-file">
          <button
            className="btn-outline"
            onClick={() => setImporting(!importing)}
            aria-expanded={importing}
          >
            Load File…
          </button>
          {importing && (
            <div className="ctx-menu kn-menu" role="menu" aria-label="Load keynote file">
              <button role="menuitem" className="ctx-item" onClick={() => void load(false)}>
                Merge into this table
              </button>
              <button role="menuitem" className="ctx-item" onClick={() => void load(true)}>
                Replace this table
              </button>
            </div>
          )}
          <button className="btn-outline" onClick={() => void save()}>
            Save File…
          </button>
        </div>
      </div>
      <div className="kn-body">
        <div className="kn-left">
          <KeynoteTree
            entries={table.entries}
            query={query}
            selected={sel}
            onSelect={setSel}
            usage={usage}
          />
          <button className="btn-ghost" onClick={() => setSel("")}>
            + New division
          </button>
        </div>
        <Editor key={sel ?? "none"} table={table} selected={sel || null} onSelect={setSel} />
      </div>
    </div>
  );
}

function AssignTab({ table }: { table: KeynoteTableInfo }) {
  const revision = useAppStore((s) => s.app?.revision);
  const [items, setItems] = useState<Assignable[]>([]);
  const [cat, setCat] = useState<string>("all");
  const [unassigned, setUnassigned] = useState(false);
  const [query, setQuery] = useState("");
  const [checked, setChecked] = useState<Set<string>>(new Set());
  const [picking, setPicking] = useState<string[] | null>(null);
  useEffect(() => {
    let live = true;
    ipc.keynoteAssignables().then(
      (x) => live && setItems(x),
      () => {},
    );
    return () => {
      live = false;
    };
  }, [revision]);
  const text = new Map(table.entries.map((k) => [k.key, k.text]));
  const cats = [...new Set(items.map((i) => i.category))];
  const shown = items.filter(
    (i) =>
      (cat === "all" || i.category === cat) &&
      (!unassigned || !i.key) &&
      (!query || `${i.name} ${i.key ?? ""}`.toLowerCase().includes(query.toLowerCase())),
  );
  const done = items.filter((i) => i.key).length;
  return (
    <div className="kn-tab">
      <div className="kn-toolbar">
        <input
          className="kn-search"
          aria-label="Search types and materials"
          placeholder="Search types and materials"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
        <label className="ob-check">
          <input
            type="checkbox"
            checked={unassigned}
            onChange={(e) => setUnassigned(e.target.checked)}
          />
          Unassigned only
        </label>
        <span className="muted kn-progress">
          {done} of {items.length} keynoted
          <span className="kn-meter">
            <span style={{ width: `${items.length ? (100 * done) / items.length : 0}%` }} />
          </span>
        </span>
      </div>
      <div className="kn-cats" role="tablist" aria-label="Categories">
        {["all", ...cats].map((c) => (
          <button
            key={c}
            role="tab"
            aria-selected={cat === c}
            className={cat === c ? "on" : ""}
            onClick={() => setCat(c)}
          >
            {c === "all" ? "All" : (CATEGORY_LABELS[c] ?? c)}
          </button>
        ))}
      </div>
      {checked.size > 0 && (
        <div className="kn-bulk" role="group" aria-label="Selected">
          {checked.size} selected
          <button className="btn-cyan" onClick={() => setPicking([...checked])}>
            Assign Keynote…
          </button>
          <button
            className="btn-outline"
            onClick={async () => {
              if (await apply(() => ipc.keynoteAssign([...checked], null))) setChecked(new Set());
            }}
          >
            Clear
          </button>
        </div>
      )}
      <div className="kn-scroll">
        <table className="kn-table">
          <thead>
            <tr>
              <th>
                <input
                  type="checkbox"
                  aria-label="Select all shown"
                  checked={shown.length > 0 && shown.every((i) => checked.has(i.id))}
                  onChange={(e) =>
                    setChecked(e.target.checked ? new Set(shown.map((i) => i.id)) : new Set())
                  }
                />
              </th>
              <th>Category</th>
              <th>Type / material</th>
              <th>Keynote</th>
            </tr>
          </thead>
          <tbody>
            {shown.map((i) => (
              <tr key={i.id} className={i.key ? "" : "unset"}>
                <td>
                  <input
                    type="checkbox"
                    aria-label={`Select ${i.name}`}
                    checked={checked.has(i.id)}
                    onChange={(e) => {
                      const n = new Set(checked);
                      if (e.target.checked) n.add(i.id);
                      else n.delete(i.id);
                      setChecked(n);
                    }}
                  />
                </td>
                <td className="muted">{CATEGORY_LABELS[i.category] ?? i.category}</td>
                <td>{i.name}</td>
                <td>
                  <button
                    className="kn-assign"
                    aria-label={`Keynote for ${i.name}`}
                    onClick={() => setPicking([i.id])}
                  >
                    {i.key ? (
                      <>
                        <span className="kn-key">{i.key}</span> {text.get(i.key)}
                      </>
                    ) : (
                      <span className="muted">Assign…</span>
                    )}
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      {picking && (
        <KeynotePicker
          title={
            picking.length === 1
              ? `Keynote for ${items.find((i) => i.id === picking[0])?.name}`
              : `Keynote for ${picking.length} types and materials`
          }
          initial={picking.length === 1 ? items.find((i) => i.id === picking[0])?.key : null}
          onCancel={() => setPicking(null)}
          onPick={async (key) => {
            if (await apply(() => ipc.keynoteAssign(picking, key))) {
              setPicking(null);
              setChecked(new Set());
            }
          }}
        />
      )}
    </div>
  );
}

export function KeynoteManager({ onClose }: { onClose: () => void }) {
  const table = useTable();
  const [tab, setTab] = useState<Tab>("keynotes");
  return (
    <div className="modal-backdrop" role="dialog" aria-label="Keynote Manager">
      <div className="modal kn-manager">
        <header className="kn-head">
          <h2>Keynote Manager</h2>
          <div className="kn-tabs" role="tablist" aria-label="Keynote Manager tabs">
            <button
              role="tab"
              aria-selected={tab === "keynotes"}
              className={tab === "keynotes" ? "on" : ""}
              onClick={() => setTab("keynotes")}
            >
              Keynotes
            </button>
            <button
              role="tab"
              aria-selected={tab === "assign"}
              className={tab === "assign" ? "on" : ""}
              onClick={() => setTab("assign")}
            >
              Assign to Types &amp; Materials
            </button>
          </div>
        </header>
        {!table ? (
          <p className="muted">Loading keynotes…</p>
        ) : tab === "keynotes" ? (
          <KeynotesTab table={table} />
        ) : (
          <AssignTab table={table} />
        )}
        <div className="modal-actions">
          <button className="btn-cyan" onClick={onClose}>
            Done
          </button>
        </div>
      </div>
    </div>
  );
}
