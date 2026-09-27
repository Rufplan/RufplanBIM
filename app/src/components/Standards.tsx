import { useEffect, useRef, useState } from "react";
import type { StandardCategory } from "../bindings/StandardCategory";
import type { StandardItem } from "../bindings/StandardItem";
import type { Standards } from "../bindings/Standards";
import { apply } from "../fileActions";
import { ipc } from "../ipc";
import { useAppStore } from "../store";

// The Standards tab (ADR-047): the office's drawing-set standards by category, each defined
// or open, so the tab is also a checklist of how complete the set is. Kept in the project
// (studio-core `standards`); this module only shows and edits them.

export const STANDARD_GROUPS = ["SHEETS", "ANNOTATION", "GRAPHICS", "DATA", "VIEWS", "OUTPUT"];

/** Ribbon icons per category (simple line strokes, as the app's icons). */
const ICONS: Record<string, string> = {
  sheet: "M4 3h16v18H4zM4 16h16M14 16v5",
  sym: "M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18zM3 12h18",
  tags: "M3 12l7-7h11v14H10zM14 12h.01",
  text: "M5 5h14M12 5v14M9 19h6",
  line: "M3 7h18M3 12h18M3 17h18",
  mat: "M4 4h16v16H4zM4 12l8-8M4 20L20 4M12 20l8-8",
  phase: "M4 20V8l8-4 8 4v12M9 20v-6h6v6",
  dim: "M3 12h18M3 8v8M21 8v8M7 10l-2 4M19 10l-2 4",
  num: "M9 4L7 20M17 4l-2 16M4 9h16M3 15h16",
  sched: "M3 4h18v16H3zM3 9h18M3 14h18M9 4v16",
  key: "M6 3h9l4 4v14H6zM9 12h6M9 16h6",
  scale: "M3 17l14-14 4 4L7 21H3zM7 13l2 2M11 9l2 2",
  bim: "M12 3l8 4.5v9L12 21l-8-4.5v-9zM12 12l8-4.5M12 12v9M12 12L4 7.5",
  issue: "M4 12l16-8-6 16-3-6zM11 14l9-10",
};

/** The project's standards, fetched with the model; values being typed show at once. */
export function useStandards(): Standards | null {
  const revision = useAppStore((s) => s.app?.revision ?? 0);
  const standards = useAppStore((s) => s.standards);
  const setStandards = useAppStore((s) => s.setStandards);
  useEffect(() => {
    let live = true;
    ipc.standardsGet().then(
      (s) => live && setStandards(s),
      () => {},
    );
    return () => {
      live = false;
    };
  }, [revision, setStandards]);
  return standards;
}

const defined = (c: StandardCategory) => c.items.filter((i) => i.done).length;

export function totals(s: Standards | null): { done: number; all: number; pct: number } {
  const items = s?.categories.flatMap((c) => c.items) ?? [];
  const done = items.filter((i) => i.done).length;
  return {
    done,
    all: items.length,
    pct: items.length ? Math.round((done / items.length) * 100) : 0,
  };
}

function useCategory(s: Standards | null): StandardCategory | null {
  const id = useAppStore((st) => st.standardsUi.category);
  return s?.categories.find((c) => c.id === id) ?? s?.categories[0] ?? null;
}

function StrokeIcon({ d }: { d: string }) {
  return (
    <svg
      width="18"
      height="18"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.6"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden
    >
      <path d={d} />
    </svg>
  );
}

/** The Standards ribbon: a button per category in its group, and the office library. */
export function StandardsRibbon() {
  const s = useStandards();
  const cat = useCategory(s);
  const pick = useAppStore((st) => st.setStandardsUi);
  const [libraries, setLibraries] = useState<string[]>([]);
  useEffect(() => {
    ipc.standardsLibraries().then(setLibraries, () => {});
  }, []);
  return (
    <>
      {STANDARD_GROUPS.map((g) => (
        <div className="rb-group" key={g}>
          <div className="rb-items">
            {s?.categories
              .filter((c) => c.group === g)
              .map((c) => (
                <button
                  key={c.id}
                  className={`rb-btn std-btn${c.short.length > 8 ? " wide" : ""}${c.id === cat?.id ? " active" : ""}`}
                  title={`${c.label}: ${defined(c)} of ${c.items.length} defined`}
                  onClick={() => pick({ category: c.id, item: 0 })}
                >
                  <StrokeIcon d={ICONS[c.id] ?? ICONS.sheet!} />
                  <span>{c.short}</span>
                </button>
              ))}
          </div>
          <div className="rb-title">{g}</div>
        </div>
      ))}
      <div className="rb-group">
        <div className="rb-items std-library">
          <label>
            <span>Office Standard</span>
            <select
              aria-label="Office Standard"
              value={s?.library ?? ""}
              onChange={(e) => void apply(() => ipc.standardsLoadLibrary(e.target.value))}
            >
              {libraries.map((l) => (
                <option key={l} value={l}>
                  {l}
                </option>
              ))}
            </select>
          </label>
        </div>
        <div className="rb-title">LIBRARY</div>
      </div>
    </>
  );
}

