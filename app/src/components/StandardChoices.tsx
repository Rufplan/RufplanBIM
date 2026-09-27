import { useEffect, useRef, useState } from "react";
import type { DisplayList } from "../bindings/DisplayList";
import type { StandardChoice } from "../bindings/StandardChoice";
import { draw, fit } from "../canvas/render";
import { apply } from "../fileActions";
import { ipc } from "../ipc";
import { useAppStore } from "../store";

// The Standards tab's choice pop-up (ADR-048): every way a standard can be set, as a list
// or a grid of previews (the toggle top right), and the one picked is saved as its value.

type Mode = "list" | "grid";
const MODE_KEY = "rufplan.standards.choicesView";

function savedMode(): Mode {
  try {
    return localStorage.getItem(MODE_KEY) === "list" ? "list" : "grid";
  } catch {
    return "grid";
  }
}

/** A choice's drawing (from Rust), fitted into a w × h canvas. */
function Preview({ dl, w, h }: { dl: DisplayList; w: number; h: number }) {
  const ref = useRef<HTMLCanvasElement>(null);
  useEffect(() => {
    const c = ref.current;
    const ctx = c?.getContext("2d");
    if (!c || !ctx) return;
    const k = window.devicePixelRatio || 1;
    c.width = w * k;
    c.height = h * k;
    ctx.setTransform(k, 0, 0, k, 0, 0);
    draw(ctx, dl, fit(dl.bounds, w, h), w, h, { selected: new Set(), hover: null });
  }, [dl, w, h]);
  return <canvas ref={ref} style={{ width: w, height: h }} aria-hidden />;
}

const listIcon = (
  <svg width="16" height="16" viewBox="0 0 16 16" aria-hidden fill="currentColor">
    <path d="M1 2h2v2H1zM5 2h10v2H5zM1 7h2v2H1zM5 7h10v2H5zM1 12h2v2H1zM5 12h10v2H5z" />
  </svg>
);
const gridIcon = (
  <svg width="16" height="16" viewBox="0 0 16 16" aria-hidden fill="currentColor">
    <path d="M1 1h6v6H1zM9 1h6v6H9zM1 9h6v6H1zM9 9h6v6H9z" />
  </svg>
);

export function StandardChoicesDialog() {
  const open = useAppStore((s) => s.choicesOpen);
  const setOpen = useAppStore((s) => s.setChoicesOpen);
  const ui = useAppStore((s) => s.standardsUi);
  const standards = useAppStore((s) => s.standards);
  const cat = standards?.categories.find((c) => c.id === ui.category) ?? null;
  const item = cat?.items[ui.item] ?? null;
  const [mode, setMode] = useState<Mode>(savedMode);
  const [loaded, setLoaded] = useState<{ key: string; choices: StandardChoice[] } | null>(null);
  const [picked, setPicked] = useState<{ key: string; label: string } | null>(null);
  const key = `${ui.category}:${ui.item}`;

  useEffect(() => {
    if (!open) return;
    let live = true;
    ipc.standardsChoices(ui.category, ui.item).then(
      (choices) => live && setLoaded({ key, choices }),
      () => live && setLoaded({ key, choices: [] }),
    );
    return () => {
      live = false;
    };
  }, [open, ui.category, ui.item, key]);

  // Escape closes it wherever the focus is.
  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, setOpen]);

  if (!open || !cat || !item) return null;
  const choices = loaded?.key === key ? loaded.choices : null;
  const selected = picked?.key === key ? picked.label : item.value;
  const close = () => setOpen(false);
  const use = async (label: string) => {
    if (label !== item.value) {
      await apply(() => ipc.standardsSet(cat.id, ui.item, label, null));
    }
    close();
  };
  const pickMode = (m: Mode) => {
    setMode(m);
    try {
      localStorage.setItem(MODE_KEY, m);
    } catch {
      // Remembered per computer when storage is available.
    }
  };
  const custom = item.value !== "" && !choices?.some((c) => c.label === item.value);
  const graphic = choices?.some((c) => c.preview) ?? false;

  return (
    <div
      className="modal-backdrop"
      role="dialog"
      aria-modal
      aria-label={`${item.name} choices`}
      onMouseDown={(e) => e.target === e.currentTarget && close()}
    >
      <div className="modal std-choices">
        <div className="std-choices-head">
          <div>
            <div className="modal-kicker">
              {cat.label} · {cat.group}
            </div>
            <h2>{item.name}</h2>
            <div className="std-choices-current">
              Current:{" "}
              <strong className={item.value ? undefined : "unset"}>
                {item.value || "Not yet set"}
              </strong>
              {custom && <span className="std-chip">CUSTOM</span>}
            </div>
          </div>
          <div className="std-choices-tools">
            <div className="std-mode" role="radiogroup" aria-label="Show choices as">
              <button
                role="radio"
                aria-checked={mode === "list"}
                aria-label="List"
                title="List"
                className={mode === "list" ? "on" : undefined}
                onClick={() => pickMode("list")}
              >
                {listIcon}
              </button>
              <button
                role="radio"
                aria-checked={mode === "grid"}
                aria-label="Grid"
                title="Grid"
                className={mode === "grid" ? "on" : undefined}
                onClick={() => pickMode("grid")}
              >
                {gridIcon}
              </button>
            </div>
            <button className="rp-close" aria-label="Close" onClick={close}>
              ×
            </button>
          </div>
        </div>
        <div className={`std-choices-body ${mode}`} role="listbox" aria-label="Choices">
          {choices === null && <div className="std-empty">Loading…</div>}
          {choices?.length === 0 && (
            <div className="std-empty">No preset choices; type a value in Properties.</div>
          )}
          {choices?.map((c) => {
            const on = c.label === selected;
            const current = c.label === item.value;
            return (
              <div
                key={c.label}
                role="option"
                aria-selected={on}
                tabIndex={0}
                className={`std-choice${on ? " on" : ""}`}
                onClick={() => setPicked({ key, label: c.label })}
                onDoubleClick={() => void use(c.label)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") void use(c.label);
                  if (e.key === " ") {
                    e.preventDefault();
                    setPicked({ key, label: c.label });
                  }
                }}
              >
                {mode === "grid" ? (
                  <div className="std-choice-art">
                    {c.preview ? (
                      <Preview dl={c.preview} w={196} h={112} />
                    ) : (
                      <span className="std-choice-sample">{c.label}</span>
                    )}
                  </div>
                ) : (
                  <>
                    <span className={`std-radio${on ? " on" : ""}`} aria-hidden />
                    {graphic && (
                      <span className="std-choice-thumb">
                        {c.preview && <Preview dl={c.preview} w={64} h={40} />}
                      </span>
                    )}
                  </>
                )}
                <div className="std-choice-text">
                  <span className="std-choice-label">{c.label}</span>
                  <span className="std-choice-detail">{c.detail}</span>
                </div>
                {current && <span className="std-chip">CURRENT</span>}
              </div>
            );
          })}
        </div>
        <div className="std-choices-foot">
          <span className="muted">
            Double-click a choice to use it. For a value of your own, type it in Properties.
          </span>
          <div className="modal-actions">
            <button className="btn-ghost" onClick={close}>
              Cancel
            </button>
            <button
              className="btn-cyan"
              disabled={!selected || selected === item.value}
              onClick={() => void use(selected)}
            >
              Use this
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
