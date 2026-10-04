import type { IndexRow } from "./bindings/IndexRow";

// The sheet index's rows being edited (ADR-113): sheets, new sheets (no id yet) and
// placeholders, in the order the index lists them.

/** `rows` with `row` inserted at `at`. */
export function insertRow(rows: IndexRow[], at: number, row: IndexRow): IndexRow[] {
  const i = Math.max(0, Math.min(at, rows.length));
  return [...rows.slice(0, i), row, ...rows.slice(i)];
}

/** `rows` with the row at `from` moved to `to` (clamped). */
export function moveRow(rows: IndexRow[], from: number, to: number): IndexRow[] {
  if (from < 0 || from >= rows.length) return rows;
  const out = rows.slice();
  const [r] = out.splice(from, 1);
  out.splice(Math.max(0, Math.min(to, out.length)), 0, r!);
  return out;
}

/** Rows the index can drop: new sheets and placeholders (a sheet is deleted in the
 * project browser). */
export function removable(r: IndexRow): boolean {
  return r.sheet === null || r.placeholder;
}

/** What a row is, for its badge. */
export function rowKind(r: IndexRow): string {
  if (r.placeholder) return "Placeholder";
  return r.sheet === null ? "New sheet" : "Sheet";
}
