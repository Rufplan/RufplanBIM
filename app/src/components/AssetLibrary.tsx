import { useEffect, useMemo, useState } from "react";
import type { Climate } from "../bindings/Climate";
import type { ElementId } from "../bindings/ElementId";
import type { PlantGroup } from "../bindings/PlantGroup";
import type { PlantLibrary } from "../bindings/PlantLibrary";
import type { PlantPreset } from "../bindings/PlantPreset";
import type { PlantSource } from "../bindings/PlantSource";
import type { Season } from "../bindings/Season";
import { apply } from "../fileActions";
import { errorMessage, ipc } from "../ipc";
import { plantThumb } from "../render/plantThumbs";
import type { PlantLoader } from "../render/plants";
import { useAppStore } from "../store";
import { formatFeet } from "./assetFormat";

// Enscape's Asset Library for vegetation (ADR-064): categories down the left (Trees,
// Bushes, Grass & Flowers, Plants), a search and season and climate filters across the
// top, rendered thumbnails of every asset, and its details with Place. Picking an asset
// loads it into the project and starts placing it, click after click.

let library: Promise<PlantLibrary> | null = null;
const loadLibrary = () => {
  library ??= ipc.plantingLibrary();
  library.catch(() => (library = null));
  return library;
};

export const plantLoader: PlantLoader = {
  model: (source, variant) => ipc.plantModel(source, variant),
  texture: (source, map, size) => ipc.plantTexture(source, map, size),
};

