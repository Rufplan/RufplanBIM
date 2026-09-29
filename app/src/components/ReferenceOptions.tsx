import { useEffect, useState } from "react";
import type { RefTarget } from "../bindings/RefTarget";
import { ipc } from "../ipc";
import { activeViewInfo, useAppStore } from "../store";
import { referenceForced } from "../tools";

// Section and Callout on the options bar (ADR-076): Revit's "Reference Other View" and the
// view to reference, "<New drafting view>" first. In drafting views it is always on.

export const NEW_DRAFTING = "";

export function ReferenceOptions({ callout }: { callout: boolean }) {
  const view = useAppStore(activeViewInfo);
  const revision = useAppStore((s) => s.app?.revision);
  const on = useAppStore((s) => s.options.refOther);
  const target = useAppStore((s) => s.options.refTarget);
  const set = useAppStore((s) => s.setOption);
  const [targets, setTargets] = useState<RefTarget[]>([]);
  const forced = referenceForced(view?.viewType);
  useEffect(() => {
    if (!view) return;
    let live = true;
    ipc.referenceTargets(view.id, callout).then(
      (t) => live && setTargets(t),
      () => live && setTargets([]),
    );
    return () => {
      live = false;
    };
  }, [view, callout, revision]);
  const checked = on || forced;
  const value = targets.some((t) => t.id === target) ? target : NEW_DRAFTING;
  return (
    <>
      <label className="ob-check">
        <input
          type="checkbox"
          checked={checked}
          disabled={forced}
          onChange={(e) => set("refOther", e.target.checked)}
        />
        Reference Other View
      </label>
      <label className="ob-field">
        <select
          aria-label="Referenced View"
          disabled={!checked}
          value={value}
          onChange={(e) => set("refTarget", e.target.value)}
        >
          <option value={NEW_DRAFTING}>&lt;New drafting view&gt;</option>
          {targets.map((t) => (
            <option key={t.id} value={t.id}>
              {t.label}
            </option>
          ))}
        </select>
      </label>
    </>
  );
}

/** The reference the Section or Callout tool makes: null when it makes a new view. */
export function referenceChoice(): { target: string | null } | null {
  const s = useAppStore.getState();
  const view = activeViewInfo(s);
  if (!s.options.refOther && !referenceForced(view?.viewType)) return null;
  return { target: s.options.refTarget || null };
}
