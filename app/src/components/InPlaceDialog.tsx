import { useEffect, useMemo, useState } from "react";
import type { Category } from "../bindings/Category";
import type { CategoryChoice } from "../bindings/CategoryChoice";
import { errorMessage, ipc } from "../ipc";
import { beginInPlace } from "../inplace";
import { useAppStore } from "../store";

// Revit's Family Category and Parameters dialog for Model In-Place (ADR-068): pick what the
// element is (Walls, Doors, Casework…), name it, and the In-Place Editor opens.

export function InPlaceDialog({ onClose }: { onClose: () => void }) {
  const [choices, setChoices] = useState<CategoryChoice[]>([]);
  const [filter, setFilter] = useState("");
  const [category, setCategory] = useState<Category>("GenericModel");
  const [name, setName] = useState("");
  const [named, setNamed] = useState(false);
  useEffect(() => {
    ipc
      .inPlaceCategories()
      .then(setChoices, (e) => useAppStore.getState().setError(errorMessage(e)));
  }, []);
  // The name follows the category ("Casework 1") until it's typed.
  useEffect(() => {
    if (named) return;
    let live = true;
    ipc.inPlaceDefaultName(category).then(
      (n) => live && setName(n),
      () => {},
    );
    return () => {
      live = false;
    };
  }, [category, named]);
  const shown = useMemo(() => {
    const f = filter.trim().toLowerCase();
    return f ? choices.filter((c) => c.label.toLowerCase().includes(f)) : choices;
  }, [choices, filter]);
  const ok = async () => {
    if (await beginInPlace(category, name)) onClose();
  };
  return (
    <div className="modal-backdrop" role="dialog" aria-label="Family Category and Parameters">
      <div className="modal inplace-dialog">
        <h2>Family Category and Parameters</h2>
        <p className="muted">
          Model In-Place: choose what the element is. The category decides how views cut and show
          it, what it schedules and filters with, and its IFC class.
        </p>
        <label className="inplace-filter">
          Filter list
          <input
            aria-label="Filter list"
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            placeholder="Walls, Casework, Furniture…"
          />
        </label>
        <div className="inplace-list" role="listbox" aria-label="Family Category">
          {shown.map((c) => (
            <div
              key={c.category}
              role="option"
              aria-selected={c.category === category}
              className={`inplace-cat${c.category === category ? " on" : ""}`}
              onClick={() => setCategory(c.category)}
              onDoubleClick={() => {
                setCategory(c.category);
                void ok();
              }}
            >
              {c.label}
            </div>
          ))}
          {shown.length === 0 && <p className="muted">No category matches.</p>}
        </div>
        <label className="inplace-name">
          Name
          <input
            aria-label="Name"
            value={name}
            onChange={(e) => {
              setNamed(true);
              setName(e.target.value);
            }}
            onKeyDown={(e) => {
              if (e.key === "Enter") void ok();
            }}
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
