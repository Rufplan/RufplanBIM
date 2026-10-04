import { useEffect, useRef, useState } from "react";
import type { IndexIssue } from "../bindings/IndexIssue";
import type { IndexRow } from "../bindings/IndexRow";
import type { BuildingType } from "../bindings/BuildingType";
import type { ScheduleStyle } from "../bindings/ScheduleStyle";
import type { TextFont } from "../bindings/TextFont";
import { apply } from "../fileActions";
import { errorMessage, ipc } from "../ipc";
import {
  dropGap,
  dropTo,
  indexProblems,
  insertRow,
  moveRow,
  removable,
  rowKind,
} from "../sheetIndex";
import { useAppStore } from "../store";
import { ContextMenu, type MenuItem } from "./ContextMenu";

// The Sheet Index dialog (ADR-114), to the owner's design handoff: the sheets on the left
// (type a number or name, drag ⋮⋮ to order, right-click to add, move or remove), the
// schedule's type on the right with a 1:1 preview, OK to make the new sheets and save.

const IN = 25.4;
/** Preview scale: 1" on paper = 128 px. */
const PX_PER_IN = 128;
const px = (mm: number) => `${((mm / IN) * PX_PER_IN).toFixed(2)}px`;

/** The heights each control offers, inches → label. Text is never under 3/32" (ADR-109). */
const TITLE = [1 / 8, 5 / 32, 3 / 16, 1 / 4];
const HEADERS = [3 / 32, 1 / 8, 5 / 32];
const BODY = [3 / 32, 1 / 8, 5 / 32, 3 / 16];
const FRACTION: Record<string, string> = {
  "0.09375": `3/32"`,
  "0.125": `1/8"`,
  "0.15625": `5/32"`,
  "0.1875": `3/16"`,
  "0.25": `1/4"`,
};
const label = (inches: number) => FRACTION[String(inches)] ?? `${inches}"`;

const FAMILY: Record<TextFont, string> = {
  Drafting: `"Barlow Condensed", "Barlow", sans-serif`,
  Sans: `Carlito, Calibri, Arial, sans-serif`,
  Serif: `Tinos, "Times New Roman", serif`,
};

/** The design's fixed canvas, fitted to the window as it asks. */
const W = 1240;
const H = 860;
function fit(): number {
  if (typeof window === "undefined" || !window.innerWidth) return 1;
  const s = Math.min((window.innerWidth - 32) / W, (window.innerHeight - 32) / H);
  return Math.max(0.4, Math.min(1.6, s));
}

/** The offered height nearest `mm`, in inches. */
function nearest(mm: number, opts: number[]): number {
  return opts.reduce((a, b) => (Math.abs(b * IN - mm) < Math.abs(a * IN - mm) ? b : a));
}

/** A phase's sheet list as edited: `had` is the sheets in its set when it was loaded. */
interface PhaseList {
  rows: IndexRow[];
  dirty: boolean;
  had: string[];
}

const sheetsOf = (rows: IndexRow[]) => rows.flatMap((r) => (r.sheet ? [r.sheet] : []));

/** The issue to open on: the current stage's (CD at 100%), else the first. */
function defaultIssue(issues: IndexIssue[], current: string | null): string {
  const mine = issues.filter((i) => i.stage === current);
  return (mine.find((i) => i.label.startsWith("100%")) ?? mine[0] ?? issues[0])?.key ?? "";
}

/** The building type the Sheet Sets dialog last used (its typical sheets follow it). */
function buildingType(): BuildingType {
  try {
    return (
      (localStorage.getItem("rufplan.sheetSets.type") as BuildingType | null) ?? "SingleFamily"
    );
  } catch {
    return "SingleFamily";
  }
}