/** The Standards Browser: the categories with their counts, and how complete the set is. */
export function StandardsBrowser() {
  const s = useStandards();
  const cat = useCategory(s);
  const pick = useAppStore((st) => st.setStandardsUi);
  const t = totals(s);
  return (
    <aside className="panel browser std-browser" aria-label="Standards Browser">
      <div className="panel-title">Standards Browser</div>
      <div className="std-tree">
        <div className="std-root">
          <span aria-hidden>▼</span>DRAWING SET
        </div>
        {s?.categories.map((c) => {
          const d = defined(c);
          return (
            <button
              key={c.id}
              className={`std-row${c.id === cat?.id ? " on" : ""}`}
              onClick={() => pick({ category: c.id, item: 0 })}
            >
              <span>{c.label}</span>
              <span className={`std-count${d === c.items.length ? " full" : ""}`}>
                {d}/{c.items.length}
              </span>
            </button>
          );
        })}
      </div>
      <div className="std-foot">
        <div className="std-foot-head">
          <span>SET COMPLETE</span>
          <span>{t.pct}%</span>
        </div>
        <div className="std-bar">
          <div style={{ width: `${t.pct}%` }} />
        </div>
        <div className="std-foot-line">
          {t.done} of {t.all} standards defined
        </div>
      </div>
    </aside>
  );
}

/** The checklist of the category picked: each standard, its office value and status. */
export function StandardsView() {
  const s = useStandards();
  const cat = useCategory(s);
  const ui = useAppStore((st) => st.standardsUi);
  const setUi = useAppStore((st) => st.setStandardsUi);
  const pending = useAppStore((st) => st.standardsPending);
  if (!s || !cat) return <section className="workspace std-workspace" />;
  const index = s.categories.indexOf(cat);
  const d = defined(cat);
  const pct = Math.round((d / cat.items.length) * 100);
  const rows = cat.items
    .map((it, i) => ({ it, i }))
    .filter(({ it }) => ui.filter === "all" || !it.done);
  const toggle = (i: number, it: StandardItem) => {
    setUi({ item: i });
    void apply(() => ipc.standardsSet(cat.id, i, null, !it.done));
  };
  return (
    <section className="workspace std-workspace">
      <div className="tabs" role="tablist" aria-label="Open views">
        <div className="tab active std-tab" role="tab" aria-selected>
          <span className="tab-label">
            <span className="tab-kind">STANDARDS</span>
            {cat.label}
          </span>
        </div>
      </div>
      <div className="std-canvas">
        <div className="std-card">
          <div className="std-card-head">
            <div className="std-title">
              <span className="std-eyebrow">
                {String(index + 1).padStart(2, "0")} · {cat.group}
              </span>
              <h2>{cat.label}</h2>
            </div>
            <div className="std-head-right">
              <div className="std-progress">
                <span>
                  {d}/{cat.items.length} defined
                </span>
                <div className="std-bar">
                  <div style={{ width: `${pct}%` }} />
                </div>
              </div>
              <div className="std-filter" role="radiogroup" aria-label="Show">
                {(["all", "open"] as const).map((f) => (
                  <button
                    key={f}
                    role="radio"
                    aria-checked={ui.filter === f}
                    className={ui.filter === f ? "on" : undefined}
                    onClick={() => setUi({ filter: f })}
                  >
                    {f === "all" ? "ALL" : "UNDEFINED"}
                  </button>
                ))}
              </div>
            </div>
          </div>
          <div className="std-grid std-cols">
            <span />
            <span>STANDARD</span>
            <span>OFFICE VALUE</span>
            <span className="right">STATUS</span>
          </div>
          {rows.map(({ it, i }) => {
            const value = pending[`${cat.id}:${i}`] ?? it.value;
            return (
              <div
                key={i}
                className={`std-grid std-item${i === ui.item ? " on" : ""}`}
                onClick={() => setUi({ item: i })}
              >
                <button
                  className={`std-check${it.done ? " on" : ""}`}
                  role="checkbox"
                  aria-checked={it.done}
                  aria-label={`${it.name} defined`}
                  onClick={(e) => {
                    e.stopPropagation();
                    toggle(i, it);
                  }}
                >
                  {it.done ? "✓" : ""}
                </button>
                <span className="std-name">{it.name}</span>
                <span className={`std-value${value ? "" : " unset"}`}>
                  {value || "Not yet set"}
                </span>
                <span className={`std-status${it.done ? " done" : ""}`}>
                  {it.done ? "DEFINED" : "OPEN"}
                </span>
              </div>
            );
          })}
          {rows.length === 0 && (
            <div className="std-empty">Every standard in this category is defined.</div>
          )}
        </div>
      </div>
    </section>
  );
}

