import { useState, type FormEvent } from "react";
import { apply } from "../fileActions";
import { ipc, type Category } from "../ipc";
import { useAppStore } from "../store";

// Revit's Project Parameters: user-defined fields on chosen categories (ADR-017).

const KINDS = ["Text", "Length", "Area", "Angle", "Number", "Integer", "Yes/No"];

const CATEGORIES: [Category, string][] = [
  ["Wall", "Walls"],
  ["Door", "Doors"],
  ["Window", "Windows"],
  ["Room", "Rooms"],
  ["Floor", "Floors"],
  ["Ceiling", "Ceilings"],
  ["Roof", "Roofs"],
  ["Stair", "Stairs"],
  ["Sheet", "Sheets"],
];

const KIND_LABEL: Record<string, string> = { Bool: "Yes/No" };

export function ParamsDialog() {
  const open = useAppStore((s) => s.paramsOpen);
  const stored = useAppStore((s) => s.app?.paramDefs);
  const defs = stored ?? [];
  const setOpen = useAppStore((s) => s.setParamsOpen);
  const [label, setLabel] = useState("");
  const [kind, setKind] = useState("Text");
  const [typeScope, setTypeScope] = useState(false);
  const [cats, setCats] = useState<Category[]>(["Door"]);
  if (!open) return null;

  const add = async (e: FormEvent) => {
    e.preventDefault();
    if (await apply(() => ipc.addProjectParameter(label, kind, typeScope, cats))) setLabel("");
  };
  const toggle = (c: Category) =>
    setCats((cur) => (cur.includes(c) ? cur.filter((x) => x !== c) : [...cur, c]));
  // Types exist for these categories only.
  const typeable: Category[] = ["Wall", "Door", "Window", "Floor", "Ceiling", "Roof"];

  return (
    <div
      className="modal-backdrop"
      role="dialog"
      aria-modal
      aria-label="Project Parameters"
      onKeyDown={(e) => e.key === "Escape" && setOpen(false)}
    >
      <div className="modal rp-modal params-modal">
        <div className="rp-head">
          <div className="modal-kicker">Project Parameters</div>
          <button className="rp-close" aria-label="Close" onClick={() => setOpen(false)}>
            ×
          </button>
        </div>
        {defs.length === 0 ? (
          <p className="rp-note">
            No project parameters yet. They add your own fields (fire rating, finish, cost code…) to
            elements, shown under “Other” in Properties.
          </p>
        ) : (
          <ul className="param-list">
            {defs.map((d) => (
              <li key={d.key}>
                <span className="param-name">{d.label}</span>
                <span className="rp-sub">
                  {KIND_LABEL[d.kind] ?? d.kind} · {d.scope} · {d.categories.join(", ")}
                </span>
                <button
                  className="btn-ghost"
                  aria-label={`Remove ${d.label}`}
                  onClick={() => void apply(() => ipc.removeProjectParameter(d.key))}
                >
                  Remove
                </button>
              </li>
            ))}
          </ul>
        )}
        <form className="rp-form" onSubmit={(e) => void add(e)}>
          <label className="rp-field">
            <span>Name</span>
            <input value={label} onChange={(e) => setLabel(e.target.value)} autoFocus />
          </label>
          <label className="rp-field">
            <span>Type of parameter</span>
            <select value={kind} onChange={(e) => setKind(e.target.value)}>
              {KINDS.map((k) => (
                <option key={k}>{k}</option>
              ))}
            </select>
          </label>
          <div className="rp-field" role="radiogroup" aria-label="Instance or type">
            <span>Belongs to</span>
            <div className="param-scope">
              <label>
                <input type="radio" checked={!typeScope} onChange={() => setTypeScope(false)} />{" "}
                Each instance
              </label>
              <label>
                <input type="radio" checked={typeScope} onChange={() => setTypeScope(true)} /> The
                type
              </label>
            </div>
          </div>
          <fieldset className="param-cats">
            <legend>Categories</legend>
            {CATEGORIES.filter(([c]) => !typeScope || typeable.includes(c)).map(([c, name]) => (
              <label key={c}>
                <input type="checkbox" checked={cats.includes(c)} onChange={() => toggle(c)} />{" "}
                {name}
              </label>
            ))}
          </fieldset>
          <div className="modal-actions">
            <button
              className="btn-cyan"
              type="submit"
              disabled={!label.trim() || cats.length === 0}
            >
              Add Parameter
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
