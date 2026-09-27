import { useEffect, useState } from "react";
import type { LightInfo } from "../bindings/LightInfo";
import type { SunPosition } from "../bindings/SunPosition";
import type { SunSettings } from "../bindings/SunSettings";
import { apply } from "../fileActions";
import { ipc } from "../ipc";
import { useAppStore } from "../store";

// Revit's Sun Settings and Artificial Lights dialogs (ADR-057).

const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

export function clock(hour: number): string {
  const h = Math.floor(hour);
  const m = Math.round((hour - h) * 60);
  const h12 = h % 12 === 0 ? 12 : h % 12;
  return `${h12}:${String(m).padStart(2, "0")} ${h < 12 ? "AM" : "PM"}`;
}

/** Revit's presets: solstices and equinoxes for Still, directions for Lighting. */
export const SUN_PRESETS: { name: string; patch: Partial<SunSettings> }[] = [
  { name: "Summer Solstice, Noon", patch: { mode: "Still", month: 6, day: 21, hour: 12 } },
  { name: "Summer Afternoon", patch: { mode: "Still", month: 6, day: 21, hour: 15 } },
  { name: "Spring Equinox, Noon", patch: { mode: "Still", month: 3, day: 20, hour: 12 } },
  { name: "Fall Equinox, Noon", patch: { mode: "Still", month: 9, day: 22, hour: 12 } },
  { name: "Winter Solstice, Noon", patch: { mode: "Still", month: 12, day: 21, hour: 12 } },
  { name: "Winter Afternoon", patch: { mode: "Still", month: 12, day: 21, hour: 15 } },
  { name: "Sunlight from the Southwest", patch: { mode: "Lighting", azimuth: 225, altitude: 35 } },
  { name: "Sunlight from the Southeast", patch: { mode: "Lighting", azimuth: 135, altitude: 35 } },
  { name: "Sunlight from Overhead", patch: { mode: "Lighting", azimuth: 180, altitude: 80 } },
];

const DEFAULT_SUN: SunSettings = {
  mode: "Still",
  month: 6,
  day: 21,
  hour: 15,
  azimuth: 225,
  altitude: 35,
};

export function SunSettingsDialog({ onClose }: { onClose: () => void }) {
  const saved = useAppStore((s) => s.app?.sun ?? DEFAULT_SUN);
  const site = useAppStore((s) => s.app?.site ?? null);
  const [s, setS] = useState<SunSettings>(saved);
  const [sun, setSun] = useState<SunPosition | null>(null);
  const set = (patch: Partial<SunSettings>) => setS((o) => ({ ...o, ...patch }));
  useEffect(() => {
    if (s.mode !== "Still") return;
    let live = true;
    ipc.sunPosition(s.month, s.day, s.hour).then(
      (p) => live && setSun(p),
      () => live && setSun(null),
    );
    return () => {
      live = false;
    };
  }, [s.mode, s.month, s.day, s.hour]);
  const save = async () => {
    if (await apply(() => ipc.setSunSettings(s))) onClose();
  };
  const where = site ? "the site" : "central USA (no site set on the Site tab)";
  return (
    <div className="modal-backdrop" role="dialog" aria-label="Sun Settings">
      <div className="modal sun-settings">
        <h2>Sun Settings</h2>
        <fieldset className="sun-mode">
          <legend>Solar Study</legend>
          {(["Still", "Lighting"] as const).map((m) => (
            <label key={m}>
              <input
                type="radio"
                name="solar-study"
                checked={s.mode === m}
                onChange={() => set({ mode: m })}
              />
              {m}
            </label>
          ))}
          <span className="muted">Single Day and Multi-Day studies aren&apos;t available yet.</span>
        </fieldset>
        <label className="field">
          Presets
          <select
            aria-label="Presets"
            value=""
            onChange={(e) => {
              const p = SUN_PRESETS.find((x) => x.name === e.target.value);
              if (p) set(p.patch);
            }}
          >
            <option value="">Choose a preset…</option>
            {SUN_PRESETS.map((p) => (
              <option key={p.name} value={p.name}>
                {p.name}
              </option>
            ))}
          </select>
        </label>
        {s.mode === "Still" ? (
          <>
            <p className="muted">Location: {where}.</p>
            <div className="row">
              <label className="field">
                Month
                <select
                  aria-label="Month"
                  value={s.month}
                  onChange={(e) => set({ month: Number(e.target.value) })}
                >
                  {MONTHS.map((m, i) => (
                    <option key={m} value={i + 1}>
                      {m}
                    </option>
                  ))}
                </select>
              </label>
              <label className="field">
                Day
                <input
                  aria-label="Day"
                  type="number"
                  min={1}
                  max={31}
                  value={s.day}
                  onChange={(e) =>
                    set({ day: Math.max(1, Math.min(31, Number(e.target.value) || 1)) })
                  }
                />
              </label>
            </div>
            <label className="field">
              Time: {clock(s.hour)}
              <input
                aria-label="Time of day"
                type="range"
                min={0}
                max={23.75}
                step={0.25}
                value={s.hour}
                onChange={(e) => set({ hour: Number(e.target.value) })}
              />
            </label>
            <p className="muted" role="status">
              {sun
                ? sun.altitude > 0
                  ? `Sun ${sun.altitude.toFixed(0)}° up, ${sun.azimuth.toFixed(0)}° from north.`
                  : "The sun is down at that time."
                : ""}
            </p>
          </>
        ) : (
          <>
            <label className="field">
              Azimuth: {s.azimuth.toFixed(0)}° from north
              <input
                aria-label="Azimuth"
                type="range"
                min={0}
                max={359}
                step={1}
                value={s.azimuth}
                onChange={(e) => set({ azimuth: Number(e.target.value) })}
              />
            </label>
            <label className="field">
              Altitude: {s.altitude.toFixed(0)}° above the horizon
              <input
                aria-label="Altitude"
                type="range"
                min={0}
                max={90}
                step={1}
                value={s.altitude}
                onChange={(e) => set({ altitude: Number(e.target.value) })}
              />
            </label>
          </>
        )}
        <div className="modal-actions">
          <button className="btn-outline" onClick={onClose}>
            Cancel
          </button>
          <button className="btn-cyan" onClick={() => void save()}>
            OK
          </button>
        </div>
      </div>
    </div>
  );
}