export function SheetIndexDialog({
  view,
  title,
  onClose,
}: {
  view: string;
  title: string;
  onClose: () => void;
}) {
  const app = useAppStore((s) => s.app);
  const activeView = useAppStore((s) => s.activeView);
  // The phases' issues (ADR-116), the one shown, and each phase's list as edited.
  const [issues, setIssues] = useState<IndexIssue[]>([]);
  const [issue, setIssue] = useState("");
  const [lists, setLists] = useState<Record<string, PhaseList>>({});
  const [style, setStyle] = useState<ScheduleStyle | null>(null);
  const [fonts, setFonts] = useState<[TextFont, string][]>([]);
  const [error, setError] = useState<string | null>(null);
  const [selected, setSelected] = useState<number | null>(null);
  const [drag, setDrag] = useState<number | null>(null);
  // While a row is dragged by its handle: the gap (0..rows) it would drop into.
  const [over, setOver] = useState<number | null>(null);
  const listRef = useRef<HTMLDivElement | null>(null);
  const [menu, setMenu] = useState<{ x: number; y: number; row: number | null } | null>(null);
  const [rowText, setRowText] = useState<string | null>(null);
  const [scale, setScale] = useState(fit);

  useEffect(() => {
    const onResize = () => setScale(fit());
    window.addEventListener("resize", onResize);
    return () => window.removeEventListener("resize", onResize);
  }, []);
  useEffect(() => {
    let live = true;
    void Promise.all([ipc.scheduleStyle(view), ipc.textFonts(), ipc.sheetIndexIssues()])
      .then(async ([s, f, [iss, current]]) => {
        if (!live) return;
        setStyle(s);
        setFonts(f);
        setIssues(iss);
        // The current phase's sheets as they are (its typical list on picking a phase).
        const key = defaultIssue(iss, current);
        const stage = iss.find((i) => i.key === key)?.stage ?? null;
        const r = await ipc.sheetIndexRows(stage);
        if (!live) return;
        setIssue(key);
        setLists({ [stage ?? ""]: { rows: r, dirty: false, had: sheetsOf(r) } });
        setSelected(r.length ? 0 : null);
      })
      .catch((e) => setError(errorMessage(e)));
    return () => {
      live = false;
    };
  }, [view]);

  // The phase shown, and its list.
  const shown = issues.find((i) => i.key === issue) ?? null;
  const stage = shown?.stage ?? null;
  const slot = stage ?? "";
  const rows = lists[slot]?.rows ?? null;
  const setRows = (r: IndexRow[]) =>
    setLists((l) => ({ ...l, [slot]: { ...(l[slot] ?? { had: [] }), rows: r, dirty: true } }));
  /** Shows another phase: its list as edited, else its typical sheet list. */
  const pickIssue = async (key: string) => {
    setIssue(key);
    setSelected(null);
    const st = issues.find((i) => i.key === key)?.stage ?? null;
    if (!st || lists[st]) return;
    try {
      const [now, typical] = await Promise.all([
        ipc.sheetIndexRows(st),
        ipc.sheetIndexTypical(st, buildingType()),
      ]);
      setLists((l) =>
        l[st] ? l : { ...l, [st]: { rows: typical, dirty: false, had: sheetsOf(now) } },
      );
    } catch (e) {
      setError(errorMessage(e));
    }
  };

  // The sheet it's on: the open sheet, else the first it's placed on.
  const views = app?.views ?? [];
  const open = views.find((v) => v.id === activeView);
  const sheetId =
    open?.viewType === "Sheet" ? open.id : (views.find((v) => v.id === view)?.onSheet ?? null);
  const sheetNumber =
    (sheetId ? rows?.find((r) => r.sheet === sheetId)?.number : undefined) ??
    views.find((v) => v.id === sheetId)?.name.split(" ")[0] ??
    "";

  const list = rows ?? [];
  const problems = indexProblems(list);
  const set = (i: number, patch: Partial<IndexRow>) =>
    setRows(list.map((r, k) => (k === i ? { ...r, ...patch } : r)));
  const add = async (at: number, placeholder: boolean) => {
    const taken = list.map((r) => r.number);
    const after = list[at - 1]?.number ?? list[list.length - 1]?.number ?? "A-100";
    const number = await ipc.nextIndexNumber(after, taken).catch(() => "");
    setRows(insertRow(list, at, { sheet: null, number, name: "", placeholder }));
    setSelected(at);
  };
  const move = (from: number, to: number) => {
    if (to < 0 || to >= list.length) return;
    setRows(moveRow(list, from, to));
    setSelected(to);
  };
  const remove = (i: number) => {
    if (!list[i] || !removable(list[i]!, stage !== null)) return;
    setRows(list.filter((_, k) => k !== i));
    setSelected(list.length > 1 ? Math.min(i, list.length - 2) : null);
  };
  const setRow = (mm: number) =>
    style &&
    setStyle({
      ...style,
      // 3–15 mm, and never shorter than twice the body text (as Rust keeps it).
      row: Math.max(3, Math.ceil(style.body * 4) / 2, Math.min(15, Math.round(mm * 2) / 2)),
    });

  // OK saves the type, the phase shown, and any other phase whose list was edited.
  const toSave = Object.entries(lists).filter(([k, l]) => k === slot || l.dirty);
  const blocked = toSave.some(([, l]) => indexProblems(l.rows).length > 0);
  const save = async () => {
    if (!style || !rows || blocked) return;
    if (!(await apply(() => ipc.setScheduleStyle(view, style)))) return;
    for (const [k, l] of toSave)
      if (!(await apply(() => ipc.setSheetIndexRows(l.rows, k || null)))) return;
    onClose();
  };

  // Dragging a row by its handle (ADR-115): pointer events, so it works in the app's
  // window (which keeps HTML drag and drop for dropping files). The list scrolls near its ends.
  const dragTo = (y: number) => {
    const el = listRef.current;
    if (!el) return;
    const box = el.getBoundingClientRect();
    if (y < box.top + 24) el.scrollTop -= 12;
    else if (y > box.bottom - 24) el.scrollTop += 12;
    const mids = Array.from(el.children).map((c) => {
      const r = c.getBoundingClientRect();
      return (r.top + r.bottom) / 2;
    });
    const gap = dropGap(mids, y);
    if (gap !== over) setOver(gap);
  };
  const endDrag = (drop: boolean) => {
    if (drop && drag !== null && over !== null) {
      const to = dropTo(drag, over);
      if (to !== drag) move(drag, to);
    }
    setDrag(null);
    setOver(null);
  };
  // The gap a line shows at: none where the row would stay put.
  const showGap = drag !== null && over !== null && dropTo(drag, over) !== drag ? over : null;

  const items = (i: number | null): MenuItem[] => {
    if (i === null)
      return [
        { label: "Add Sheet", onClick: () => void add(list.length, false) },
        { label: "Add Placeholder Sheet", onClick: () => void add(list.length, true) },
      ];
    const r = list[i]!;
    const last = list.length - 1;
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
        label: r.sheet && !r.placeholder && shown ? `Remove from ${shown.phase}` : "Remove",
        disabled: !removable(r, stage !== null),
        onClick: () => remove(i),
        separator: r.sheet !== null,
      },
    ];
  };

  const nNew = list.filter((r) => r.sheet === null && !r.placeholder).length;
  const nPh = list.filter((r) => r.placeholder).length;
  // Sheets joining or leaving the phase's set.
  const had = lists[slot]?.had ?? [];
  const listed = sheetsOf(list);
  const joins = listed.filter((id) => !had.includes(id)).length;
  const leaves = stage ? had.filter((id) => !listed.includes(id)).length : 0;
  const others = toSave.filter(([k]) => k !== slot).length;
  const foot =
    (nNew ? `${nNew} new sheet${nNew > 1 ? "s" : ""} will be created` : "No new sheets") +
    (joins && shown ? ` · ${joins} join${joins > 1 ? "" : "s"} the ${shown.phase} set` : "") +
    (leaves && shown ? ` · ${leaves} leave${leaves > 1 ? "" : "s"} the ${shown.phase} set` : "") +
    (nPh ? ` · ${nPh} placeholder${nPh > 1 ? "s" : ""}` : "") +
    (others ? ` · edits in ${others} other phase${others > 1 ? "s" : ""}` : "");
  const sel = selected !== null ? list[selected] : undefined;
  const sizes: [string, keyof ScheduleStyle, number[]][] = [
    ["TITLE", "title", TITLE],
    ["HEADERS", "header", HEADERS],
    ["BODY", "body", BODY],
  ];

  return (
    <div className="sid-backdrop" role="dialog" aria-label="Sheet Index">
      <div className="sid" style={{ zoom: scale }}>
        <header className="sid-head">
          <div className="sid-titles">
            <div className="sid-eyebrow">SCHEDULE{sheetNumber ? ` · ${sheetNumber}` : ""}</div>
            <h2 className="sid-title">{title || "SHEET INDEX"}</h2>
          </div>
          {issues.length > 0 && (
            <label className="sid-field sid-phase">
              <span>PHASE / ISSUE</span>
              <select
                aria-label="Phase"
                value={issue}
                onChange={(e) => void pickIssue(e.target.value)}
              >
                {issues.map((i) => (
                  <option key={i.key} value={i.key}>
                    {i.phase} · {i.label}
                  </option>
                ))}
              </select>
            </label>
          )}
          <div className="sid-count">
            <span>{list.length} SHEETS</span>
            <button className="sid-close" aria-label="Close" onClick={onClose}>
              ✕
            </button>
          </div>
        </header>
        {error && <p className="error sid-error">{error}</p>}
        <div className="sid-body">
          <section className="sid-left">
            <div className="sid-toolbar">
              <h3>Sheets{shown ? ` · ${shown.label}` : ""}</h3>
              <button
                className="sid-remove"
                title={
                  sel && !removable(sel, stage !== null)
                    ? "A sheet in the project is deleted in the project browser"
                    : sel?.sheet && !sel.placeholder && shown
                      ? `Take it out of the ${shown.phase} set (the sheet stays in the project)`
                      : "Remove"
                }
                disabled={!sel || !removable(sel, stage !== null)}
                onClick={() => selected !== null && remove(selected)}
              >
                REMOVE
              </button>
            </div>
            <div className="sid-cols">
              <span />
              <span>NUMBER</span>
              <span>NAME</span>
              <span className="r">TYPE</span>
            </div>
            <div
              ref={listRef}
              className="sid-rows"
              role="list"
              aria-label="Sheet index rows"
              onContextMenu={(e) => {
                e.preventDefault();
                setMenu({ x: e.clientX, y: e.clientY, row: null });
              }}
            >
              {list.map((r, i) => {
                const kind = rowKind(r);
                const bad = problems.some((p) => p.row === i);
                return (
                  <div
                    key={`${r.sheet ?? "new"}-${i}`}
                    role="listitem"
                    aria-label={`${r.number} ${r.name}`}
                    className={`sid-row${selected === i ? " on" : ""}${drag === i ? " dragging" : ""}${showGap === i ? " over" : ""}${showGap === list.length && i === list.length - 1 ? " over-end" : ""}`}
                    onClick={() => setSelected(i)}
                    onContextMenu={(e) => {
                      e.preventDefault();
                      e.stopPropagation();
                      setSelected(i);
                      setMenu({ x: e.clientX, y: e.clientY, row: i });
                    }}
                  >
                    <span
                      className="sid-handle"
                      title="Drag to reorder"
                      aria-label={`Drag ${r.number} to reorder`}
                      onPointerDown={(e) => {
                        if (e.button !== 0) return;
                        e.preventDefault();
                        e.currentTarget.setPointerCapture?.(e.pointerId);
                        setDrag(i);
                        setOver(i);
                        setSelected(i);
                      }}
                      onPointerMove={(e) => drag !== null && dragTo(e.clientY)}
                      onPointerUp={() => endDrag(true)}
                      onPointerCancel={() => endDrag(false)}
                    >
                      ⋮⋮
                    </span>
                    <input
                      className={`sid-num${bad ? " bad" : ""}`}
                      aria-label={`Row ${i + 1} number`}
                      value={r.number}
                      onChange={(e) => set(i, { number: e.target.value })}
                    />
                    <input
                      className="sid-name"
                      aria-label={`Row ${i + 1} name`}
                      value={r.name}
                      placeholder="Sheet name"
                      onChange={(e) => set(i, { name: e.target.value })}
                    />
                    <span className={`sid-tag ${kind.toLowerCase().replace(" ", "-")}`}>
                      {kind.toUpperCase()}
                    </span>
                  </div>
                );
              })}
            </div>
            <div className="sid-add">
              <div className="sid-joined wide">
                <button className="sid-big" onClick={() => void add(list.length, false)}>
                  + ADD SHEET
                </button>
                <button className="sid-big" onClick={() => void add(list.length, true)}>
                  + ADD PLACEHOLDER
                </button>
              </div>
              <p className="sid-hint">
                Pick a phase above for its typical sheet list. Drag ⋮⋮ to reorder, or right-click a
                row. New sheets are created when you click OK; placeholders only appear in the
                index.
              </p>
            </div>
          </section>
          <section className="sid-right">
            <div className="sid-settings">
              <label className="sid-field font">
                <span>FONT</span>
                <select
                  aria-label="Font"
                  value={style?.font ?? "Drafting"}
                  onChange={(e) =>
                    style && setStyle({ ...style, font: e.target.value as TextFont })
                  }
                >
                  {fonts.map(([f, l]) => (
                    <option key={f} value={f}>
                      {l}
                    </option>
                  ))}
                </select>
              </label>
              <div className="sid-field">
                <span>ROW (MM)</span>
                <div className="sid-stepper">
                  <button aria-label="Row smaller" onClick={() => style && setRow(style.row - 0.5)}>
                    −
                  </button>
                  <input
                    aria-label="Row height"
                    value={rowText ?? String(style?.row ?? "")}
                    onChange={(e) => {
                      setRowText(e.target.value);
                      const v = parseFloat(e.target.value);
                      if (!isNaN(v)) setRow(v);
                    }}
                    onBlur={() => setRowText(null)}
                  />
                  <button aria-label="Row larger" onClick={() => style && setRow(style.row + 0.5)}>
                    +
                  </button>
                </div>
              </div>
              {sizes.map(([name, key, opts]) => (
                <div className="sid-field" key={name}>
                  <span>{name}</span>
                  <div className="sid-seg" role="group" aria-label={`${name} height`}>
                    {opts.map((o) => {
                      const on = style ? nearest(style[key] as number, opts) === o : false;
                      return (
                        <button
                          key={o}
                          className={on ? "on" : ""}
                          aria-pressed={on}
                          onClick={() => style && setStyle({ ...style, [key]: o * IN })}
                        >
                          {label(o)}
                        </button>
                      );
                    })}
                  </div>
                </div>
              ))}
            </div>
            <div className="sid-caption">
              <span>PREVIEW · 1:1 AT SHEET SCALE</span>
              <span>
                TABLE HEIGHT ≈ {style ? +((list.length + 1) * style.row).toFixed(1) : 0} MM
              </span>
            </div>
            <div className="sid-preview">
              {style && (
                <div
                  className="sid-sheet"
                  style={{ fontFamily: FAMILY[style.font], gap: px(style.row * 0.6) }}
                >
                  <div className="sid-ptitle" style={{ fontSize: px(style.title) }}>
                    {title || "SHEET INDEX"}
                  </div>
                  <div className="sid-table">
                    {["SHEET NUMBER", "SHEET NAME"].map((h, k) => (
                      <div
                        key={h}
                        className={`sid-th${k === 0 ? " c0" : ""}`}
                        style={{ height: px(style.row), fontSize: px(style.header) }}
                      >
                        {h}
                      </div>
                    ))}
                    {list.flatMap((r, i) => [
                      <div
                        key={`n${i}`}
                        className="sid-td c0 num"
                        style={{ height: px(style.row), fontSize: px(style.body) }}
                      >
                        {r.number}
                      </div>,
                      <div
                        key={`m${i}`}
                        className={`sid-td${r.placeholder ? " ph" : ""}`}
                        style={{ height: px(style.row), fontSize: px(style.body) }}
                      >
                        {r.name || " "}
                      </div>,
                    ])}
                  </div>
                </div>
              )}
            </div>
          </section>
        </div>
        <footer className="sid-foot">
          <div className={`sid-status${problems.length ? " bad" : ""}`}>
            {problems.length ? problems[0]!.message : foot}
          </div>
          <div className="sid-actions">
            <button className="sid-cancel" onClick={onClose}>
              CANCEL
            </button>
            <button
              className="sid-ok"
              disabled={!style || !rows || blocked}
              onClick={() => void save()}
            >
              OK
            </button>
          </div>
        </footer>
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
