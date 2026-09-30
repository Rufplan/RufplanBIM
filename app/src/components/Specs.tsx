import { useEffect, useLayoutEffect, useRef, useState, type ReactNode } from "react";
import { create } from "zustand";
import type { SpecArticle } from "../bindings/SpecArticle";
import type { SpecChange } from "../bindings/SpecChange";
import type { SpecEditPlan } from "../bindings/SpecEditPlan";
import type { SpecNumbering } from "../bindings/SpecNumbering";
import type { SpecSection } from "../bindings/SpecSection";
import type { SpecState } from "../bindings/SpecState";
import type { SpecStyle } from "../bindings/SpecStyle";
import { apply } from "../fileActions";
import { dialogs, errorMessage, ipc } from "../ipc";
import { useAppStore } from "../store";

// The Specifications tab (ADR-085): the project manual, generated from the model and
// Project Info (studio-specs picks the MasterFormat sections the building calls for),
// edited here section by section, styled, and exported to PDF or Word. Edit Specs asks
// Claude for changes, previewed as a diff and applied as one undo step. The book lives in
// the project (studio-core `specs`); this module shows and edits it.

export const ISSUES = [
  "Schematic Design",
  "Design Development",
  "Issued for Permit",
  "Bid Set",
  "Issued for Construction",
  "Addendum 1",
];

const FONTS: Record<string, string> = {
  Serif: '"Times New Roman", Tinos, Times, serif',
  Sans: 'Calibri, Carlito, "Segoe UI", Arial, sans-serif',
  Brand: 'Barlow, "Segoe UI", sans-serif',
};

// ---- State ----

interface LogEntry {
  prompt: string;
  summary: string;
  label: string;
  undone: boolean;
}

interface SpecsUi {
  state: SpecState | null;
  /** The section open in the editor, by number. */
  selected: string | null;
  draft: SpecSection | null;
  dirty: boolean;
  saving: boolean;
  query: string;
  collapsed: Record<string, boolean>;
  dialog: null | "add" | "custom" | "generate";
  editOpen: boolean;
  log: LogEntry[];
  note: string | null;
}

export const useSpecs = create<SpecsUi>(() => ({
  state: null,
  selected: null,
  draft: null,
  dirty: false,
  saving: false,
  query: "",
  collapsed: {},
  dialog: null,
  editOpen: false,
  log: [],
  note: null,
}));

let timer = 0;

export async function refreshSpecs() {
  try {
    const state = await ipc.specState();
    const s = useSpecs.getState();
    const sections = state.book?.sections ?? [];
    const selected =
      s.selected && sections.some((x) => x.number === s.selected)
        ? s.selected
        : (sections.find((x) => x.kind === "ThreePart")?.number ?? sections[0]?.number ?? null);
    const fresh = sections.find((x) => x.number === selected) ?? null;
    useSpecs.setState({
      state,
      selected,
      ...(s.dirty && s.selected === selected
        ? {}
        : { draft: fresh && structuredClone(fresh), dirty: false }),
    });
  } catch {
    // No project open.
  }
}

/** Saves the section being edited now. */
export async function commitSection() {
  window.clearTimeout(timer);
  const { draft, dirty, selected } = useSpecs.getState();
  if (!draft || !dirty || !selected) return;
  if (!/^\d\d \d\d \d\d(\.\d\d)?$/.test(draft.number) || !draft.title.trim()) return;
  useSpecs.setState({ saving: true });
  const sent = draft;
  const ok = await apply(() => ipc.specSetSection(selected, sent));
  const now = useSpecs.getState();
  useSpecs.setState({
    saving: false,
    dirty: now.draft === sent ? !ok : true,
    selected: ok && now.selected === selected ? sent.number : now.selected,
  });
  await refreshSpecs();
}

function editDraft(f: (s: SpecSection) => void) {
  const { draft } = useSpecs.getState();
  if (!draft) return;
  const next = structuredClone(draft);
  f(next);
  useSpecs.setState({ draft: next, dirty: true });
  window.clearTimeout(timer);
  timer = window.setTimeout(() => void commitSection(), 700);
}

async function select(number: string) {
  await commitSection();
  const book = useSpecs.getState().state?.book;
  const s = book?.sections.find((x) => x.number === number) ?? null;
  useSpecs.setState({ selected: number, draft: s && structuredClone(s), dirty: false });
}

function useSpecState() {
  const revision = useAppStore((s) => s.app?.revision ?? 0);
  useEffect(() => {
    void refreshSpecs();
  }, [revision]);
  return useSpecs();
}

function note(text: string) {
  useSpecs.setState({ note: text });
  window.setTimeout(() => {
    if (useSpecs.getState().note === text) useSpecs.setState({ note: null });
  }, 4000);
}

// ---- Numbering (presentational; the writers number the same way in Rust) ----

function letters(n: number, upper: boolean) {
  const i = Math.max(n, 1) - 1;
  const s = String.fromCharCode(65 + (i % 26)).repeat(Math.floor(i / 26) + 1);
  return upper ? s : s.toLowerCase();
}

export function paraLabel(n: SpecNumbering, level: number, count: number, path: number[]) {
  if (n === "Decimal") return path.join(".");
  if (n === "Bullets") return level === 0 ? "•" : "–";
  return level === 0
    ? `${letters(count, true)}.`
    : level === 1
      ? `${count}.`
      : level === 2
        ? `${letters(count, false)}.`
        : `${count})`;
}

export function articleLabel(n: SpecNumbering, part: number, a: number) {
  if (n === "Bullets") return "";
  if (n === "CsiZero") return `${part}.${String(a).padStart(2, "0")}`;
  return `${part}.${a}`;
}

/** Label and text indents in pt, as the PDF sets them. */
function indents(n: SpecNumbering, level: number | null): [number, number] {
  if (n === "Bullets") return level === null ? [0, 0] : [14 * level, 14 * (level + 1)];
  if (n === "Decimal") {
    if (level === null) return [0, 40];
    const at = [40, 94, 154, 220];
    const to = [94, 154, 220, 290];
    const l = Math.min(level, 3);
    return [at[l]!, to[l]!];
  }
  return level === null ? [0, 36] : [36 * (level + 1), 36 * (level + 2)];
}

export function titleCase(s: string) {
  const small = ["and", "or", "of", "the", "for", "to", "in", "a", "an", "with", "by", "on", "at"];
  const keep = ["HVAC", "LED", "TPO", "PVC", "EPDM", "CMU", "EIFS", "AV", "MEP"];
  return s
    .split(" ")
    .map((w, i) => {
      if (keep.includes(w.replace(/[^A-Z0-9]/gi, ""))) return w;
      return w
        .split("-")
        .map((p, k) => {
          const l = p.toLowerCase();
          if ((i > 0 || k > 0) && small.includes(l)) return l;
          const j = l.startsWith("(") ? 1 : 0;
          return l.slice(0, j) + l.charAt(j).toUpperCase() + l.slice(j + 1);
        })
        .join("-");
    })
    .join(" ");
}

const divisionOf = (n: string) => n.slice(0, 2);

// ---- Icons ----

