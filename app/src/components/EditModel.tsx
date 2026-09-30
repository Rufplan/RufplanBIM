import { useEffect, useRef, useState } from "react";
import type { EditPlan } from "../bindings/EditPlan";
import { apply } from "../fileActions";
import { errorMessage, ipc } from "../ipc";
import { activeViewInfo, useAppStore, type EditLogEntry } from "../store";

type UiStore = ReturnType<typeof useAppStore.getState>;

// Edit Model with Claude (ADR-050): a floating button over model views opens a
// prompt; Claude's structured edit is previewed (the elements it changes highlighted) and
// applied as one undo step. Rust checks and makes every change.

const SUGGESTIONS = [
  "Set all doors to 3'-0\" W",
  "Add a 12' x 10' office off the east side with a door and window",
  "Dimension the south wall",
  "Add a note: VERIFY IN FIELD",
];

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

/** Where Edit Model is offered: plans (floor, ceiling, site), elevations, sections and 3D;
 * not sheets, schedules or the Standards tab. */
export function editModelAllowed(s: UiStore): boolean {
  if (!s.app || s.app.sketch || s.activeViewport) return false;
  // Tabs that replace the views (Specifications has its own Edit Specs).
  if (["Standards", "Project Info", "Specifications"].includes(s.ribbonTab)) return false;
  const t = activeViewInfo(s)?.viewType;
  return (
    t === "Plan" || t === "CeilingPlan" || t === "Elevation" || t === "Section" || t === "ThreeD"
  );
}

export function useEditModelAllowed(): boolean {
  return useAppStore(editModelAllowed);
}

/** The EDIT MODEL pill, bottom right of the view. */
export function EditModelButton() {
  const allowed = useEditModelAllowed();
  const open = useAppStore((s) => s.editModel.open);
  const setEditModel = useAppStore((s) => s.setEditModel);
  if (!allowed) return null;
  return (
    <button
      className="edit-model-btn"
      title="Edit model with Claude (⌘K)"
      aria-label="Edit model with Claude"
      aria-expanded={open}
      onClick={() => setEditModel({ open: !open })}
    >
      {sparkle("#29B5E8")}
      <span>EDIT MODEL</span>
    </button>
  );
}

/** The Edit Model dialog and its ⌘K / Ctrl+K shortcut. It closes (dropping any preview)
 * when the active view becomes one it isn't offered in. */
export function EditModelDialog() {
  const allowed = useEditModelAllowed();
  const open = useAppStore((s) => s.editModel.open);
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!(e.metaKey || e.ctrlKey) || e.key.toLowerCase() !== "k") return;
      const s = useAppStore.getState();
      if (!editModelAllowed(s)) return;
      e.preventDefault();
      e.stopPropagation();
      s.setEditModel({ open: !s.editModel.open });
    };
    window.addEventListener("keydown", onKey, true);
    const unsub = useAppStore.subscribe((s) => {
      if (s.editModel.open && !editModelAllowed(s)) s.setEditModel({ open: false });
    });
    return () => {
      window.removeEventListener("keydown", onKey, true);
      unsub();
    };
  }, []);
  // Mounted only while open, so each opening starts clean.
  return open && allowed ? <Dialog /> : null;
}