/** A rendered thumbnail, drawn when it's first shown. */
function Thumb({ source, label, big }: { source: PlantSource; label: string; big?: boolean }) {
  const key = JSON.stringify(source);
  const [url, setUrl] = useState<{ key: string; url: string | null } | null>(null);
  useEffect(() => {
    let live = true;
    plantThumb(source, plantLoader).then(
      (u) => live && setUrl({ key, url: u }),
      () => live && setUrl({ key, url: null }),
    );
    return () => {
      live = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key]);
  const cls = `al-thumb${big ? " big" : ""}`;
  if (url?.key === key && url.url)
    return <img className={cls} src={url.url} alt={label} draggable={false} />;
  return (
    <div className={`${cls} al-rendering`} role="img" aria-label={label}>
      {url?.key === key ? "" : "Growing…"}
    </div>
  );
}

type Category = "Trees" | "Bushes" | "Grass & Flowers" | "Plants";
const CATEGORIES: Category[] = ["Trees", "Bushes", "Grass & Flowers", "Plants"];
const SEASONS: Season[] = ["Spring", "Summer", "Autumn", "Winter"];

const size = (p: PlantPreset) =>
  `${formatFeet(p.spec.height)} tall, ${formatFeet(p.spec.spread)} spread`;

export function AssetLibrary({ onClose }: { onClose: () => void }) {
  const app = useAppStore((s) => s.app);
  const picker = useAppStore((s) => s.picker);
  const targets = picker?.change ?? [];
  const [tab, setTab] = useState<"project" | "library">(picker?.tab ?? "library");
  const [lib, setLib] = useState<PlantLibrary | null>(null);
  const start = useAppStore.getState().assetFilter;
  const [category, setCategory] = useState<Category | null>(
    (start?.category as Category | undefined) ?? null,
  );
  const [group, setGroup] = useState<PlantGroup | null>(
    (start?.group as PlantGroup | null | undefined) ?? null,
  );
  const [season, setSeason] = useState<Season | null>(null);
  const [climate, setClimate] = useState<Climate | null>(null);
  const [query, setQuery] = useState("");
  const [chosen, setChosen] = useState<string | null>(null);
  const [picked, setPicked] = useState<ElementId | null>(null);
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

  const types = app?.plantingTypes ?? [];
  const toolType = useAppStore((s) => s.toolTypes.plant);
  const q = query.trim().toLowerCase();
  const groupsOf = (c: Category) => (lib?.groups ?? []).filter(([, , cat]) => cat === c);
  const shown = useMemo(
    () =>
      (lib?.presets ?? []).filter((p) => {
        const [, , cat] = lib?.groups.find(([g]) => g === p.spec.group) ?? [];
        if (category && cat !== category) return false;
        if (group && p.spec.group !== group) return false;
        // A season shows that season's variants, and evergreens (which have none).
        if (season) {
          const variants = (lib?.presets ?? []).some(
            (o) => o.species === p.species && o.name !== p.name,
          );
          if (variants ? p.spec.season !== season : false) return false;
        } else if (p.name !== p.species) return false;
        if (climate && !p.climates.includes(climate)) return false;
        return (
          !q ||
          p.name.toLowerCase().includes(q) ||
          p.spec.botanical.toLowerCase().includes(q) ||
          p.description.toLowerCase().includes(q)
        );
      }),
    [lib, category, group, season, climate, q],
  );
  const preset = chosen ? lib?.presets.find((p) => p.name === chosen) : undefined;
  const current = picked ?? toolType;

  /** Place with `id`, or change the selected plants to it. */
  const finishWith = async (id: ElementId, change: boolean) => {
    const s = useAppStore.getState();
    if (change && targets.length) {
      for (const t of targets) if (!(await apply(() => ipc.setProperty(t, "type", id)))) return;
      s.select(targets);
      onClose();
      return;
    }
    s.setToolType("plant", id);
    s.setTool("plant");
    onClose();
  };
  const load = async (change: boolean, andUse = true) => {
    if (!preset) return;
    setBusy(true);
    try {
      const r = await ipc.loadPlantingTypes([preset.name]);
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

  const count = (c: Category | null, g: PlantGroup | null) =>
    (lib?.presets ?? []).filter((p) => {
      if (p.name !== p.species) return false;
      const [, , cat] = lib?.groups.find(([x]) => x === p.spec.group) ?? [];
      return (!c || cat === c) && (!g || p.spec.group === g);
    }).length;

  return (
    <div className="modal-backdrop" role="dialog" aria-label="Asset Library">
      <div className="modal asset-library">
        <div className="al-head">
          <h2>Asset Library</h2>
          <div className="mb-scope" role="tablist">
            {(["library", "project"] as const).map((t) => (
              <button
                key={t}
                role="tab"
                aria-selected={tab === t}
                className={`mb-tab${tab === t ? " active" : ""}`}
                onClick={() => setTab(t)}
              >
                {t === "project" ? `In This Project (${types.length})` : "Vegetation"}
              </button>
            ))}
          </div>
          <input
            className="al-search"
            aria-label="Search assets"
            placeholder="Search maple, palm, hedge, Quercus…"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
          {targets.length > 0 && (
            <span className="tp-mode">
              Changing {targets.length} selected {targets.length === 1 ? "plant" : "plants"}
            </span>
          )}
          <button className="btn-ghost" onClick={onClose} aria-label="Close">
            ×
          </button>
        </div>
        {tab === "library" ? (
          <div className="al-body">
            <nav className="al-nav" aria-label="Categories">
              <h3>Vegetation</h3>
              <button
                className={`al-cat${!category ? " active" : ""}`}
                onClick={() => {
                  setCategory(null);
                  setGroup(null);
                }}
              >
                All Vegetation <span>{count(null, null)}</span>
              </button>
              {CATEGORIES.map((c) => (
                <div key={c}>
                  <button
                    className={`al-cat${category === c && !group ? " active" : ""}`}
                    onClick={() => {
                      setCategory(c);
                      setGroup(null);
                    }}
                  >
                    {c} <span>{count(c, null)}</span>
                  </button>
                  {(category === c || c === "Trees") &&
                    groupsOf(c).map(([g, label]) => (
                      <button
                        key={g}
                        className={`al-sub${group === g ? " active" : ""}`}
                        onClick={() => {
                          setCategory(c);
                          setGroup(g);
                        }}
                      >
                        {label} <span>{count(c, g)}</span>
                      </button>
                    ))}
                </div>
              ))}
            </nav>
            <div className="al-main">
              <div className="al-filters">
                <div className="al-chips" role="radiogroup" aria-label="Season">
                  {[null, ...SEASONS].map((x) => (
                    <button
                      key={x ?? "all"}
                      role="radio"
                      aria-checked={season === x}
                      className={`al-chip${season === x ? " on" : ""}`}
                      onClick={() => setSeason(x)}
                    >
                      {x ?? "Any season"}
                    </button>
                  ))}
                </div>
                <select
                  aria-label="Climate"
                  value={climate ?? ""}
                  onChange={(e) => setClimate((e.target.value || null) as Climate | null)}
                >
                  <option value="">Any climate</option>
                  {(lib?.climates ?? []).map(([c, label]) => (
                    <option key={c} value={c}>
                      {label}
                    </option>
                  ))}
                </select>
                <span className="muted">{shown.length} assets</span>
              </div>
              <div className="al-grid" role="list" aria-label="Assets">
                {shown.map((p) => (
                  <button
                    key={p.name}
                    role="listitem"
                    className={`al-card${chosen === p.name ? " active" : ""}`}
                    onClick={() => setChosen(p.name)}
                    onDoubleClick={() => {
                      setChosen(p.name);
                      void (async () => {
                        setBusy(true);
                        try {
                          const r = await ipc.loadPlantingTypes([p.name]);
                          useAppStore.getState().setApp(r.state);
                          if (r.ids[0]) await finishWith(r.ids[0], targets.length > 0);
                        } catch (e) {
                          useAppStore.getState().setError(errorMessage(e));
                        } finally {
                          setBusy(false);
                        }
                      })();
                    }}
                    title={`${p.name} (${p.spec.botanical}): ${size(p)}`}
                  >
                    <Thumb source={{ Preset: p.name }} label={p.name} />
                    <span className="al-name">{p.name}</span>
                    <span className="al-size">{formatFeet(p.spec.height)}</span>
                    {types.some((t) => t.name === p.name) && (
                      <span className="al-badge">In project</span>
                    )}
                  </button>
                ))}
                {lib && shown.length === 0 && <p className="muted">No assets match.</p>}
              </div>
            </div>
            <aside className="al-detail">
              {preset ? (
                <>
                  <Thumb source={{ Preset: preset.name }} label={preset.name} big />
                  <h3>{preset.name}</h3>
                  <p className="al-botanical">{preset.spec.botanical}</p>
                  <p className="muted">{preset.description}</p>
                  <table className="lp-specs" aria-label="Asset details">
                    <tbody>
                      <tr>
                        <th>Size</th>
                        <td>{size(preset)}</td>
                      </tr>
                      <tr>
                        <th>Group</th>
                        <td>{lib?.groups.find(([g]) => g === preset.spec.group)?.[1]}</td>
                      </tr>
                      {lib?.presets.some(
                        (o) => o.species === preset.species && o.name !== preset.name,
                      ) && (
                        <tr>
                          <th>Season</th>
                          <td>{preset.spec.season}</td>
                        </tr>
                      )}
                      <tr>
                        <th>Climate</th>
                        <td>
                          {preset.climates
                            .map((c) => lib?.climates.find(([x]) => x === c)?.[1] ?? c)
                            .join(", ")}
                        </td>
                      </tr>
                    </tbody>
                  </table>
                  <div className="mb-actions">
                    {targets.length > 0 && (
                      <button className="btn-cyan" disabled={busy} onClick={() => void load(true)}>
                        Change Selected ({targets.length})
                      </button>
                    )}
                    <button
                      className={targets.length ? "btn-outline" : "btn-cyan"}
                      disabled={busy}
                      onClick={() => void load(false)}
                    >
                      Place
                    </button>
                    <button
                      className="btn-outline"
                      disabled={busy}
                      onClick={() => void load(false, false)}
                    >
                      Load
                    </button>
                  </div>
                  <p className="muted">
                    Click in a plan or the 3D view to place it, again and again; Esc stops. Random
                    rotation and size are on the options bar.
                  </p>
                </>
              ) : (
                <p className="muted">
                  Pick an asset:{" "}
                  {lib ? lib.presets.filter((p) => p.name === p.species).length : "…"} trees,
                  bushes, grasses, flowers and succulents, with spring, summer, autumn and winter
                  variants of the deciduous trees.
                </p>
              )}
            </aside>
          </div>
        ) : (
          <div className="al-body al-project">
            <div className="al-main">
              <div className="al-grid" role="list" aria-label="Project plants">
                {types
                  .filter((t) => !q || t.name.toLowerCase().includes(q))
                  .sort((a, b) => a.name.localeCompare(b.name))
                  .map((t) => (
                    <button
                      key={t.id}
                      role="listitem"
                      className={`al-card${current === t.id ? " active" : ""}`}
                      onClick={() => setPicked(t.id)}
                      onDoubleClick={() => void finishWith(t.id, targets.length > 0)}
                      title={t.name}
                    >
                      <Thumb source={{ Type: t.id }} label={t.name} />
                      <span className="al-name">{t.name}</span>
                    </button>
                  ))}
                {types.length === 0 && (
                  <p className="muted">No plants yet: pick some from Vegetation.</p>
                )}
              </div>
            </div>
            <aside className="al-detail">
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
                      Place
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
                </>
              ) : (
                <p className="muted">Pick a plant.</p>
              )}
            </aside>
          </div>
        )}
      </div>
    </div>
  );
}