function StrokeIcon({ d, size = 18 }: { d: string; size?: number }) {
  return (
    <svg
      width={size}
      height={size}
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

const ICON = {
  book: "M5 4h11a3 3 0 0 1 3 3v13H8a3 3 0 0 1-3-3zM5 17a3 3 0 0 1 3-3h11M9 8h6",
  update: "M20 12a8 8 0 1 1-2.3-5.6M20 4v4h-4",
  add: "M4 6h10M4 11h10M4 16h6M18 13v8M14 17h8",
  custom: "M6 3h9l4 4v14H6zM14 3v5h5M12 11v6M9 14h6",
  include: "M4 12l5 5L20 6",
  exclude: "M5 5l14 14M19 5L5 19",
  remove: "M4 7h16M9 7V4h6v3M6 7l1 13h10l1-13",
  pdf: "M6 3h9l4 4v14H6zM14 3v5h5M8.5 16.5h7M8.5 13h7",
  word: "M6 3h9l4 4v14H6zM14 3v5h5M8 12l1.5 6 2.5-5 2.5 5L16 12",
};

const sparkle = (color: string) => (
  <svg
    width="18"
    height="18"
    viewBox="0 0 24 24"
    fill="none"
    stroke={color}
    strokeWidth="2"
    strokeLinecap="round"
    strokeLinejoin="round"
    aria-hidden
  >
    <path d="M12 3l2 5.5L19.5 10.5 14 12.5 12 18l-2-5.5L4.5 10.5 10 8.5z" />
    <path d="M19 17l.8 2.2L22 20l-2.2.8L19 23l-.8-2.2L16 20l2.2-.8z" />
  </svg>
);

// ---- Actions ----

async function exportBook(format: "pdf" | "docx") {
  await commitSection();
  const project = useAppStore.getState().app?.projectName || "Project";
  const name = `${project} - Project Manual`;
  const path =
    format === "pdf" ? await dialogs.pickPdfLocation(name) : await dialogs.pickWordLocation(name);
  if (!path) return;
  try {
    const [written, pages] = await ipc.specExport(path, format);
    note(`Saved ${pages} pages to ${written}`);
  } catch (e) {
    useAppStore.getState().setError(errorMessage(e));
  }
}

async function updateFromModel() {
  await commitSection();
  try {
    const [report, state] = await ipc.specUpdate();
    if (state) useAppStore.getState().setApp(state);
    note(
      report.added.length
        ? `Added ${report.added.length} section${report.added.length === 1 ? "" : "s"} the model now calls for: ${report.added.join(", ")}`
        : report.unindicated.length
          ? `Up to date. ${report.unindicated.length} section(s) the model no longer calls for are listed in Coordination.`
          : "The manual has every section the model calls for.",
    );
    await refreshSpecs();
  } catch (e) {
    useAppStore.getState().setError(errorMessage(e));
  }
}

// ---- Ribbon ----

export function SpecsRibbon() {
  const { state, selected, dialog } = useSpecs();
  const book = state?.book ?? null;
  const sel = book?.sections.find((s) => s.number === selected) ?? null;
  const setOpen = (d: SpecsUi["dialog"]) => useSpecs.setState({ dialog: d === dialog ? null : d });
  return (
    <>
      <div className="rb-group">
        <div className="rb-items">
          <button
            className="rb-btn std-btn wide"
            title="Make the project manual from the model and Project Info"
            onClick={() => setOpen("generate")}
          >
            <StrokeIcon d={ICON.book} />
            <span>{book ? "Regenerate" : "Generate"}</span>
          </button>
          <button
            className="rb-btn std-btn wide"
            disabled={!book}
            title="Add the sections the model now calls for (edited sections are never replaced)"
            onClick={() => void updateFromModel()}
          >
            <StrokeIcon d={ICON.update} />
            <span>Update from Model</span>
          </button>
        </div>
        <div className="rb-title">MANUAL</div>
      </div>
      <div className="rb-group">
        <div className="rb-items">
          <button
            className="rb-btn std-btn"
            disabled={!book}
            title="Add sections from the library"
            onClick={() => setOpen("add")}
          >
            <StrokeIcon d={ICON.add} />
            <span>Add</span>
          </button>
          <button
            className="rb-btn std-btn"
            disabled={!book}
            title="Write a new section"
            onClick={() => setOpen("custom")}
          >
            <StrokeIcon d={ICON.custom} />
            <span>New</span>
          </button>
          <button
            className="rb-btn std-btn"
            disabled={!sel}
            title={
              sel?.included ? "Leave this section out of the issued manual" : "Issue this section"
            }
            onClick={() =>
              sel && void apply(() => ipc.specSetIncluded([sel.number], !sel.included))
            }
          >
            <StrokeIcon d={sel?.included === false ? ICON.include : ICON.exclude} />
            <span>{sel?.included === false ? "Include" : "Exclude"}</span>
          </button>
          <button
            className="rb-btn std-btn"
            disabled={!sel}
            title="Remove this section from the manual"
            onClick={() => {
              if (!sel) return;
              void commitSection().then(() => apply(() => ipc.specRemove([sel.number])));
            }}
          >
            <StrokeIcon d={ICON.remove} />
            <span>Remove</span>
          </button>
        </div>
        <div className="rb-title">SECTIONS</div>
      </div>
      <div className="rb-group">
        <div className="rb-items std-library">
          <label>
            <span>Section Style</span>
            <select
              aria-label="Section Style"
              disabled={!book}
              value={book?.style ?? ""}
              onChange={(e) =>
                book && void apply(() => ipc.specSetSettings(e.target.value, book.issue, book.date))
              }
            >
              {state?.styles.map((s) => (
                <option key={s.id} value={s.id}>
                  {s.name}
                </option>
              ))}
            </select>
          </label>
        </div>
        <div className="rb-title">STYLE</div>
      </div>
      <div className="rb-group">
        <div className="rb-items">
          <button
            className="rb-btn std-btn"
            disabled={!book}
            title="Export the issued manual as a vector PDF"
            onClick={() => void exportBook("pdf")}
          >
            <StrokeIcon d={ICON.pdf} />
            <span>PDF</span>
          </button>
          <button
            className="rb-btn std-btn"
            disabled={!book}
            title="Export the issued manual as a Word document (.docx) with live numbering"
            onClick={() => void exportBook("docx")}
          >
            <StrokeIcon d={ICON.word} />
            <span>Word</span>
          </button>
        </div>
        <div className="rb-title">OUTPUT</div>
      </div>
      <div className="rb-group">
        <div className="rb-items">
          <button
            className="rb-btn rb-claude"
            disabled={!book}
            title="Edit specs with Claude (Ctrl+K)"
            onClick={() => useSpecs.setState({ editOpen: true })}
          >
            {sparkle("#29B5E8")}
            <span>Edit Specs</span>
          </button>
        </div>
        <div className="rb-title">CLAUDE</div>
      </div>
    </>
  );
}

// ---- Browser ----

export function SpecsBrowser() {
  const { state, selected, query, collapsed } = useSpecState();
  const book = state?.book ?? null;
  const q = query.trim().toLowerCase();
  const sections = (book?.sections ?? []).filter(
    (s) => !q || s.number.includes(q) || s.title.toLowerCase().includes(q),
  );
  const divisions = state?.divisions ?? [];
  const included = book?.sections.filter((s) => s.included).length ?? 0;
  const edited = book?.sections.filter((s) => s.edited).length ?? 0;
  return (
    <aside className="panel browser std-browser sp-browser" aria-label="Specifications Browser">
      <div className="panel-title">Project Manual</div>
      <div className="sp-search">
        <input
          aria-label="Find a section"
          placeholder="Find a section…"
          value={query}
          onChange={(e) => useSpecs.setState({ query: e.target.value })}
        />
      </div>
      <div className="std-tree">
        {!book && <div className="sp-empty-tree">Generate the manual to see its sections.</div>}
        {divisions
          .filter((d) => sections.some((s) => divisionOf(s.number) === d.code))
          .map((d) => {
            const open = !collapsed[d.code] || !!q;
            const inDiv = sections.filter((s) => divisionOf(s.number) === d.code);
            return (
              <div key={d.code} className="sp-div">
                <button
                  className="std-root sp-div-head"
                  aria-expanded={open}
                  onClick={() => useSpecs.setState({ collapsed: { ...collapsed, [d.code]: open } })}
                >
                  <span aria-hidden>{open ? "▼" : "▶"}</span>
                  <b>{d.code}</b>
                  <em>{titleCase(d.title)}</em>
                  <i>{inDiv.length}</i>
                </button>
                {open &&
                  inDiv.map((s) => (
                    <div
                      key={s.number}
                      className={`std-row sp-row${s.number === selected ? " on" : ""}${s.included ? "" : " out"}`}
                    >
                      <input
                        type="checkbox"
                        aria-label={`Include ${s.number}`}
                        checked={s.included}
                        onChange={(e) =>
                          void apply(() => ipc.specSetIncluded([s.number], e.target.checked))
                        }
                      />
                      <button className="sp-row-main" onClick={() => void select(s.number)}>
                        <span className="sp-num">{s.number}</span>
                        <span className="sp-title">{titleCase(s.title)}</span>
                      </button>
                      {s.origin === "Claude" ? (
                        <span className="sp-badge claude" title="Written or edited by Claude">
                          AI
                        </span>
                      ) : s.edited ? (
                        <span className="sp-badge" title="Edited">
                          ●
                        </span>
                      ) : null}
                    </div>
                  ))}
              </div>
            );
          })}
      </div>
      <div className="std-foot">
        <div className="std-foot-head">
          <span>ISSUED</span>
          <span>
            {included}/{book?.sections.length ?? 0}
          </span>
        </div>
        <div className="std-bar">
          <div
            style={{
              width: `${book?.sections.length ? (included / book.sections.length) * 100 : 0}%`,
            }}
          />
        </div>
        <div className="std-foot-line">
          {included} sections issued · {edited} edited
        </div>
      </div>
    </aside>
  );
}

// ---- The editor ----

/** A textarea that grows with its text and takes focus when asked. */
const focusWanted: { key: string | null; caret: number } = { key: null, caret: 0 };

function want(key: string, caret = 0) {
  focusWanted.key = key;
  focusWanted.caret = caret;
}

function AutoText({
  id,
  value,
  onChange,
  onKeyDown,
  className,
  label,
  placeholder,
}: {
  id: string;
  value: string;
  onChange: (v: string) => void;
  onKeyDown?: (e: React.KeyboardEvent<HTMLTextAreaElement>) => void;
  className?: string;
  label: string;
  placeholder?: string;
}) {
  const ref = useRef<HTMLTextAreaElement>(null);
  useLayoutEffect(() => {
    const t = ref.current;
    if (!t) return;
    t.style.height = "auto";
    t.style.height = `${t.scrollHeight}px`;
    if (focusWanted.key === id) {
      focusWanted.key = null;
      t.focus();
      const c = Math.min(focusWanted.caret, t.value.length);
      t.setSelectionRange(c, c);
    }
  });
  return (
    <textarea
      ref={ref}
      data-key={id}
      rows={1}
      aria-label={label}
      className={className}
      value={value}
      placeholder={placeholder}
      onChange={(e) => onChange(e.target.value)}
      onKeyDown={onKeyDown}
    />
  );
}

function ArticleEditor({
  article,
  pi,
  ai,
  numbering,
  partNo,
  count,
  bold,
}: {
  article: SpecArticle;
  pi: number;
  ai: number;
  numbering: SpecNumbering;
  partNo: number;
  count: number;
  bold: boolean;
}) {
  const at = (s: SpecSection) => s.parts[pi]!.articles[ai]!;
  const key = (i: number) => `p${pi}a${ai}i${i}`;
  const counts = [0, 0, 0, 0];
  const [alx, atx] = indents(numbering, null);
  const onKey = (i: number) => (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    const t = e.currentTarget;
    const p = article.paragraphs[i]!;
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      const head = t.value.slice(0, t.selectionStart);
      const tail = t.value.slice(t.selectionEnd);
      // A paragraph ending in ":" starts its subparagraphs one level down.
      const level = !tail && head.trimEnd().endsWith(":") ? Math.min(p.level + 1, 3) : p.level;
      editDraft((s) => {
        const ps = at(s).paragraphs;
        ps[i] = { level: p.level, text: head };
        ps.splice(i + 1, 0, { level, text: tail });
      });
      want(key(i + 1), 0);
    } else if (e.key === "Tab") {
      e.preventDefault();
      const level = Math.max(0, Math.min(3, p.level + (e.shiftKey ? -1 : 1)));
      want(key(i), t.selectionStart);
      editDraft((s) => void (at(s).paragraphs[i]!.level = level));
    } else if (e.key === "Backspace" && t.selectionStart === 0 && t.selectionEnd === 0 && i > 0) {
      e.preventDefault();
      const prev = article.paragraphs[i - 1]!;
      want(key(i - 1), prev.text.length);
      editDraft((s) => {
        const ps = at(s).paragraphs;
        ps[i - 1] = {
          level: prev.level,
          text: prev.text + (p.text ? (prev.text ? " " : "") + p.text : ""),
        };
        ps.splice(i, 1);
      });
    } else if (e.altKey && (e.key === "ArrowUp" || e.key === "ArrowDown")) {
      e.preventDefault();
      const j = e.key === "ArrowUp" ? i - 1 : i + 1;
      if (j < 0 || j >= article.paragraphs.length) return;
      want(key(j), t.selectionStart);
      editDraft((s) => {
        const ps = at(s).paragraphs;
        [ps[i], ps[j]] = [ps[j]!, ps[i]!];
      });
    } else if (e.key === "ArrowUp" && t.selectionStart === 0 && i > 0) {
      e.preventDefault();
      (document.querySelector(`[data-key="${key(i - 1)}"]`) as HTMLTextAreaElement | null)?.focus();
    } else if (
      e.key === "ArrowDown" &&
      t.selectionEnd === t.value.length &&
      i < article.paragraphs.length - 1
    ) {
      e.preventDefault();
      (document.querySelector(`[data-key="${key(i + 1)}"]`) as HTMLTextAreaElement | null)?.focus();
    }
  };
  return (
    <div className="sp-article">
      <div className="sp-art-head" style={{ paddingLeft: `${alx}pt` }}>
        <span className="sp-label" style={{ width: `${atx - alx}pt` }}>
          {articleLabel(numbering, partNo, count)}
        </span>
        <input
          aria-label="Article title"
          className={`sp-art-title${bold ? " bold" : ""}`}
          value={article.title}
          onChange={(e) => editDraft((s) => void (at(s).title = e.target.value.toUpperCase()))}
        />
        <span className="sp-tools">
          <button
            title="Move article up"
            aria-label="Move article up"
            disabled={ai === 0}
            onClick={() =>
              editDraft((s) => {
                const a = s.parts[pi]!.articles;
                [a[ai - 1], a[ai]] = [a[ai]!, a[ai - 1]!];
              })
            }
          >
            ↑
          </button>
          <button
            title="Move article down"
            aria-label="Move article down"
            onClick={() =>
              editDraft((s) => {
                const a = s.parts[pi]!.articles;
                if (ai + 1 < a.length) [a[ai + 1], a[ai]] = [a[ai]!, a[ai + 1]!];
              })
            }
          >
            ↓
          </button>
          <button
            title="Add a paragraph"
            aria-label="Add a paragraph"
            onClick={() => {
              want(key(article.paragraphs.length));
              editDraft((s) => void at(s).paragraphs.push({ level: 0, text: "" }));
            }}
          >
            ¶+
          </button>
          <button
            className="danger"
            title="Delete article"
            aria-label="Delete article"
            onClick={() => editDraft((s) => void s.parts[pi]!.articles.splice(ai, 1))}
          >
            ×
          </button>
        </span>
      </div>
      {article.paragraphs.map((p, i) => {
        const l = Math.min(p.level, 3);
        counts[l] = counts[l]! + 1;
        for (let k = l + 1; k < 4; k++) counts[k] = 0;
        const path = [partNo, count, ...counts.slice(0, l + 1)];
        const [lx, tx] = indents(numbering, l);
        return (
          <div key={i} className="sp-para" style={{ paddingLeft: `${lx}pt` }}>
            <span className="sp-label" style={{ width: `${tx - lx}pt` }}>
              {paraLabel(numbering, l, counts[l]!, path)}
            </span>
            <AutoText
              id={key(i)}
              label={`Paragraph ${i + 1} of ${article.title}`}
              className="sp-text"
              value={p.text}
              placeholder="Type the paragraph…"
              onChange={(v) => editDraft((s) => void (at(s).paragraphs[i]!.text = v))}
              onKeyDown={onKey(i)}
            />
          </div>
        );
      })}
    </div>
  );
}

function SectionHeading({ s, style }: { s: SpecSection; style: SpecStyle }) {
  const title = (
    <input
      aria-label="Section title"
      className="sp-sec-title"
      value={s.title}
      onChange={(e) => editDraft((x) => void (x.title = e.target.value.toUpperCase()))}
    />
  );
  if (style.heading === "Centered")
    return (
      <div className="sp-head centered">
        <span>SECTION {s.number} -</span>
        {title}
      </div>
    );
  return (
    <div className={`sp-head ${style.heading === "Banner" ? "banner" : "ruled"}`}>
      <span className="sp-head-num">{s.number}</span>
      {title}
    </div>
  );
}

function Generated({ s, state }: { s: SpecSection; state: SpecState }) {
  const f = state.front;
  const book = state.book!;
  const party = (p: {
    role: string;
    company: string;
    name: string;
    address: string;
    phone: string;
    email: string;
  }) => (
    <div key={p.role + p.company} className="sp-party">
      <b>{p.role.toUpperCase()}</b>
      {[p.company, p.name, p.address, p.phone, p.email].filter(Boolean).map((l) => (
        <span key={l}>{l}</span>
      ))}
    </div>
  );
  const hint = (t: string) => <div className="sp-gen-hint">{t}</div>;
  switch (s.kind) {
    case "TitlePage":
      return (
        <div className="sp-titlepage">
          <span className="sp-tp-kicker">PROJECT MANUAL</span>
          <h1>{f.projectName}</h1>
          {f.address && <span>{f.address}</span>}
          {f.projectNumber && <span>Project No. {f.projectNumber}</span>}
          {f.owner && (
            <div className="sp-tp-block">
              <b>OWNER</b>
              <span>{f.owner.company || f.owner.name}</span>
            </div>
          )}
          {f.team[0] && (
            <div className="sp-tp-block">
              <b>{f.team[0].role.toUpperCase()}</b>
              <span>{f.team[0].company || f.team[0].name}</span>
            </div>
          )}
          <div className="sp-tp-block">
            <b>{book.issue.toUpperCase()}</b>
            <span>{book.date}</span>
          </div>
          {hint(
            "Written from Project Info and the manual's issue. Edit them there and here in Properties.",
          )}
        </div>
      );
    case "ProjectDirectory":
    case "SealsPage":
      return (
        <div className="sp-gen">
          {f.owner && s.kind === "ProjectDirectory" && party(f.owner)}
          {f.team.map(party)}
          {!f.owner &&
            f.team.length === 0 &&
            hint("Fill in the client and consultants on the Project Info tab.")}
          {hint(
            s.kind === "SealsPage"
              ? "A seal box prints for each design professional."
              : "Written from Project Info › Client and Consultants.",
          )}
        </div>
      );
    case "Contents": {
      const shown = book.sections.filter((x) => x.included);
      return (
        <div className="sp-gen sp-toc">
          {state.divisions
            .filter((d) => shown.some((x) => divisionOf(x.number) === d.code))
            .map((d) => (
              <div key={d.code}>
                <b>
                  DIVISION {d.code} - {d.title}
                </b>
                {shown
                  .filter((x) => divisionOf(x.number) === d.code)
                  .map((x) => (
                    <div key={x.number} className="sp-toc-line">
                      <span>{x.number}</span>
                      <span>{titleCase(x.title)}</span>
                    </div>
                  ))}
              </div>
            ))}
          {hint("Lists the issued sections, grouped as MasterFormat does.")}
        </div>
      );
    }
    case "DrawingList":
      return (
        <div className="sp-gen sp-toc">
          {f.sheets.map(([n, name]) => (
            <div key={n} className="sp-toc-line">
              <span>{n}</span>
              <span>{name}</span>
            </div>
          ))}
          {hint(
            f.sheets.length
              ? "Written from the project's sheets."
              : "Sheets you add to the project list here.",
          )}
        </div>
      );
    default:
      return null;
  }
}

export function SpecsView() {
  const { state, draft } = useSpecs();
  useEffect(() => () => void commitSection(), []);
  const book = state?.book ?? null;
  const style = state?.styles.find((s) => s.id === book?.style) ?? state?.styles[0];
  if (!state || !book || !style) {
    return (
      <section className="workspace std-workspace">
        <div className="std-canvas sp-canvas">
          <GenerateCard state={state} />
        </div>
      </section>
    );
  }
  const s = draft;
  return (
    <section className="workspace std-workspace">
      <div className="tabs" role="tablist" aria-label="Open views">
        <div className="tab active std-tab" role="tab" aria-selected>
          <span className="tab-label">
            <span className="tab-kind">SPECIFICATIONS</span>
            {s ? `${s.number} ${titleCase(s.title)}` : "Project Manual"}
          </span>
        </div>
      </div>
      <div className="std-canvas sp-canvas">
        {s && (
          <div
            className={`sp-page${s.included ? "" : " excluded"}`}
            style={{ fontFamily: FONTS[style.font], fontSize: `${style.size}pt` }}
          >
            <div className="sp-page-hdr">
              <span>{state.front.projectName}</span>
              <span>{book.issue}</span>
            </div>
            {!s.included && (
              <div className="sp-excluded">Excluded: this section won't be issued.</div>
            )}
            {s.kind === "TitlePage" ? null : s.kind !== "ThreePart" && s.kind !== "Document" ? (
              <div className="sp-head centered static">
                SECTION {s.number} - {s.title}
              </div>
            ) : (
              <SectionHeading s={s} style={style} />
            )}
            {s.kind === "ThreePart" || s.kind === "Document" ? (
              <>
                {s.parts.map((part, pi) => {
                  let n = 0;
                  return (
                    <div key={pi} className="sp-part">
                      {s.kind === "ThreePart" && (
                        <div className="sp-part-head">
                          {style.numbering === "Decimal"
                            ? `${pi + 1}  ${part.title}`
                            : `PART ${pi + 1} - ${part.title}`}
                        </div>
                      )}
                      {part.articles.map((a, ai) => {
                        n += 1;
                        return (
                          <ArticleEditor
                            key={ai}
                            article={a}
                            pi={pi}
                            ai={ai}
                            numbering={style.numbering}
                            partNo={pi + 1}
                            count={n}
                            bold={style.boldArticles}
                          />
                        );
                      })}
                      <button
                        className="sp-add-article"
                        onClick={() => {
                          want(`p${pi}a${part.articles.length}i0`);
                          editDraft((x) =>
                            x.parts[pi]!.articles.push({
                              title: "NEW ARTICLE",
                              paragraphs: [{ level: 0, text: "" }],
                            }),
                          );
                        }}
                      >
                        + Add article to {part.title ? `Part ${pi + 1}` : "the document"}
                      </button>
                    </div>
                  );
                })}
                <div className="sp-end">END OF SECTION {s.number}</div>
              </>
            ) : (
              <Generated s={s} state={state} />
            )}
            <div className="sp-page-ftr">
              <span>{titleCase(s.title)}</span>
              <span>{s.number} - 1</span>
            </div>
          </div>
        )}
        <EditSpecsButton />
      </div>
    </section>
  );
}

function GenerateCard({ state }: { state: SpecState | null }) {
  const picked = state?.library.filter((l) => l.picked).length ?? 0;
  return (
    <div className="sp-start">
      <span className="std-eyebrow">SPECIFICATIONS</span>
      <h2>Project Manual</h2>
      <p>
        Rufplan reads the model and Project Info — its walls, roofs, doors and windows, rooms,
        finishes, stairs and systems — and assembles the MasterFormat sections the building calls
        for, in CSI three-part format, ready to edit.
      </p>
      <div className="sp-start-stats">
        <div>
          <strong>{state?.library.length ?? "—"}</strong>
          <span>sections in the library</span>
        </div>
        <div>
          <strong>{picked || "—"}</strong>
          <span>called for by this project</span>
        </div>
        <div>
          <strong>{state?.styles.length ?? "—"}</strong>
          <span>section styles</span>
        </div>
      </div>
      <button className="pi-btn" onClick={() => useSpecs.setState({ dialog: "generate" })}>
        Generate Project Manual
      </button>
    </div>
  );
}

// ---- Properties ----

function StyleThumb({ s }: { s: SpecStyle }) {
  return (
    <div className={`sp-thumb ${s.heading.toLowerCase()} ${s.font.toLowerCase()}`} aria-hidden>
      <div className="h" />
      {s.content === "Narrative" ? (
        <>
          <div className="b l1" />
          <div className="b l1" />
          <div className="b l1 s" />
        </>
      ) : (
        <>
          <div className="a" />
          <div className="b l1" />
          <div className="b l2" />
          <div className={`b l2${s.content === "Short" ? " s" : ""}`} />
          <div className="a" />
          <div className="b l1 s" />
        </>
      )}
    </div>
  );
}

export function SpecsProperties() {
  const { state, draft, selected } = useSpecs();
  const [libraryText, setLibraryText] = useState<string | null>(null);
  const book = state?.book ?? null;
  if (!state) return <aside className="panel properties" aria-label="Properties" />;
  const s = draft;
  const lib = state.library.find((l) => l.number === selected);
  const setSettings = (style: string, issue: string, date: string) =>
    void apply(() => ipc.specSetSettings(style, issue, date));
  const row = (label: string, value: ReactNode) => (
    <div className="std-prop">
      <span>{label}</span>
      <span>{value}</span>
    </div>
  );
  return (
    <aside className="panel properties std-props sp-props" aria-label="Properties">
      <div className="panel-title">Properties</div>
      <div className="std-props-body">
        {s && (
          <>
            <div className="std-props-title">
              <span>SECTION</span>
              <strong>{titleCase(s.title)}</strong>
            </div>
            <div className="std-prop">
              <span>Number</span>
              <input
                aria-label="Section number"
                className={/^\d\d \d\d \d\d(\.\d\d)?$/.test(s.number) ? "" : "bad"}
                value={s.number}
                disabled={s.kind !== "ThreePart" && s.kind !== "Document"}
                onChange={(e) => editDraft((x) => void (x.number = e.target.value))}
              />
            </div>
            {row("Issued", s.included ? "Yes" : <span className="muted">Excluded</span>)}
            {row(
              "Source",
              s.origin === "Claude"
                ? "Claude"
                : s.origin === "Custom"
                  ? "Written here"
                  : s.edited
                    ? "Library, edited"
                    : "Library",
            )}
            {lib?.reason && row("Called for by", <span className="muted">{lib.reason}</span>)}
            {row(
              "Paragraphs",
              String(
                s.parts.reduce(
                  (n, p) => n + p.articles.reduce((m, a) => m + a.paragraphs.length, 0),
                  0,
                ),
              ),
            )}
            {s.edited && lib && (
              <button
                className="std-choices-btn sp-revert"
                onClick={async () => {
                  const fresh = await ipc.specLibrarySection(s.number);
                  if (fresh) {
                    setLibraryText(s.number);
                    editDraft((x) => {
                      x.parts = fresh.parts;
                      x.title = fresh.title;
                    });
                  }
                }}
              >
                Revert to Library Text
              </button>
            )}
            {libraryText === s.number && (
              <div className="sp-muted">Reverted — Undo (Ctrl+Z) brings your text back.</div>
            )}
            <div className="sp-help">
              Enter: new paragraph · Tab / Shift+Tab: indent · Backspace at start: join · Alt+↑↓:
              move
            </div>
          </>
        )}
        {book && (
          <>
            <div className="std-section">MANUAL</div>
            <div className="std-prop">
              <span>Issue</span>
              <input
                aria-label="Issue"
                list="sp-issues"
                defaultValue={book.issue}
                key={`issue-${book.issue}`}
                onBlur={(e) =>
                  e.target.value !== book.issue &&
                  setSettings(book.style, e.target.value, book.date)
                }
              />
              <datalist id="sp-issues">
                {ISSUES.map((i) => (
                  <option key={i} value={i} />
                ))}
              </datalist>
            </div>
            <div className="std-prop">
              <span>Date</span>
              <input
                aria-label="Issue date"
                type="date"
                value={book.date}
                onChange={(e) => setSettings(book.style, book.issue, e.target.value)}
              />
            </div>
            <div className="std-section">SECTION STYLE</div>
            <div className="sp-styles" role="radiogroup" aria-label="Section style">
              {state.styles.map((st) => (
                <button
                  key={st.id}
                  role="radio"
                  aria-checked={book.style === st.id}
                  className={`sp-style${book.style === st.id ? " on" : ""}`}
                  title={st.description}
                  onClick={() => setSettings(st.id, book.issue, book.date)}
                >
                  <StyleThumb s={st} />
                  <span>{st.name}</span>
                </button>
              ))}
            </div>
            <div className="sp-muted">
              {state.styles.find((x) => x.id === book.style)?.description}
            </div>
            <Coordination state={state} />
          </>
        )}
      </div>
    </aside>
  );
}

function Coordination({ state }: { state: SpecState }) {
  const refs = state.references;
  const title = (n: string) => state.library.find((l) => l.number === n)?.title ?? "";
  const issues = refs.length + state.unindicated.length + state.indicated.length;
  return (
    <>
      <div className="std-section">
        COORDINATION <span className={`sp-count${issues ? " warn" : ""}`}>{issues || "✓"}</span>
      </div>
      {issues === 0 && (
        <div className="sp-muted">
          Every cross-reference resolves, and the manual matches the model.
        </div>
      )}
      {state.indicated.length > 0 && (
        <div className="sp-coord">
          <b>The model calls for</b>
          {state.indicated.map((n) => (
            <div key={n} className="sp-coord-row">
              <span>
                {n} {titleCase(title(n))}
              </span>
              <button onClick={() => void apply(() => ipc.specAddLibrary([n]))}>Add</button>
            </div>
          ))}
        </div>
      )}
      {refs.length > 0 && (
        <div className="sp-coord">
          <b>Referenced but not issued</b>
          {refs.map((r) => (
            <div key={r.to} className="sp-coord-row">
              <span title={`Referenced in ${r.from.join(", ")}`}>
                {r.to} {r.title ? titleCase(r.title) : ""}
                <em>
                  {" "}
                  in {r.from[0]}
                  {r.from.length > 1 ? ` +${r.from.length - 1}` : ""}
                </em>
              </span>
              {r.excluded ? (
                <button onClick={() => void apply(() => ipc.specSetIncluded([r.to], true))}>
                  Include
                </button>
              ) : r.title ? (
                <button onClick={() => void apply(() => ipc.specAddLibrary([r.to]))}>Add</button>
              ) : (
                <button onClick={() => void select(r.from[0]!)}>Open</button>
              )}
            </div>
          ))}
        </div>
      )}
      {state.unindicated.length > 0 && (
        <div className="sp-coord">
          <b>No longer in the model</b>
          {state.unindicated.map((n) => (
            <div key={n} className="sp-coord-row">
              <span>
                {n} {titleCase(title(n))}
              </span>
              <button onClick={() => void apply(() => ipc.specSetIncluded([n], false))}>
                Exclude
              </button>
            </div>
          ))}
        </div>
      )}
    </>
  );
}

// ---- Dialogs ----

function Modal({
  title,
  children,
  onClose,
  wide,
}: {
  title: string;
  children: ReactNode;
  onClose: () => void;
  wide?: boolean;
}) {
  useEffect(() => {
    const k = (e: KeyboardEvent) => e.key === "Escape" && onClose();
    window.addEventListener("keydown", k);
    return () => window.removeEventListener("keydown", k);
  }, [onClose]);
  return (
    <div
      className="edit-model-backdrop"
      role="presentation"
      onMouseDown={(e) => e.target === e.currentTarget && onClose()}
    >
      <div
        className={`edit-model sp-modal${wide ? " wide" : ""}`}
        role="dialog"
        aria-modal
        aria-label={title}
      >
        <div className="em-head">
          <div className="em-title">
            <span className="em-name">{title.toUpperCase()}</span>
          </div>
          <button className="em-close" aria-label="Close" onClick={onClose}>
            ×
          </button>
        </div>
        {children}
      </div>
    </div>
  );
}

function GenerateDialog() {
  const state = useSpecs((s) => s.state);
  const book = state?.book ?? null;
  const [style, setStyle] = useState(book?.style ?? "csi-classic");
  const [issue, setIssue] = useState(book?.issue ?? "Issued for Permit");
  const [date, setDate] = useState(book?.date || state?.today || "");
  const close = () => useSpecs.setState({ dialog: null });
  const picked = state?.library.filter((l) => l.picked) ?? [];
  const edited = book?.sections.filter((s) => s.edited).length ?? 0;
  return (
    <Modal
      title={book ? "Regenerate Project Manual" : "Generate Project Manual"}
      onClose={close}
      wide
    >
      <div className="sp-gen-body">
        <div className="sp-gen-sum">
          <strong>{picked.length} sections</strong> called for by the model and Project Info, across{" "}
          {new Set(picked.map((p) => divisionOf(p.number))).size} divisions.
        </div>
        <div className="sp-styles big" role="radiogroup" aria-label="Section style">
          {state?.styles.map((st) => (
            <button
              key={st.id}
              role="radio"
              aria-checked={style === st.id}
              className={`sp-style${style === st.id ? " on" : ""}`}
              onClick={() => setStyle(st.id)}
            >
              <StyleThumb s={st} />
              <span>{st.name}</span>
              <em>{st.description}</em>
            </button>
          ))}
        </div>
        <div className="pi-fields">
          <label className="pi-field">
            <span>Issue</span>
            <input list="sp-issues-g" value={issue} onChange={(e) => setIssue(e.target.value)} />
            <datalist id="sp-issues-g">
              {ISSUES.map((i) => (
                <option key={i} value={i} />
              ))}
            </datalist>
          </label>
          <label className="pi-field">
            <span>Date</span>
            <input type="date" value={date} onChange={(e) => setDate(e.target.value)} />
          </label>
        </div>
        {book && (
          <div className="pi-warn">
            This replaces the current manual ({book.sections.length} sections
            {edited ? `, ${edited} edited` : ""}). Undo brings it back. To add only what&apos;s new,
            use Update from Model.
          </div>
        )}
      </div>
      <div className="em-foot">
        <span className="em-hint">
          Placeholders fill from Project Info: project, owner, architect, door and window types…
        </span>
        <button
          className="em-apply"
          onClick={async () => {
            await commitSection();
            const ok = await apply(() => ipc.specGenerate(style, issue, date));
            if (ok) {
              useSpecs.setState({ dialog: null, selected: null, dirty: false });
              await refreshSpecs();
            }
          }}
        >
          {book ? "REGENERATE" : "GENERATE"}
        </button>
      </div>
    </Modal>
  );
}

function AddDialog() {
  const state = useSpecs((s) => s.state);
  const [q, setQ] = useState("");
  const [picked, setPicked] = useState<string[]>([]);
  const [onlyCalled, setOnlyCalled] = useState(false);
  const [showIn, setShowIn] = useState(false);
  const close = () => useSpecs.setState({ dialog: null });
  const rows = (state?.library ?? []).filter(
    (l) =>
      (!onlyCalled || l.picked) &&
      (showIn || !l.inBook) &&
      (!q || l.number.includes(q) || l.title.toLowerCase().includes(q.toLowerCase())),
  );
  const toggle = (n: string) =>
    setPicked((p) => (p.includes(n) ? p.filter((x) => x !== n) : [...p, n]));
  return (
    <Modal title="Add Sections" onClose={close} wide>
      <div className="sp-add-top">
        <input
          autoFocus
          aria-label="Search the library"
          placeholder={`Search ${state?.library.length ?? ""} sections — number or title…`}
          value={q}
          onChange={(e) => setQ(e.target.value)}
        />
        <label className="sp-check">
          <input
            type="checkbox"
            checked={onlyCalled}
            onChange={(e) => setOnlyCalled(e.target.checked)}
          />
          Only what the model calls for
        </label>
        <label className="sp-check">
          <input type="checkbox" checked={showIn} onChange={(e) => setShowIn(e.target.checked)} />
          Show sections in the manual
        </label>
      </div>
      <div className="sp-lib" role="list">
        {state?.divisions
          .filter((d) => rows.some((r) => divisionOf(r.number) === d.code))
          .map((d) => (
            <div key={d.code}>
              <div className="sp-lib-div">
                DIVISION {d.code} — {d.title}
              </div>
              {rows
                .filter((r) => divisionOf(r.number) === d.code)
                .map((r) => (
                  <label
                    key={r.number}
                    className={`sp-lib-row${r.inBook ? " in" : ""}`}
                    role="listitem"
                  >
                    <input
                      type="checkbox"
                      disabled={r.inBook}
                      checked={r.inBook || picked.includes(r.number)}
                      onChange={() => toggle(r.number)}
                    />
                    <span className="sp-num">{r.number}</span>
                    <span className="sp-title">{titleCase(r.title)}</span>
                    {r.inBook ? (
                      <span className="sp-tag">IN MANUAL</span>
                    ) : r.picked ? (
                      <span className="sp-tag rec" title={r.reason ?? ""}>
                        RECOMMENDED
                      </span>
                    ) : null}
                    <span className="sp-paras">{r.paragraphs} ¶</span>
                  </label>
                ))}
            </div>
          ))}
      </div>
      <div className="em-foot">
        <span className="em-hint">
          {picked.length ? `${picked.length} selected` : "Pick the sections to add"}
        </span>
        <button
          className="em-apply"
          disabled={!picked.length}
          onClick={async () => {
            const ok = await apply(() => ipc.specAddLibrary(picked));
            if (ok) {
              close();
              await refreshSpecs();
              await select(picked.slice().sort()[0]!);
            }
          }}
        >
          ADD {picked.length || ""}
        </button>
      </div>
    </Modal>
  );
}

function CustomDialog() {
  const [number, setNumber] = useState("");
  const [title, setTitle] = useState("");
  const close = () => useSpecs.setState({ dialog: null });
  const ok = /^\d\d \d\d \d\d(\.\d\d)?$/.test(number.trim()) && title.trim().length > 0;
  return (
    <Modal title="New Section" onClose={close}>
      <div className="sp-gen-body">
        <div className="pi-fields">
          <label className="pi-field">
            <span>MasterFormat Number</span>
            <input
              autoFocus
              placeholder="09 72 00"
              value={number}
              onChange={(e) => setNumber(e.target.value)}
            />
          </label>
          <label className="pi-field">
            <span>Title</span>
            <input
              placeholder="Wall Coverings"
              value={title}
              onChange={(e) => setTitle(e.target.value)}
            />
          </label>
        </div>
        <div className="sp-muted">
          It starts with the usual articles in all three parts, ready to fill in — or ask Claude to
          write it.
        </div>
      </div>
      <div className="em-foot">
        <span className="em-hint">Numbers look like 09 72 00 (or 23 74 16.11)</span>
        <button
          className="em-apply"
          disabled={!ok}
          onClick={async () => {
            const n = number.trim();
            if (await apply(() => ipc.specAddCustom(n, title))) {
              close();
              await refreshSpecs();
              await select(n);
            }
          }}
        >
          CREATE
        </button>
      </div>
    </Modal>
  );
}

export function SpecsDialogs() {
  const dialog = useSpecs((s) => s.dialog);
  const editOpen = useSpecs((s) => s.editOpen);
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!(e.metaKey || e.ctrlKey) || e.key.toLowerCase() !== "k") return;
      if (!useSpecs.getState().state?.book) return;
      e.preventDefault();
      e.stopPropagation();
      useSpecs.setState((s) => ({ editOpen: !s.editOpen }));
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, []);
  return (
    <>
      {dialog === "generate" && <GenerateDialog />}
      {dialog === "add" && <AddDialog />}
      {dialog === "custom" && <CustomDialog />}
      {editOpen && <EditSpecsDialog />}
    </>
  );
}

