import { useEffect, useState } from "react";
import type { BuildingUse } from "../bindings/BuildingUse";
import type { ElementId } from "../bindings/ElementId";
import type { FixtureSource } from "../bindings/FixtureSource";
import type { FixtureSpec } from "../bindings/FixtureSpec";
import type { LightLibrary } from "../bindings/LightLibrary";
import type { LightPreset } from "../bindings/LightPreset";
import { apply } from "../fileActions";
import { errorMessage, ipc } from "../ipc";
import { thumbnail } from "../render/thumbs";
import { useAppStore } from "../store";

// The lighting fixture picker (ADR-057), like the door and window picker: the project's
// fixture types as rendered thumbnails, and the library of typical fixtures for every
// building type. Pick one to place it, or to change the selected fixtures to it.

let library: Promise<LightLibrary> | null = null;
const loadLibrary = () => {
  library ??= ipc.lightingLibrary();
  library.catch(() => (library = null));
  return library;
};

const MM_PER_IN = 25.4;
const inches = (mm: number) => {
  const i = mm / MM_PER_IN;
  if (i >= 36 && Math.abs(i % 12) < 0.01) return `${i / 12}'`;
  return `${Math.round(i * 4) / 4}"`;
};

/** Revit's photometrics of a fixture, as (label, value). */
export function photometrics(s: FixtureSpec): [string, string][] {
  const rows: [string, string][] = [
    ["Initial intensity", `${s.lumens.toLocaleString()} lm`],
    ["Wattage", `${s.watts} W (${Math.round(s.lumens / Math.max(s.watts, 0.1))} lm/W)`],
    ["Color temperature", `${s.kelvin.toLocaleString()} K`],
    [
      "Light source",
      `${s.shape}, ${s.distribution}${s.distribution === "Spot" ? ` ${s.beam}°` : ""}`,
    ],
    ["Mounting", s.mount === "Pendant" ? `Pendant, ${inches(s.drop)} stem` : s.mount],
    ["Size", `${inches(s.width)} x ${inches(s.depth)} x ${inches(s.height)}`],
  ];
  if (s.mount === "Wall" || (s.mount === "Floor" && s.mount_height > 0))
    rows.push(["Mounting height", inches(s.mount_height)]);
  return rows;
}