/** Properties of the standard picked: its value (typed, or chosen from its presets) and
 * status. A value is saved as it's typed (after a pause) and a non-empty one defines it. */
export function StandardsProperties() {
  const s = useStandards();
  const cat = useCategory(s);
  const ui = useAppStore((st) => st.standardsUi);
  const setPending = useAppStore((st) => st.setStandardsPending);
  const item = cat?.items[ui.item] ?? cat?.items[0] ?? null;
  const index = cat && item ? cat.items.indexOf(item) : -1;
  const key = cat ? `${cat.id}:${index}` : "";
  // The value being typed, for the standard it was typed in.
  const [draft, setDraftFor] = useState<{ key: string; v: string } | null>(null);
  const timer = useRef(0);
  if (!cat || !item) return <aside className="panel properties" aria-label="Properties" />;
  const setDraft = (v: string) => setDraftFor({ key, v });
  const value = draft?.key === key ? draft.v : item.value;
  const options = item.options ?? [];
  const commit = (v: string) => {
    window.clearTimeout(timer.current);
    setPending(key, null);
    if (v !== item.value) void apply(() => ipc.standardsSet(cat.id, index, v, null));
  };
  const type = (v: string) => {
    setDraft(v);
    setPending(key, v);
    window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => commit(v), 600);
  };
  return (
    <aside className="panel properties std-props" aria-label="Properties">
      <div className="panel-title">Properties</div>
      <div className="std-props-body">
        <div className="std-props-title">
          <span>STANDARD</span>
          <strong>{item.name}</strong>
        </div>
        <div className="std-section">IDENTITY DATA</div>
        <div className="std-prop">
          <span>Category</span>
          <span>{cat.label}</span>
        </div>
        <div className="std-prop">
          <span>Group</span>
          <span className="muted">{cat.group}</span>
        </div>
        <div className="std-section">VALUE</div>
        <div className="std-value-edit">
          {options.length > 0 && (
            <select
              aria-label="Preset"
              value={options.includes(value) ? value : ""}
              onChange={(e) => {
                setDraft(e.target.value);
                commit(e.target.value);
              }}
            >
              {!options.includes(value) && <option value="">Custom…</option>}
              {options.map((o) => (
                <option key={o} value={o}>
                  {o}
                </option>
              ))}
            </select>
          )}
          <textarea
            aria-label="Office value"
            rows={3}
            placeholder="Enter office standard…"
            value={value}
            onChange={(e) => type(e.target.value)}
            onBlur={(e) => commit(e.target.value)}
          />
        </div>
        <div className="std-prop">
          <span>Status</span>
          <select
            aria-label="Status"
            value={item.done ? "yes" : "no"}
            onChange={(e) =>
              void apply(() => ipc.standardsSet(cat.id, index, null, e.target.value === "yes"))
            }
          >
            <option value="yes">Defined</option>
            <option value="no">Undefined</option>
          </select>
        </div>
        <div className="std-section">APPLIES TO</div>
        <div className="std-applies">
          {cat.applies.map((a) => (
            <span key={a}>{a}</span>
          ))}
        </div>
      </div>
    </aside>
  );
}

/** The status bar on the Standards tab. */
export function StandardsStatus() {
  const s = useAppStore((st) => st.standards);
  const t = totals(s);
  return (
    <footer className="statusbar">
      <span className="status-prompt">
        Pick a category in the ribbon or browser. Click a standard to edit it in Properties; tick
        the box to mark it defined.
      </span>
      <span className="status-cursor std-status-total">
        {t.done} of {t.all} standards defined
      </span>
    </footer>
  );
}
