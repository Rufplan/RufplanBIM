import { useEffect, useMemo, useState } from "react";
import type { ComponentTypeInfo } from "../bindings/ComponentTypeInfo";
import { componentTypes } from "../components";
import { useAppStore } from "../store";

// The options bar for Detail Component (ADR-071): Revit's type selector (grouped by family),
// a point-based component's rotation, and which side of its line a line-based one lies.

export function ComponentOptions() {
  const o = useAppStore((s) => s.options);
  const setOption = useAppStore((s) => s.setOption);
  const [types, setTypes] = useState<ComponentTypeInfo[]>([]);
  useEffect(() => {
    let live = true;
    componentTypes().then(
      (t) => live && setTypes(t),
      () => {},
    );
    return () => {
      live = false;
    };
  }, []);
  const families = useMemo(() => {
    const m = new Map<string, ComponentTypeInfo[]>();
    for (const t of types) m.set(t.familyLabel, [...(m.get(t.familyLabel) ?? []), t]);
    return [...m.entries()];
  }, [types]);
  const current = types.find((t) => t.key === o.componentKey);
  return (
    <>
      <label className="ob-field">
        Type
        <select
          aria-label="Component Type"
          value={o.componentKey}
          onChange={(e) => setOption("componentKey", e.target.value)}
        >
          {families.map(([family, ts]) => (
            <optgroup key={family} label={family}>
              {ts.map((t) => (
                <option key={t.key} value={t.key}>
                  {t.name}
                </option>
              ))}
            </optgroup>
          ))}
        </select>
      </label>
      {current && !current.lineBased && (
        <label className="ob-field">
          Rotation
          <select
            aria-label="Rotation"
            value={o.componentRotation}
            onChange={(e) => setOption("componentRotation", Number(e.target.value))}
          >
            {[0, 90, 180, 270].map((d) => (
              <option key={d} value={d}>
                {d}°
              </option>
            ))}
          </select>
        </label>
      )}
      <label className="ob-check">
        <input
          type="checkbox"
          aria-label="Flip"
          checked={o.componentFlip}
          onChange={(e) => setOption("componentFlip", e.target.checked)}
        />
        Flip
      </label>
      <span className="ob-hint">
        {current?.lineBased ? "Click its start, then its end" : "Click to place; Space rotates 90°"}
      </span>
    </>
  );
}