export function SpecsStatus() {
  const { state, saving, dirty, note: n } = useSpecs();
  const book = state?.book;
  const style = state?.styles.find((s) => s.id === book?.style);
  return (
    <footer className="statusbar">
      <span className="status-prompt">
        {n ??
          (book
            ? "Click a section to edit it. Changes save as you type and undo like any edit. Ctrl+K asks Claude."
            : "Generate the project manual from the model to start.")}
      </span>
      <span className="status-cursor std-status-total">
        {saving
          ? "Saving…"
          : dirty
            ? "Editing…"
            : book
              ? `${style?.name ?? ""} · ${book.sections.filter((s) => s.included).length} sections`
              : ""}
      </span>
    </footer>
  );
}

// ---- Edit Specs with Claude ----

function EditSpecsButton() {
  const open = useSpecs((s) => s.editOpen);
  const has = useSpecs((s) => !!s.state?.book);
  if (!has) return null;
  return (
    <button
      className="edit-model-btn sp-edit-btn"
      title="Edit specs with Claude (Ctrl+K)"
      aria-label="Edit specs with Claude"
      aria-expanded={open}
      onClick={() => useSpecs.setState({ editOpen: !open })}
    >
      {sparkle("#29B5E8")}
      <span>EDIT SPECS</span>
    </button>
  );
}

