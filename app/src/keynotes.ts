// Keynotes (ADR-081): the table as a tree for the picker and manager, and search. The
// table itself is Rust's.
import type { Keynote } from "./bindings/Keynote";

export interface KeynoteNode {
  k: Keynote;
  children: KeynoteNode[];
}

/** The table as a tree, in key order (the table comes sorted). */
export function keynoteTree(entries: Keynote[]): KeynoteNode[] {
  const nodes = new Map<string, KeynoteNode>();
  for (const k of entries) nodes.set(k.key, { k, children: [] });
  const roots: KeynoteNode[] = [];
  for (const k of entries) {
    const n = nodes.get(k.key)!;
    const parent = k.parent ? nodes.get(k.parent) : undefined;
    if (parent) parent.children.push(n);
    else roots.push(n);
  }
  return roots;
}

/** Whether a keynote matches every word of `q` (key or text). */
export function keynoteMatches(k: Keynote, q: string): boolean {
  const hay = `${k.key} ${k.text}`.toLowerCase();
  return q
    .toLowerCase()
    .split(/\s+/)
    .filter(Boolean)
    .every((w) => hay.includes(w));
}

/** The keys to show for a search: matches and their ancestors. */
export function visibleKeys(entries: Keynote[], q: string): Set<string> | null {
  if (!q.trim()) return null;
  const byKey = new Map(entries.map((k) => [k.key, k]));
  const out = new Set<string>();
  for (const k of entries) {
    if (!keynoteMatches(k, q)) continue;
    let cur: Keynote | undefined = k;
    while (cur && !out.has(cur.key)) {
      out.add(cur.key);
      cur = cur.parent ? byKey.get(cur.parent) : undefined;
    }
  }
  return out;
}

/** Keynotes that can be placed: the leaves (divisions and sections group them). */
export function placeableIn(entries: Keynote[]): (k: Keynote) => boolean {
  const parents = new Set(entries.map((k) => k.parent).filter(Boolean));
  return (k) => !parents.has(k.key);
}

// The keynotes picked most recently, for one-click reuse (this session).
const recent: string[] = [];
export function rememberKeynote(key: string) {
  const i = recent.indexOf(key);
  if (i >= 0) recent.splice(i, 1);
  recent.unshift(key);
  recent.length = Math.min(recent.length, 6);
}
export const recentKeynotes = () => [...recent];