export function ArtificialLightsDialog({ onClose }: { onClose: () => void }) {
  const app = useAppStore((s) => s.app);
  const revision = app?.revision ?? 0;
  const [lights, setLights] = useState<LightInfo[] | null>(null);
  useEffect(() => {
    let live = true;
    ipc.lights(null).then(
      (l) => live && setLights(l),
      () => live && setLights([]),
    );
    return () => {
      live = false;
    };
  }, [revision]);
  const typeName = (id: string) =>
    app?.lightingFixtureTypes.find((t) => t.id === id)?.name ?? "Lighting Fixture";
  const levelName = (id: string) => app?.levels.find((l) => l.id === id)?.name ?? "";
  // Revit's light groups: here, the fixtures of each type.
  const groups = new Map<string, LightInfo[]>();
  for (const l of lights ?? []) groups.set(l.type_id, [...(groups.get(l.type_id) ?? []), l]);
  const change = (ids: string[], on: boolean | null, dimming: number | null) =>
    void apply(() => ipc.setLights(ids, on, dimming));
  const all = (lights ?? []).map((l) => l.el);
  return (
    <div className="modal-backdrop" role="dialog" aria-label="Artificial Lights">
      <div className="modal artificial-lights">
        <h2>Artificial Lights</h2>
        {lights === null ? (
          <p className="muted">Loading…</p>
        ) : lights.length === 0 ? (
          <p className="muted">
            There are no lighting fixtures yet: place some with Lighting Fixture on the Lighting
            tab.
          </p>
        ) : (
          <table className="al-table">
            <thead>
              <tr>
                <th>Light</th>
                <th>On</th>
                <th>Dimming</th>
              </tr>
            </thead>
            {[...groups.entries()].map(([type, ls]) => {
              const on = ls.filter((l) => l.on).length;
              return (
                <tbody key={type}>
                  <tr className="al-group">
                    <th>
                      {typeName(type)} ({ls.length})
                    </th>
                    <td>
                      <input
                        type="checkbox"
                        aria-label={`${typeName(type)} on`}
                        checked={on === ls.length}
                        ref={(el) => {
                          if (el) el.indeterminate = on > 0 && on < ls.length;
                        }}
                        onChange={(e) =>
                          change(
                            ls.map((l) => l.el),
                            e.target.checked,
                            null,
                          )
                        }
                      />
                    </td>
                    <td />
                  </tr>
                  {ls.map((l, i) => (
                    <tr key={l.el}>
                      <td>
                        {typeName(type)} {i + 1}{" "}
                        <span className="muted">({levelName(l.level)})</span>
                      </td>
                      <td>
                        <input
                          type="checkbox"
                          aria-label={`${typeName(type)} ${i + 1} on`}
                          checked={l.on}
                          onChange={(e) => change([l.el], e.target.checked, null)}
                        />
                      </td>
                      <td>
                        <input
                          type="number"
                          aria-label={`${typeName(type)} ${i + 1} dimming`}
                          min={0}
                          max={100}
                          step={5}
                          defaultValue={Math.round(l.dimming * 100)}
                          onBlur={(e) =>
                            change([l.el], null, Math.min(100, Math.max(0, +e.target.value)) / 100)
                          }
                        />
                        %
                      </td>
                    </tr>
                  ))}
                </tbody>
              );
            })}
          </table>
        )}
        <div className="modal-actions">
          <button
            className="btn-outline"
            disabled={!all.length}
            onClick={() => change(all, true, null)}
          >
            All On
          </button>
          <button
            className="btn-outline"
            disabled={!all.length}
            onClick={() => change(all, false, null)}
          >
            All Off
          </button>
          <button className="btn-cyan" onClick={onClose}>
            Close
          </button>
        </div>
      </div>
    </div>
  );
}