function suggestions(s: SpecSection | null): string[] {
  const general = [
    "Add a section for wall coverings in the bathrooms",
    "Change every warranty in Division 07 and 08 to 5 years",
    "Tailor Division 01 for an occupied renovation",
  ];
  if (!s || (s.kind !== "ThreePart" && s.kind !== "Document")) return general;
  const t = titleCase(s.title);
  return [
    `Tighten ${t} for a small residential project`,
    `Add a basis-of-design product to ${t}`,
    `Add a mockup requirement to ${t}`,
    general[0]!,
  ];
}

function ChangeCard({ c }: { c: SpecChange }) {
  const [open, setOpen] = useState(c.kind === "changed" && c.lines.length <= 24);
  return (
    <div className={`sp-change ${c.kind}`}>
      <button className="sp-change-head" onClick={() => setOpen(!open)} aria-expanded={open}>
        <span className={`sp-kind ${c.kind}`}>{c.kind.toUpperCase()}</span>
        <span className="sp-num">{c.number}</span>
        <span className="sp-title">{titleCase(c.title)}</span>
        {(c.added > 0 || c.removed > 0) && (
          <span className="sp-delta">
            <b className="plus">+{c.added}</b> <b className="minus">−{c.removed}</b>
          </span>
        )}
      </button>
      {open && c.lines.length > 0 && (
        <div className="sp-diff">
          {c.lines.map((l, i) => {
            const level = l.text.match(/^>*/)?.[0].length ?? 0;
            const head = l.text.startsWith("#");
            return (
              <div
                key={i}
                className={`sp-dl ${l.sign === "+" ? "add" : l.sign === "-" ? "del" : "ctx"}${head ? " head" : ""}`}
              >
                <span className="sp-dl-sign">{l.sign}</span>
                <span style={{ paddingLeft: `${level * 14}px` }}>
                  {l.text.replace(/^[>#]+ ?(PART: )?/, "")}
                </span>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}

function EditSpecsDialog() {
  const { log, draft, state } = useSpecs();
  const undoLabel = useAppStore((s) => s.app?.undo ?? null);
  const redoLabel = useAppStore((s) => s.app?.redo ?? null);
  const [text, setText] = useState("");
  const [scope, setScope] = useState<"section" | "book">(
    draft && !draft.kind.match(/Page|Contents|List|Directory/) ? "section" : "book",
  );
  const [pending, setPending] = useState<{ plan: SpecEditPlan; prompt: string } | null>(null);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const ask = useRef(0);
  const close = () => {
    ask.current++;
    useSpecs.setState({ editOpen: false });
  };
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      e.stopPropagation();
      close();
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, []);
  const focus = scope === "section" ? (draft?.number ?? null) : null;
  const preview = async (prompt: string) => {
    const p = prompt.trim();
    if (!p || busy) return;
    await commitSection();
    const n = ++ask.current;
    setBusy(true);
    setError("");
    setPending(null);
    try {
      const plan = await ipc.specEditPreview(p, focus);
      if (n !== ask.current) return;
      if (plan.error) setError(plan.error);
      else if (plan.changes.length === 0)
        setError(plan.edit.message || "Claude didn't change anything.");
      else setPending({ plan, prompt: p });
    } catch (e) {
      if (n === ask.current) setError(errorMessage(e));
    } finally {
      if (n === ask.current) setBusy(false);
    }
  };
  const applyPending = async () => {
    if (!pending) return;
    const ok = await apply(() => ipc.specEditApply(pending.plan.edit));
    if (!ok) return;
    const summary = pending.plan.edit.summary || "Edit specs";
    useSpecs.setState((s) => ({
      log: [
        { prompt: pending.prompt, summary, label: `Edit Specs: ${summary}`, undone: false },
        ...s.log,
      ],
      dirty: false,
    }));
    setPending(null);
    setText("");
    await refreshSpecs();
    const first = pending.plan.changes.find((c) => c.kind !== "removed");
    if (first) await select(first.number);
  };
  const isUndone = (l: LogEntry) =>
    undoLabel === l.label ? false : redoLabel === l.label ? true : l.undone;
  const toggle = async (i: number) => {
    const redo = isUndone(log[i]!);
    if (!(await apply(() => (redo ? ipc.redo() : ipc.undo())))) return;
    useSpecs.setState((s) => ({
      log: s.log.map((x, j) => (j === i ? { ...x, undone: !redo } : x)),
      dirty: false,
    }));
    await refreshSpecs();
  };
  const changes = pending?.plan.changes ?? [];
  return (
    <div
      className="edit-model-backdrop"
      role="presentation"
      onMouseDown={(e) => e.target === e.currentTarget && close()}
    >
      <div
        className="edit-model sp-edit"
        role="dialog"
        aria-modal
        aria-label="Edit specs with Claude"
      >
        <div className="em-head">
          <div className="em-title">
            {sparkle("#29B5E8")}
            <span className="em-name">EDIT SPECS</span>
            <span className="em-sub">with Claude · {state?.front.projectName}</span>
          </div>
          <button className="em-close" aria-label="Close" onClick={close}>
            ×
          </button>
        </div>
        <div className="sp-scope" role="radiogroup" aria-label="Scope">
          <button
            role="radio"
            aria-checked={scope === "section"}
            className={scope === "section" ? "on" : undefined}
            disabled={!draft}
            onClick={() => setScope("section")}
          >
            {draft ? `${draft.number} ${titleCase(draft.title)}` : "This section"}
          </button>
          <button
            role="radio"
            aria-checked={scope === "book"}
            className={scope === "book" ? "on" : undefined}
            onClick={() => setScope("book")}
          >
            Whole manual
          </button>
        </div>
        {log.length > 0 && (
          <ul className="em-log" aria-label="History">
            {log.map((l, i) => {
              const undone = isUndone(l);
              const can = undone ? redoLabel === l.label : undoLabel === l.label;
              return (
                <li key={i} className={undone ? "undone" : undefined}>
                  <div className="em-log-text">
                    <span className="em-log-prompt">“{l.prompt}”</span>
                    <span className="em-log-summary">{l.summary}</span>
                  </div>
                  <button className="em-log-undo" disabled={!can} onClick={() => void toggle(i)}>
                    {undone ? "REDO" : "UNDO"}
                  </button>
                </li>
              );
            })}
          </ul>
        )}
        {pending && (
          <div className="em-preview sp-preview" aria-label="Preview">
            <span className="em-eyebrow">
              PREVIEW · {changes.length} SECTION{changes.length === 1 ? "" : "S"}
            </span>
            <strong className="em-summary">{pending.plan.edit.summary}</strong>
            {pending.plan.edit.message && (
              <span className="sp-muted">{pending.plan.edit.message}</span>
            )}
            <div className="sp-changes">
              {changes.map((c) => (
                <ChangeCard key={c.number + c.kind} c={c} />
              ))}
            </div>
          </div>
        )}
        {error && (
          <div className="em-error" role="alert">
            {error}
          </div>
        )}
        {!pending && !text && !busy && (
          <div className="em-chips">
            {suggestions(scope === "section" ? draft : null).map((t) => (
              <button
                key={t}
                className="em-chip"
                onClick={() => {
                  setText(t);
                  void preview(t);
                }}
              >
                {t}
              </button>
            ))}
          </div>
        )}
        <div className="em-input">
          <textarea
            aria-label="Describe a change to the specifications"
            rows={4}
            autoFocus
            value={text}
            placeholder={
              scope === "section"
                ? "Describe a change to this section — specify 5/8 inch Type X throughout, add a mockup, name a basis-of-design product…"
                : "Describe a change to the manual — add a section, change warranties across divisions, tailor Division 01…"
            }
            onChange={(e) => {
              setText(e.target.value);
              setError("");
              if (pending || busy) {
                ask.current++;
                setPending(null);
                setBusy(false);
              }
            }}
            onKeyDown={(e) => {
              if (e.key === "Enter" && !e.shiftKey) {
                e.preventDefault();
                if (pending) void applyPending();
                else void preview(text);
              }
            }}
          />
        </div>
        <div className="em-foot">
          <span className="em-hint">Enter to preview · Shift+Enter new line · Esc to close</span>
          {pending ? (
            <div className="em-actions">
              <button className="em-cancel" onClick={() => setPending(null)}>
                CANCEL
              </button>
              <button className="em-apply" onClick={() => void applyPending()}>
                APPLY
              </button>
            </div>
          ) : (
            <button
              className="em-send"
              disabled={!text.trim() || busy}
              onClick={() => void preview(text)}
            >
              {busy ? "CLAUDE IS WRITING…" : "PREVIEW CHANGE"}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