function Dialog() {
  const log = useAppStore((s) => s.editModel.log);
  const setEditModel = useAppStore((s) => s.setEditModel);
  const setHighlight = useAppStore((s) => s.setHighlight);
  const project = useAppStore((s) => s.app?.projectName ?? "Project");
  const undoLabel = useAppStore((s) => s.app?.undo ?? null);
  const redoLabel = useAppStore((s) => s.app?.redo ?? null);
  const [text, setText] = useState("");
  const [pending, setPending] = useState<{ plan: EditPlan; prompt: string } | null>(null);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  // Answers that come back after the prompt changed or the dialog closed are dropped.
  const ask = useRef(0);

  const discard = () => {
    ask.current++;
    setPending(null);
    setBusy(false);
    setHighlight([]);
  };
  const close = () => {
    discard();
    setError("");
    setEditModel({ open: false });
  };

  // Esc closes it; closing (however it happens) drops the highlight.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      e.stopPropagation();
      ask.current++;
      useAppStore.getState().setEditModel({ open: false });
    };
    window.addEventListener("keydown", onKey, true);
    return () => {
      window.removeEventListener("keydown", onKey, true);
      useAppStore.getState().setHighlight([]);
    };
  }, []);

  const context = () => {
    const s = useAppStore.getState();
    return { view: activeViewInfo(s)?.id ?? "", selection: s.selection };
  };

  const preview = async (prompt: string) => {
    const p = prompt.trim();
    if (!p || busy) return;
    const n = ++ask.current;
    setBusy(true);
    setError("");
    setPending(null);
    setHighlight([]);
    try {
      const { view, selection } = context();
      const plan = await ipc.modelEditPreview(p, view, selection);
      if (n !== ask.current) return;
      if (plan.preview) {
        setPending({ plan, prompt: p });
        setHighlight(plan.preview.ids);
      } else {
        setError(plan.error ?? "Claude couldn't turn that into a model change.");
      }
    } catch (e) {
      if (n === ask.current) setError(errorMessage(e));
    } finally {
      if (n === ask.current) setBusy(false);
    }
  };

  const applyPending = async () => {
    if (!pending?.plan.preview) return;
    const { view, selection } = context();
    const done = await apply(() => ipc.modelEditApply(pending.plan.edit, view, selection));
    if (!done) return;
    const summary = pending.plan.preview.summary;
    setEditModel({
      log: [
        { prompt: pending.prompt, summary, label: `Edit model: ${summary}`, undone: false },
        ...useAppStore.getState().editModel.log,
      ],
    });
    setPending(null);
    setHighlight([]);
    setText("");
  };

  // An entry is undone when its step is next to redo (Ctrl+Z counts too).
  const isUndone = (l: EditLogEntry) =>
    undoLabel === l.label ? false : redoLabel === l.label ? true : l.undone;
  const toggleEntry = async (i: number) => {
    const redo = isUndone(log[i]!);
    const ok = await apply(() => (redo ? ipc.redo() : ipc.undo()));
    if (!ok) return;
    setEditModel({
      log: useAppStore
        .getState()
        .editModel.log.map((x, j) => (j === i ? { ...x, undone: !redo } : x)),
    });
  };

  const p = pending?.plan.preview ?? null;
  return (
    <div
      className="edit-model-backdrop"
      role="presentation"
      onMouseDown={(e) => e.target === e.currentTarget && close()}
    >
      <div className="edit-model" role="dialog" aria-modal aria-label="Edit model with Claude">
        <div className="em-head">
          <div className="em-title">
            {sparkle("#29B5E8")}
            <span className="em-name">EDIT MODEL</span>
            <span className="em-sub">with Claude · {project}</span>
          </div>
          <button className="em-close" aria-label="Close" onClick={close}>
            ×
          </button>
        </div>

        {log.length > 0 && (
          <ul className="em-log" aria-label="History">
            {log.map((l, i) => {
              // Only the step at the top of the undo (or redo) stack can go from here.
              const undone = isUndone(l);
              const can = undone ? redoLabel === l.label : undoLabel === l.label;
              return (
                <li key={i} className={undone ? "undone" : undefined}>
                  <div className="em-log-text">
                    <span className="em-log-prompt">“{l.prompt}”</span>
                    <span className="em-log-summary">{l.summary}</span>
                  </div>
                  <button
                    className="em-log-undo"
                    disabled={!can}
                    title={
                      can
                        ? undefined
                        : "Only the latest change undoes from here; use Undo (Ctrl+Z) for earlier ones"
                    }
                    onClick={() => void toggleEntry(i)}
                  >
                    {undone ? "REDO" : "UNDO"}
                  </button>
                </li>
              );
            })}
          </ul>
        )}

        {p && !p.parameter && (
          <div className="em-preview" aria-label="Preview">
            <span className="em-eyebrow">
              PREVIEW · {p.steps.length} STEP{p.steps.length === 1 ? "" : "S"}
              {p.ids.length > 0 && ` · ${p.ids.length} HIGHLIGHTED`}
            </span>
            <strong className="em-summary">{p.summary}</strong>
            <div className="em-grid">
              {p.created.length > 0 && (
                <>
                  <span>Creates</span>
                  <span>{p.created.join(", ")}</span>
                </>
              )}
              {p.changed.length > 0 && (
                <>
                  <span>Changes</span>
                  <span>{p.changed.join(", ")}</span>
                </>
              )}
              {p.deleted.length > 0 && (
                <>
                  <span>Deletes</span>
                  <span className="em-deletes">{p.deleted.join(", ")}</span>
                </>
              )}
            </div>
            <ol className="em-steps" aria-label="Steps">
              {p.steps.map((s, i) => (
                <li key={i}>{s}</li>
              ))}
            </ol>
          </div>
        )}
        {p && p.parameter && (
          <div className="em-preview" aria-label="Preview">
            <span className="em-eyebrow">
              PREVIEW · {p.count} ELEMENT{p.count === 1 ? "" : "S"} HIGHLIGHTED
            </span>
            <div className="em-grid">
              <span>Category</span>
              <span>{p.category}</span>
              <span>Parameter</span>
              <span>{p.parameter}</span>
              <span>Change</span>
              <span>
                <span className="em-from">{p.from}</span> → <strong>{p.to}</strong>
              </span>
              <span>Scope</span>
              <span>{p.scope}</span>
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
            {SUGGESTIONS.map((t) => (
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
            aria-label="Describe a change to the model"
            rows={5}
            autoFocus
            value={text}
            placeholder={
              "Describe any change — add a 12' x 10' office with a door, make all doors 3'-0\", dimension the north wall, add a Roof Deck level…"
            }
            onChange={(e) => {
              setText(e.target.value);
              setError("");
              if (pending || busy) discard();
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
              <button className="em-cancel" onClick={discard}>
                CANCEL
              </button>
              <button className="em-apply" onClick={() => void applyPending()}>
                APPLY
                <svg
                  width="14"
                  height="14"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="2.4"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  aria-hidden
                >
                  <path d="M5 12l5 5 9-10" />
                </svg>
              </button>
            </div>
          ) : (
            <button
              className="em-send"
              disabled={!text.trim() || busy}
              onClick={() => void preview(text)}
            >
              {busy ? "ASKING CLAUDE…" : "PREVIEW CHANGE"}
              {!busy && (
                <svg
                  width="14"
                  height="14"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="2.2"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  aria-hidden
                >
                  <path d="M5 12h14M13 6l6 6-6 6" />
                </svg>
              )}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
