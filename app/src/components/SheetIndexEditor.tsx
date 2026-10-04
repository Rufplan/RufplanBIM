import { useState } from "react";
import type { IndexRow } from "../bindings/IndexRow";
import { ipc } from "../ipc";
import { insertRow, moveRow, removable, rowKind } from "../sheetIndex";
import { ContextMenu, type MenuItem } from "./ContextMenu";

// The sheet index's rows, edited in its dialog (ADR-113): type a number or name to change
// the sheet; right-click a row (or the list) to add a sheet or placeholder, move or remove
// it; drag rows by their handle to order them.

export function SheetIndexEditor({
  rows,
  onChange,
}: {
  rows: IndexRow[];
  onChange: (rows: IndexRow[]) => void;
}) {
  const [selected, setSelected] = useState<number | null>(null);
  const [menu, setMenu] = useState<{ x: number; y: number; row: number | null } | null>(null);
  const [dragging, setDragging] = useState<number | null>(null);

  const set = (i: number, patch: Partial<IndexRow>) =>
    onChange(rows.map((r, k) => (k === i ? { ...r, ...patch } : r)));
  const add = async (at: number, placeholder: boolean) => {
    const taken = rows.map((r) => r.number);
    const after = rows[at - 1]?.number ?? rows[rows.length - 1]?.number ?? "A-100";
    const number = await ipc.nextIndexNumber(after, taken).catch(() => "");
    onChange(insertRow(rows, at, { sheet: null, number, name: "", placeholder }));
    setSelected(at);
  };
  const move = (from: number, to: number) => {
    onChange(moveRow(rows, from, to));
    setSelected(Math.max(0, Math.min(to, rows.length - 1)));
  };
  const remove = (i: number) => {
    onChange(rows.filter((_, k) => k !== i));
    setSelected(null);
  };
  const below = () => (selected === null ? rows.length : selected + 1);

  const items = (i: number | null): MenuItem[] => {
    if (i === null)
      return [
        { label: "Add Sheet", onClick: () => void add(rows.length, false) },
        { label: "Add Placeholder Sheet", onClick: () => void add(rows.length, true) },
      ];
    const r = rows[i]!;
    const last = rows.length - 1;
    return [
      { label: "Add Sheet Above", onClick: () => void add(i, false) },
      { label: "Add Sheet Below", onClick: () => void add(i + 1, false) },
      { label: "Add Placeholder Sheet Below", onClick: () => void add(i + 1, true) },
      { label: "Move Up", disabled: i === 0, onClick: () => move(i, i - 1), separator: true },
      { label: "Move Down", disabled: i === last, onClick: () => move(i, i + 1) },
      { label: "Move to Top", disabled: i === 0, onClick: () => move(i, 0) },
      { label: "Move to Bottom", disabled: i === last, onClick: () => move(i, last) },
      ...(r.sheet === null
        ? [
            {
              label: r.placeholder ? "Make a Sheet" : "Make a Placeholder",
              onClick: () => set(i, { placeholder: !r.placeholder }),
              separator: true,
            },
          ]
        : []),
      {
        label: "Remove from Index",
        disabled: !removable(r),
        onClick: () => remove(i),
        separator: r.sheet !== null,
      },
    ];
  };

  return (
    <div className="sheet-index-editor">
      <div className="row sie-bar">
        <h3 className="grow">Sheets</h3>
        <button className="btn-outline" onClick={() => void add(below(), false)}>
          Add Sheet
        </button>
        <button className="btn-outline" onClick={() => void add(below(), true)}>
          Add Placeholder
        </button>
        <button
          className="btn-ghost"
          aria-label="Move up"
          disabled={selected === null || selected === 0}
          onClick={() => selected !== null && move(selected, selected - 1)}
        >
          ▲
        </button>
        <button
          className="btn-ghost"
          aria-label="Move down"
          disabled={selected === null || selected === rows.length - 1}
          onClick={() => selected !== null && move(selected, selected + 1)}
        >
          ▼
        </button>
      </div>
      <p className="muted">
        Right-click to add, move or remove; drag ⠿ to reorder. New sheets are made when you click
        OK; placeholders only list here.
      </p>
      <div
        className="sie-list"
        role="list"
        aria-label="Sheet index rows"
        onContextMenu={(e) => {
          e.preventDefault();
          setMenu({ x: e.clientX, y: e.clientY, row: null });
        }}
      >
        {rows.map((r, i) => (
          <div
            key={`${r.sheet ?? "new"}-${i}`}
            role="listitem"
            aria-label={`${r.number} ${r.name}`}
            className={`sie-row${selected === i ? " on" : ""}${dragging === i ? " dragging" : ""}`}
            onClick={() => setSelected(i)}
            onContextMenu={(e) => {
              e.preventDefault();
              e.stopPropagation();
              setSelected(i);
              setMenu({ x: e.clientX, y: e.clientY, row: i });
            }}
            onDragOver={(e) => {
              if (dragging !== null) e.preventDefault();
            }}
            onDrop={(e) => {
              e.preventDefault();
              if (dragging !== null && dragging !== i) move(dragging, i);
              setDragging(null);
            }}
          >
            <span
              className="sie-handle"
              draggable
              title="Drag to reorder"
              onDragStart={(e) => {
                setDragging(i);
                e.dataTransfer.effectAllowed = "move";
              }}
              onDragEnd={() => setDragging(null)}
            >
              ⠿
            </span>
            <input
              className="sie-number"
              aria-label={`Row ${i + 1} number`}
              value={r.number}
              onChange={(e) => set(i, { number: e.target.value })}
            />
            <input
              className="grow"
              aria-label={`Row ${i + 1} name`}
              value={r.name}
              placeholder="Sheet name"
              onChange={(e) => set(i, { name: e.target.value })}
            />
            <span className={`sie-kind${r.placeholder ? " ph" : r.sheet === null ? " new" : ""}`}>
              {rowKind(r)}
            </span>
          </div>
        ))}
      </div>
      {menu && (
        <ContextMenu
          x={menu.x}
          y={menu.y}
          label="Sheet index"
          items={items(menu.row)}
          onClose={() => setMenu(null)}
        />
      )}
    </div>
  );
}
