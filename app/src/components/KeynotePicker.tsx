import { useEffect, useMemo, useRef, useState } from "react";
import type { Keynote } from "../bindings/Keynote";
import { ipc } from "../ipc";
import {
  keynoteTree,
  keynoteMatches,
  placeableIn,
  recentKeynotes,
  rememberKeynote,
  visibleKeys,
  type KeynoteNode,
} from "../keynotes";
import { useAppStore } from "../store";

// The keynote picker (ADR-081): Revit's Keynotes dialog, made friendlier. Search by key or
// words, the MasterFormat tree open where it matters, recent picks one click away, and the
// keyboard (↑ ↓ Enter Esc) all the way.

/** The project's keynotes, refetched when the model changes. */
export function useKeynotes(): Keynote[] {
  const revision = useAppStore((s) => s.app?.revision);
  const [entries, setEntries] = useState<Keynote[]>([]);
  useEffect(() => {
    let live = true;
    ipc.keynoteTable().then(
      (t) => live && setEntries(t.entries),
      () => {},
    );
    return () => {
      live = false;
    };
  }, [revision]);
  return entries;
}

export function KeynoteTree({
  entries,
  query,
  selected,
  onSelect,
  onChoose,
  usage,
  only = () => true,
}: {
  entries: Keynote[];
  query: string;
  selected: string | null;
  onSelect: (key: string) => void;
  onChoose?: (key: string) => void;
  usage?: Map<string, number>;
  only?: (k: Keynote) => boolean;
}) {
  const tree = useMemo(() => keynoteTree(entries), [entries]);
  const shown = useMemo(() => visibleKeys(entries, query), [entries, query]);
  const [open, setOpen] = useState<Set<string>>(new Set());
  const row = (n: KeynoteNode, depth: number): React.ReactNode => {
    if (shown && !shown.has(n.k.key)) return null;
    const kids = n.children.length > 0;
    const expanded = !!shown || open.has(n.k.key);
    const pickable = only(n.k);
    const uses = usage?.get(n.k.key) ?? 0;
    return (
      <li key={n.k.key} role="none">
        <div
          role="treeitem"
          aria-selected={selected === n.k.key}
          aria-expanded={kids ? expanded : undefined}
          aria-label={`${n.k.key} ${n.k.text}`}
          className={`kn-row${selected === n.k.key ? " on" : ""}${pickable ? "" : " group"}`}
          style={{ paddingLeft: 8 + depth * 16 }}
          onClick={() => {
            onSelect(n.k.key);
            if (kids && !shown) {
              const next = new Set(open);
              if (next.has(n.k.key)) next.delete(n.k.key);
              else next.add(n.k.key);
              setOpen(next);
            }
          }}
          onDoubleClick={() => pickable && onChoose?.(n.k.key)}
        >
          <span className="kn-caret">{kids ? (expanded ? "▾" : "▸") : ""}</span>
          <span className="kn-key">{n.k.key}</span>
          <span className="kn-text">{n.k.text}</span>
          {uses > 0 && (
            <span className="kn-uses" title={`Used ${uses} time${uses === 1 ? "" : "s"}`}>
              {uses}
            </span>
          )}
        </div>
        {kids && expanded && <ul role="group">{n.children.map((c) => row(c, depth + 1))}</ul>}
      </li>
    );
  };
  return (
    <ul className="kn-tree" role="tree" aria-label="Keynotes">
      {tree.map((n) => row(n, 0))}
      {shown && shown.size === 0 && <li className="muted kn-empty">No keynotes match.</li>}
    </ul>
  );
}

/** Choose a keynote: a modal with search, recent picks and the tree. */
export function KeynotePicker({
  title,
  subtitle,
  initial,
  onPick,
  onCancel,
}: {
  title: string;
  subtitle?: string;
  initial?: string | null;
  onPick: (key: string) => void;
  onCancel: () => void;
}) {
  const entries = useKeynotes();
  const [query, setQuery] = useState("");
  const [sel, setSel] = useState<string | null>(initial ?? null);
  const input = useRef<HTMLInputElement>(null);
  const shown = useMemo(() => visibleKeys(entries, query), [entries, query]);
  const placeable = useMemo(() => placeableIn(entries), [entries]);
  // Keyboard: the placeable keynotes in view, in order.
  const flat = useMemo(
    () => entries.filter((k) => placeable(k) && (!shown || shown.has(k.key))),
    [entries, shown, placeable],
  );
  const chosen = entries.find((k) => k.key === sel) ?? null;
  const pick = (key: string) => {
    rememberKeynote(key);
    onPick(key);
  };
  const recent = recentKeynotes().filter((k) => entries.some((e) => e.key === k));
  return (
    <div className="modal-backdrop" role="dialog" aria-label={title}>
      <div
        className="modal kn-picker"
        onKeyDown={(e) => {
          if (e.key === "Escape") onCancel();
          if (e.key === "ArrowDown" || e.key === "ArrowUp") {
            e.preventDefault();
            const i = flat.findIndex((k) => k.key === sel);
            const next =
              flat[Math.max(0, Math.min(flat.length - 1, i + (e.key === "ArrowDown" ? 1 : -1)))];
            if (next) setSel(next.key);
          }
          if (e.key === "Enter" && chosen && placeable(chosen)) pick(chosen.key);
        }}
      >
        <h2>{title}</h2>
        {subtitle && <p className="muted">{subtitle}</p>}
        <input
          ref={input}
          autoFocus
          className="kn-search"
          aria-label="Search keynotes"
          placeholder="Search by key or words, e.g. gyp 5/8"
          value={query}
          onChange={(e) => {
            setQuery(e.target.value);
            const first = entries.find((k) => placeable(k) && keynoteMatches(k, e.target.value));
            if (first) setSel(first.key);
          }}
        />
        {recent.length > 0 && !query && (
          <div className="kn-recent" aria-label="Recent keynotes">
            {recent.map((k) => (
              <button
                key={k}
                className="kn-chip"
                onClick={() => pick(k)}
                title={entries.find((e) => e.key === k)?.text}
              >
                {k}
              </button>
            ))}
          </div>
        )}
        <KeynoteTree
          entries={entries}
          query={query}
          selected={sel}
          onSelect={setSel}
          onChoose={pick}
          only={placeable}
        />
        <div className="kn-chosen" aria-live="polite">
          {chosen ? (
            <>
              <span className="kn-key">{chosen.key}</span> {chosen.text}
            </>
          ) : (
            <span className="muted">Pick a keynote (double-click or Enter)</span>
          )}
        </div>
        <div className="modal-actions">
          <button className="btn-outline" onClick={onCancel}>
            Cancel
          </button>
          <button
            className="btn-cyan"
            disabled={!chosen || !placeable(chosen)}
            onClick={() => chosen && pick(chosen.key)}
          >
            Use Keynote
          </button>
        </div>
      </div>
    </div>
  );
}
