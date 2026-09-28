import { useEffect, useMemo, useState } from "react";
import type { Category } from "../bindings/Category";
import type { ElementId } from "../bindings/ElementId";
import { errorMessage, ipc } from "../ipc";
import { useAppStore } from "../store";
import { categoryLabel } from "./ModifyContext";

// Revit's Filter dialog: the selection's categories with how many of each, to keep only
// the ones checked (Check All, Check None, and the total).

export function FilterDialog({ onClose }: { onClose: () => void }) {
  const selection = useAppStore((s) => s.selection);
  const [cats, setCats] = useState<[ElementId, Category][] | null>(null);
  const [off, setOff] = useState<Set<Category>>(new Set());
  useEffect(() => {
    let live = true;
    ipc.elementCategories(selection).then(
      (c) => live && setCats(c),
      (e) => useAppStore.getState().setError(errorMessage(e)),
    );
    return () => {
      live = false;
    };
  }, [selection]);
  const rows = useMemo(() => {
    const m = new Map<Category, number>();
    for (const [, c] of cats ?? []) m.set(c, (m.get(c) ?? 0) + 1);
    return [...m.entries()].sort((a, b) => categoryLabel(a[0]).localeCompare(categoryLabel(b[0])));
  }, [cats]);
  const total = rows.filter(([c]) => !off.has(c)).reduce((n, [, k]) => n + k, 0);
  const toggle = (c: Category) =>
    setOff((s) => {
      const n = new Set(s);
      if (n.has(c)) n.delete(c);
      else n.add(c);
      return n;
    });
  const ok = () => {
    const keep = (cats ?? []).filter(([, c]) => !off.has(c)).map(([id]) => id);
    useAppStore.getState().select(keep);
    onClose();
  };
  return (
    <div className="modal-backdrop" role="dialog" aria-label="Filter">
      <div className="modal filter-dialog">
        <h2>Filter</h2>
        <div className="filter-head">
          <span>Category</span>
          <span>Count</span>
        </div>
        <div className="filter-list" role="list" aria-label="Categories">
          {rows.map(([c, n]) => (
            <label key={c} className="filter-row" role="listitem">
              <input type="checkbox" checked={!off.has(c)} onChange={() => toggle(c)} />
              <span className="filter-cat">{categoryLabel(c)}</span>
              <span className="filter-count">{n}</span>
            </label>
          ))}
          {cats && rows.length === 0 && <p className="muted">Nothing is selected.</p>}
        </div>
        <div className="filter-foot">
          <div className="filter-checks">
            <button className="btn-outline" onClick={() => setOff(new Set())}>
              Check All
            </button>
            <button className="btn-outline" onClick={() => setOff(new Set(rows.map(([c]) => c)))}>
              Check None
            </button>
          </div>
          <span className="muted" aria-label="Total selected items">
            Total selected items: {total}
          </span>
        </div>
        <div className="mb-actions filter-actions">
          <button className="btn-cyan" onClick={ok}>
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
