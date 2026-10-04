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

/** Rows the index can drop: new sheets and placeholders, and, when the list is a phase's
 * set (ADR-116), a sheet, which leaves that set (sheets are deleted in the project browser). */
export function removable(r: IndexRow, phased = false): boolean {
  return r.sheet === null || r.placeholder || phased;
}

/** What a row is, for its badge. */
export function rowKind(r: IndexRow): string {
  if (r.placeholder) return "Placeholder";
  return r.sheet === null ? "New sheet" : "Sheet";
}

/** What's wrong with the rows before OK: a row without a number, or a number listed twice. */
export function indexProblems(rows: IndexRow[]): { row: number; message: string }[] {
  const out: { row: number; message: string }[] = [];
  const seen = new Map<string, number>();
  rows.forEach((r, i) => {
    const n = r.number.trim();
    if (!n) out.push({ row: i, message: `Row ${i + 1} needs a sheet number` });
    else if (seen.has(n)) out.push({ row: i, message: `Sheet ${n} is listed twice` });
    else seen.set(n, i);
  });
  return out;
}

/** The gap (0 = before the first row … n = after the last) a dragged row at `y` drops
 * into, given each row's vertical middle, top down. */
export function dropGap(mids: number[], y: number): number {
  const i = mids.findIndex((m) => y < m);
  return i < 0 ? mids.length : i;
}

/** Where the row at `from` ends up when dropped into gap `gap`. */
export function dropTo(from: number, gap: number): number {
  return gap > from ? gap - 1 : gap;
}