/** A rendered fixture thumbnail, drawn on demand. */
function Thumb({ source, label, big }: { source: FixtureSource; label: string; big?: boolean }) {
  const revision = useAppStore((s) => s.app?.revision ?? 0);
  const key =
    "Type" in source ? `light:${source.Type}:${revision}` : `light:${JSON.stringify(source)}`;
  const [url, setUrl] = useState<{ key: string; url: string | null } | null>(null);
  useEffect(() => {
    let live = true;
    thumbnail(key, () => ipc.fixtureThumbnail(source)).then(
      (u) => live && setUrl({ key, url: u }),
      () => live && setUrl({ key, url: null }),
    );
    return () => {
      live = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key]);
  const cls = `tp-thumb lp-thumb${big ? " big" : ""}`;
  if (url?.key === key && url.url)
    return <img className={cls} src={url.url} alt={label} draggable={false} />;
  return (
    <div className={`${cls} tp-rendering`} role="img" aria-label={label}>
      {url?.key === key ? "" : "Rendering…"}
    </div>
  );
}

const swatch = (kelvin: number) => {
  // Warm to cool, for the color temperature chip.
  const t = Math.min(Math.max((kelvin - 2700) / (6500 - 2700), 0), 1);
  const warm = [255, 196, 137];
  const cool = [214, 229, 255];
  return `rgb(${warm.map((w, i) => Math.round(w + (cool[i]! - w) * t)).join(",")})`;
};

type Tab = "project" | "library";

export function LightPicker({ onClose }: { onClose: () => void }) {
  const app = useAppStore((s) => s.app);
  const picker = useAppStore((s) => s.picker);
  const targets = picker?.change ?? [];
  const [tab, setTab] = useState<Tab>(picker?.tab ?? "project");
  const [lib, setLib] = useState<LightLibrary | null>(null);
  const [picked, setPicked] = useState<ElementId | null>(null);
  const [group, setGroup] = useState<string | null>(null);
  const [use, setUse] = useState<BuildingUse | null>(null);
  const [preset, setPreset] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [layout, setLayout] = useState<"grid" | "list">("grid");
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    let live = true;
    loadLibrary().then(
      (l) => live && setLib(l),
      (e) => useAppStore.getState().setError(errorMessage(e)),
    );
    return () => {
      live = false;
    };
  }, []);

  const types = app?.lightingFixtureTypes ?? [];
  const toolType = useAppStore((s) => s.toolTypes.light);
  const q = query.trim().toLowerCase();
  const shownTypes = types
    .filter((t) => !q || t.name.toLowerCase().includes(q))
    .sort((a, b) => a.name.localeCompare(b.name));
  const current = picked ?? toolType;
  const presetOf = (name: string) => lib?.presets.find((p) => p.name === name);
  const shownPresets: LightPreset[] = (lib?.presets ?? []).filter(
    (p) =>
      (!group || p.group === group) &&
      (!use || p.uses.includes(use)) &&
      (!q ||
        p.name.toLowerCase().includes(q) ||
        p.description.toLowerCase().includes(q) ||
        p.group.toLowerCase().includes(q)),
  );
  const chosen = preset ? presetOf(preset) : undefined;

  /** Place with `id`, or change the targets to it. */
  const finishWith = async (id: ElementId, change: boolean) => {
    const s = useAppStore.getState();
    if (change && targets.length) {
      for (const t of targets) {
        if (!(await apply(() => ipc.setProperty(t, "type", id)))) return;
      }
      s.select(targets);
      onClose();
      return;
    }
    s.setToolType("light", id);
    s.setTool("light");
    onClose();
  };
  const loadChosen = async (change: boolean, andUse = true) => {
    if (!chosen) return;
    setBusy(true);
    try {
      const r = await ipc.loadLightingTypes([chosen.name]);
      useAppStore.getState().setApp(r.state);
      if (andUse && r.ids[0]) await finishWith(r.ids[0], change);
      else if (r.ids[0]) {
        setPicked(r.ids[0]);
        setTab("project");
      }
    } catch (e) {
      useAppStore.getState().setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  const layoutToggle = (
    <div className="lp-layout" role="radiogroup" aria-label="Layout">
      {(["grid", "list"] as const).map((l) => (
        <button
          key={l}
          role="radio"
          aria-checked={layout === l}
          className={`mb-tab${layout === l ? " active" : ""}`}
          onClick={() => setLayout(l)}
        >
          {l === "grid" ? "Grid" : "List"}
        </button>
      ))}
    </div>
  );
  const cards = <T,>(
    items: T[],
    key: (t: T) => string,
    name: (t: T) => string,
    source: (t: T) => FixtureSource,
    active: (t: T) => boolean,
    pick: (t: T) => void,
    go: (t: T) => void,
    note?: (t: T) => string,
    badge?: (t: T) => boolean,
  ) => (
    <div
      className={layout === "grid" ? "mb-grid tp-grid" : "lp-list"}
      role="list"
      aria-label={tab === "project" ? "Project lighting fixtures" : "Library fixtures"}
    >
      {items.map((t) => (
        <button
          key={key(t)}
          role="listitem"
          className={`${layout === "grid" ? "mb-card" : "lp-row"}${active(t) ? " active" : ""}`}
          onClick={() => pick(t)}
          onDoubleClick={() => go(t)}
          title={name(t)}
        >
          <Thumb source={source(t)} label={name(t)} />
          <span className="mb-name">{name(t)}</span>
          {layout === "list" && note && <span className="muted lp-note">{note(t)}</span>}
          {badge?.(t) && <span className="mb-tier wl-loaded">In project</span>}
        </button>
      ))}
      {items.length === 0 && <p className="muted">No fixtures match.</p>}
    </div>
  );

  return (
    <div className="modal-backdrop" role="dialog" aria-label="Lighting Fixture Types">
      <div className="modal material-browser type-picker light-picker">
        <div className="mb-head">
          <h2>Lighting Fixtures</h2>
          <div className="mb-scope" role="tablist">
            {(["project", "library"] as const).map((t) => (
              <button
                key={t}
                role="tab"
                aria-selected={tab === t}
                className={`mb-tab${tab === t ? " active" : ""}`}
                onClick={() => setTab(t)}
              >
                {t === "project" ? `In This Project (${types.length})` : "Lighting Library"}
              </button>
            ))}
          </div>
          <input
            className="mb-search"
            aria-label="Search lighting fixtures"
            placeholder="Search downlight, pendant, exterior…"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
          {targets.length > 0 && (
            <span className="tp-mode">
              Changing {targets.length} selected {targets.length === 1 ? "fixture" : "fixtures"}
            </span>
          )}
          {layoutToggle}
          <button className="btn-ghost" onClick={onClose} aria-label="Close">
            ×
          </button>
        </div>
        {tab === "project" ? (
          <div className="mb-body">
            {cards(
              shownTypes,
              (t) => t.id,
              (t) => t.name,
              (t) => ({ Type: t.id }),
              (t) => current === t.id,
              (t) => setPicked(t.id),
              (t) => void finishWith(t.id, targets.length > 0),
              (t) => presetOf(t.name)?.description ?? "",
            )}
            <aside className="mb-detail">
              {current && types.some((t) => t.id === current) ? (
                <>
                  <Thumb
                    source={{ Type: current }}
                    label={types.find((t) => t.id === current)?.name ?? ""}
                    big
                  />
                  <h3>{types.find((t) => t.id === current)?.name}</h3>
                  <div className="mb-actions">
                    {targets.length > 0 && (
                      <button className="btn-cyan" onClick={() => void finishWith(current, true)}>
                        Change Selected ({targets.length})
                      </button>
                    )}
                    <button
                      className={targets.length ? "btn-outline" : "btn-cyan"}
                      onClick={() => void finishWith(current, false)}
                    >
                      Place Fixture
                    </button>
                    <button
                      className="btn-outline"
                      onClick={() => {
                        useAppStore.getState().select([current]);
                        onClose();
                      }}
                    >
                      Edit Type
                    </button>
                  </div>
                  <p className="muted">
                    Double-click a type to {targets.length ? "apply it" : "place it"}. Every
                    building type's typical fixtures are in the Lighting Library.
                  </p>
                </>
              ) : (
                <p className="muted">Pick a type.</p>
              )}
            </aside>
          </div>
        ) : (
          <div className="mb-body">
            <nav className="mb-filters wl-families" aria-label="Groups">
              <h3>Fixtures</h3>
              {[null, ...(lib?.groups ?? [])].map((g) => (
                <button
                  key={g ?? "all"}
                  className={`mb-cat${group === g ? " active" : ""}`}
                  onClick={() => setGroup(g)}
                >
                  {g ?? "All Fixtures"}
                </button>
              ))}
              <h3>Building Type</h3>
              <select
                aria-label="Building type"
                value={use ?? ""}
                onChange={(e) => setUse((e.target.value || null) as BuildingUse | null)}
              >
                <option value="">All building types</option>
                {(lib?.uses ?? []).map((u) => (
                  <option key={u.id} value={u.id}>
                    {u.label}
                  </option>
                ))}
              </select>
            </nav>
            {cards(
              shownPresets,
              (p) => p.name,
              (p) => p.name,
              (p) => ({ Preset: p.name }),
              (p) => preset === p.name,
              (p) => setPreset(p.name),
              (p) => {
                setPreset(p.name);
                void loadChosen(targets.length > 0);
              },
              (p) => p.description,
              (p) => types.some((t) => t.name === p.name),
            )}
            <aside className="mb-detail">
              {chosen ? (
                <>
                  <Thumb source={{ Preset: chosen.name }} label={chosen.name} big />
                  <h3>{chosen.name}</h3>
                  <p className="muted">{chosen.description}</p>
                  <table className="lp-specs" aria-label="Photometrics">
                    <tbody>
                      {photometrics(chosen.spec).map(([k, v]) => (
                        <tr key={k}>
                          <th>{k}</th>
                          <td>
                            {k === "Color temperature" && (
                              <span
                                className="lp-swatch"
                                style={{ background: swatch(chosen.spec.kelvin) }}
                              />
                            )}
                            {v}
                          </td>
                        </tr>
                      ))}
                      <tr>
                        <th>Typical in</th>
                        <td>
                          {chosen.uses
                            .map((u) => lib?.uses.find((x) => x.id === u)?.label ?? u)
                            .join(", ")}
                        </td>
                      </tr>
                    </tbody>
                  </table>
                  <div className="mb-actions">
                    {targets.length > 0 && (
                      <button
                        className="btn-cyan"
                        disabled={busy}
                        onClick={() => void loadChosen(true)}
                      >
                        Load &amp; Change Selected ({targets.length})
                      </button>
                    )}
                    <button
                      className={targets.length ? "btn-outline" : "btn-cyan"}
                      disabled={busy}
                      onClick={() => void loadChosen(false)}
                    >
                      Load &amp; Place
                    </button>
                    <button
                      className="btn-outline"
                      disabled={busy}
                      onClick={() => void loadChosen(false, false)}
                    >
                      Load
                    </button>
                  </div>
                </>
              ) : (
                <p className="muted">
                  Pick a fixture: {lib?.presets.length ?? "…"} typical fixtures for homes, offices,
                  stores, hotels, hospitals, schools, warehouses and sites.
                </p>
              )}
            </aside>
          </div>
        )}
      </div>
    </div>
  );
}
