import { useEffect, useMemo, useState } from "react";
import type { Category } from "../bindings/Category";
import { apply } from "../fileActions";
import { ipc } from "../ipc";
import {
  activeShortcuts,
  conflicts,
  DEFAULT_SHORTCUTS,
  loadOverrides,
  saveOverrides,
} from "../shortcuts";
import { activeViewInfo, useAppStore } from "../store";
import { GenerateDialog } from "./GenerateDialog";
import { MaterialBrowser } from "./MaterialBrowser";
import { RenderDialog } from "./RenderDialog";
import { SheetSetsDialog } from "./SheetSetsDialog";
import { WindowLibrary } from "./WindowLibrary";

// Keyboard Shortcuts (KS) and Visibility/Graphics (VV) dialogs (ADR-024).

export function ViewDialogs() {
  const which = useAppStore((s) => s.viewDialog);
  const close = () => useAppStore.getState().setUi({ viewDialog: null });
  if (which === "keyboard") return <KeyboardDialog onClose={close} />;
  if (which === "visibility") return <VisibilityDialog onClose={close} />;
  if (which === "render") return <RenderDialog onClose={close} />;
  if (which === "materials") return <MaterialBrowser onClose={close} />;
  if (which === "generate") return <GenerateDialog onClose={close} />;
  if (which === "windows") return <WindowLibrary onClose={close} />;
  if (which === "sheetSets") return <SheetSetsDialog onClose={close} />;
  return null;
}

function KeyboardDialog({ onClose }: { onClose: () => void }) {
  const [overrides, setOverrides] = useState<Record<string, string[]>>(loadOverrides);
  const [filter, setFilter] = useState("");
  const defs = activeShortcuts(overrides);
  const clash = new Set(conflicts(defs));
  const shown = defs.filter((d) =>
    `${d.label} ${d.group} ${d.keys.join(" ")}`.toLowerCase().includes(filter.toLowerCase()),
  );
  const setKeys = (id: string, text: string) => {
    const keys = text
      .toUpperCase()
      .split(/[\s,]+/)
      .map((k) => k.replace(/[^A-Z]/g, "").slice(0, 2))
      .filter((k) => k.length === 2);
    setOverrides({ ...overrides, [id]: keys });
  };
  return (
    <div className="modal-backdrop" role="dialog" aria-label="Keyboard Shortcuts">
      <div className="modal ks-dialog">
        <h2>Keyboard Shortcuts</h2>
        <p className="muted">
          Revit&apos;s default two-letter shortcuts. Type new keys (comma-separated) to change them.
        </p>
        <input
          className="ks-filter"
          aria-label="Search commands"
          placeholder="Search commands or keys…"
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
        />
        <div className="ks-table" role="table">
          {shown.map((d) => (
            <div className="ks-row" role="row" key={d.id}>
              <span className="ks-group">{d.group}</span>
              <span className="ks-label">{d.label}</span>
              <input
                aria-label={`Keys for ${d.label}`}
                className={d.keys.some((k) => clash.has(k.toUpperCase())) ? "clash" : ""}
                defaultValue={d.keys.join(", ")}
                onBlur={(e) => setKeys(d.id, e.target.value)}
              />
            </div>
          ))}
        </div>
        <div className="modal-actions">
          <button
            className="btn-ghost"
            onClick={() => {
              setOverrides({});
              saveOverrides({});
              onClose();
            }}
          >
            Reset to Revit Defaults
          </button>
          <button className="btn-outline" onClick={onClose}>
            Cancel
          </button>
          <button
            className="btn-cyan"
            onClick={() => {
              // Keep only real changes from the defaults.
              const out: Record<string, string[]> = {};
              for (const d of DEFAULT_SHORTCUTS) {
                const k = overrides[d.id];
                if (k && k.join(",") !== d.keys.join(",")) out[d.id] = k;
              }
              saveOverrides(out);
              onClose();
            }}
          >
            Save
          </button>
        </div>
      </div>
    </div>
  );
}

function VisibilityDialog({ onClose }: { onClose: () => void }) {
  const view = useAppStore((s) => activeViewInfo(s));
  const [present, setPresent] = useState<string[]>([]);
  useEffect(() => {
    if (!view) return;
    let live = true;
    ipc.viewCategories(view.id).then(
      (list) => live && setPresent([...new Set(list.map(([, c]) => c as string))]),
      () => {},
    );
    return () => {
      live = false;
    };
  }, [view]);
  const cats = useMemo(
    () => [...new Set([...present, ...(view?.hiddenCategories ?? [])])].sort(),
    [present, view],
  );
  if (!view) return null;
  const hidden = new Set<string>(view.hiddenCategories);
  const label = (c: string) => c.replace(/([a-z])([A-Z])/g, "$1 $2");
  return (
    <div className="modal-backdrop" role="dialog" aria-label="Visibility/Graphics">
      <div className="modal vv-dialog">
        <h2>Visibility/Graphics — {view.name}</h2>
        <div className="vv-list">
          {cats.map((c) => (
            <label key={c} className="ob-check">
              <input
                type="checkbox"
                checked={!hidden.has(c)}
                onChange={(e) =>
                  void apply(() =>
                    ipc.setCategoryVisible(view.id, [c as Category], e.target.checked),
                  )
                }
              />
              {label(c)}
            </label>
          ))}
          {cats.length === 0 && <p className="muted">Nothing in this view yet.</p>}
        </div>
        <div className="modal-actions">
          <button
            className="btn-ghost"
            disabled={view.hiddenCount === 0 && view.hiddenCategories.length === 0}
            onClick={() => void apply(() => ipc.unhideAll(view.id))}
          >
            Unhide All ({view.hiddenCount} elements)
          </button>
          <button className="btn-cyan" onClick={onClose}>
            OK
          </button>
        </div>
      </div>
    </div>
  );
}
