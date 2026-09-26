import { useEffect, useMemo, useState } from "react";
import type { Appearance } from "../bindings/Appearance";
import type { Preset } from "../bindings/Preset";
import type { RenderMaterial } from "../bindings/RenderMaterial";
import type { Tier } from "../bindings/Tier";
import { apply } from "../fileActions";
import { errorMessage, ipc } from "../ipc";
import { useAppStore } from "../store";

// The Material Browser (ADR-029): a V-Ray-style library of US residential, hospitality and
// multifamily materials, previewed on path-traced cubes, and the project's own materials.

export const TIER_LABEL: Record<Tier, string> = {
  Typical: "Typical",
  MidRange: "Mid-range",
  HighEnd: "High-end",
};

let library: Promise<Preset[]> | null = null;
function loadLibrary(): Promise<Preset[]> {
  if (!library) {
    library = ipc.materialLibrary();
    library.catch(() => (library = null));
  }
  return library;
}

/** The bundled preview of a library material. */
export const presetThumb = (id: string) => `/materials/${id}.webp`;

/** Whether a project material still looks exactly like the preset it came from. */
export function matchesPreset(m: RenderMaterial, p: Preset | undefined): boolean {
  return (
    !!p &&
    m.color.join() === p.color.join() &&
    JSON.stringify(m.appearance) === JSON.stringify(p.appearance)
  );
}

// Previews of edited materials, rendered one at a time and kept for the session.
const previews = new Map<string, Promise<string>>();
let queue: Promise<unknown> = Promise.resolve();
function previewKey(color: [number, number, number], a: Appearance) {
  return JSON.stringify([color, a]);
}
export function renderedPreview(color: [number, number, number], a: Appearance): Promise<string> {
  const key = previewKey(color, a);
  let p = previews.get(key);
  if (!p) {
    p = queue.then(async () => {
      const [{ renderPreview }, { texturesFor }] = await Promise.all([
        import("../render/preview"),
        import("../render/materials"),
      ]);
      const tex = await texturesFor(a, color, (set, map) => ipc.materialTexture(set, map)).catch(
        () => null,
      );
      const canvas = await renderPreview(a, color, tex, 192, 160);
      return canvas.toDataURL("image/webp", 0.9);
    });
    queue = p.catch(() => {});
    p.catch(() => previews.delete(key));
    previews.set(key, p);
  }
  return p;
}

function Thumb({
  src,
  material,
  label,
}: {
  src: string | null;
  material?: RenderMaterial;
  label: string;
}) {
  if (src || !material)
    return <img className="mb-thumb" src={src ?? ""} alt={label} draggable={false} />;
  return (
    <RenderedThumb
      key={previewKey(material.color, material.appearance)}
      material={material}
      label={label}
    />
  );
}

/** An edited material's preview, path-traced on demand (one at a time). */
function RenderedThumb({ material, label }: { material: RenderMaterial; label: string }) {
  const [url, setUrl] = useState<string | null>(null);
  useEffect(() => {
    let live = true;
    renderedPreview(material.color, material.appearance).then(
      (u) => live && setUrl(u),
      () => {},
    );
    return () => {
      live = false;
    };
  }, [material]);
  return url ? (
    <img className="mb-thumb" src={url} alt={label} draggable={false} />
  ) : (
    <div className="mb-thumb mb-rendering" aria-label={`${label} (rendering preview)`}>
      Rendering…
    </div>
  );
}

type Scope = "library" | "project";
type Sector = "residential" | "hospitality" | "multifamily";

export function MaterialBrowser({ onClose }: { onClose: () => void }) {
  const app = useAppStore((s) => s.app);
  const selection = useAppStore((s) => s.selection);
  const revision = app?.revision ?? 0;
  const [presets, setPresets] = useState<Preset[]>([]);
  const [project, setProject] = useState<RenderMaterial[]>([]);
  const [scope, setScope] = useState<Scope>("library");
  const [category, setCategory] = useState("All");
  const [sectors, setSectors] = useState<Set<Sector>>(new Set());
  const [tiers, setTiers] = useState<Set<Tier>>(new Set());
  const [query, setQuery] = useState("");
  const [picked, setPicked] = useState<string | null>(null);
  // Elements to apply to: the selection when the browser opened (materials excluded).
  const [targets] = useState(() =>
    selection.filter((id) => !useAppStore.getState().app?.materials.some((m) => m.id === id)),
  );

  useEffect(() => {
    let live = true;
    loadLibrary().then(
      (l) => live && setPresets(l),
      (e) => useAppStore.getState().setError(errorMessage(e)),
    );
    return () => {
      live = false;
    };
  }, []);
  useEffect(() => {
    let live = true;
    ipc.renderMaterials().then(
      (m) => live && setProject(m),
      () => {},
    );
    return () => {
      live = false;
    };
  }, [revision]);

  const byId = useMemo(() => new Map(presets.map((p) => [p.id, p])), [presets]);
  const categories = useMemo(
    () => ["All", ...presets.map((p) => p.category).filter((c, i, a) => a.indexOf(c) === i)],
    [presets],
  );
  const q = query.trim().toLowerCase();
  const shownPresets = presets.filter(
    (p) =>
      (category === "All" || p.category === category) &&
      (tiers.size === 0 || tiers.has(p.tier)) &&
      (sectors.size === 0 || [...sectors].some((s) => p[s])) &&
      (!q || `${p.name} ${p.category} ${p.description}`.toLowerCase().includes(q)),
  );
  const shownProject = project
    .filter((m) => !q || m.name.toLowerCase().includes(q))
    .sort((a, b) => a.name.localeCompare(b.name));

  const toggle = <T,>(set: Set<T>, v: T, put: (s: Set<T>) => void) => {
    const next = new Set(set);
    if (next.has(v)) next.delete(v);
    else next.add(v);
    put(next);
  };

  const pickedPreset = scope === "library" ? byId.get(picked ?? "") : undefined;
  const pickedMaterial = scope === "project" ? project.find((m) => m.id === picked) : undefined;
  const sourcePreset = pickedMaterial?.appearance.preset
    ? byId.get(pickedMaterial.appearance.preset)
    : undefined;

  /** Adds a preset; returns the new project material's id. */
  const addPreset = async (id: string): Promise<string | null> => {
    const before = new Set(project.map((m) => m.id));
    if (!(await apply(() => ipc.addLibraryMaterial(id)))) return null;
    const list = await ipc.renderMaterials();
    setProject(list);
    return list.find((m) => !before.has(m.id))?.id ?? null;
  };
  const applyTo = async (material: string) => {
    if (await apply(() => ipc.applyMaterial(targets, material))) onClose();
  };

  return (
    <div className="modal-backdrop" role="dialog" aria-label="Material Browser">
      <div className="modal material-browser">
        <div className="mb-head">
          <h2>Material Browser</h2>
          <div className="mb-scope" role="tablist">
            {(["library", "project"] as const).map((s) => (
              <button
                key={s}
                role="tab"
                aria-selected={scope === s}
                className={`mb-tab${scope === s ? " active" : ""}`}
                onClick={() => {
                  setScope(s);
                  setPicked(null);
                }}
              >
                {s === "library"
                  ? `Library (${presets.length})`
                  : `In This Project (${project.length})`}
              </button>
            ))}
          </div>
          <input
            className="mb-search"
            aria-label="Search materials"
            placeholder="Search oak, marble, brass…"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
          <button className="btn-ghost" onClick={onClose} aria-label="Close">
            ×
          </button>
        </div>
        <div className="mb-body">
          {scope === "library" && (
            <nav className="mb-filters" aria-label="Filters">
              <h3>Category</h3>
              {categories.map((c) => (
                <button
                  key={c}
                  className={`mb-cat${category === c ? " active" : ""}`}
                  onClick={() => setCategory(c)}
                >
                  {c}
                </button>
              ))}
              <h3>Building type</h3>
              {(["residential", "hospitality", "multifamily"] as const).map((s) => (
                <label key={s} className="ob-check">
                  <input
                    type="checkbox"
                    checked={sectors.has(s)}
                    onChange={() => toggle(sectors, s, setSectors)}
                  />
                  {s[0]!.toUpperCase() + s.slice(1)}
                </label>
              ))}
              <h3>Finish level</h3>
              {(["Typical", "MidRange", "HighEnd"] as const).map((t) => (
                <label key={t} className="ob-check">
                  <input
                    type="checkbox"
                    checked={tiers.has(t)}
                    onChange={() => toggle(tiers, t, setTiers)}
                  />
                  {TIER_LABEL[t]}
                </label>
              ))}
            </nav>
          )}
          <div className="mb-grid" role="list">
            {scope === "library"
              ? shownPresets.map((p) => (
                  <button
                    key={p.id}
                    role="listitem"
                    className={`mb-card${picked === p.id ? " active" : ""}`}
                    onClick={() => setPicked(p.id)}
                    onDoubleClick={() => void addPreset(p.id)}
                    title={p.description}
                  >
                    <Thumb src={presetThumb(p.id)} label={p.name} />
                    <span className="mb-name">{p.name}</span>
                    <span className={`mb-tier t-${p.tier}`}>{TIER_LABEL[p.tier]}</span>
                  </button>
                ))
              : shownProject.map((m) => {
                  const p = m.appearance.preset ? byId.get(m.appearance.preset) : undefined;
                  return (
                    <button
                      key={m.id}
                      role="listitem"
                      className={`mb-card${picked === m.id ? " active" : ""}`}
                      onClick={() => setPicked(m.id)}
                    >
                      <Thumb
                        src={matchesPreset(m, p) ? presetThumb(p!.id) : null}
                        material={m}
                        label={m.name}
                      />
                      <span className="mb-name">{m.name}</span>
                    </button>
                  );
                })}
            {scope === "library" && shownPresets.length === 0 && (
              <p className="muted">No library materials match.</p>
            )}
          </div>
          <aside className="mb-detail">
            {pickedPreset && (
              <>
                <Thumb src={presetThumb(pickedPreset.id)} label={pickedPreset.name} />
                <h3>{pickedPreset.name}</h3>
                <p className="mb-meta">
                  {pickedPreset.category} · {TIER_LABEL[pickedPreset.tier]} ·{" "}
                  {[
                    pickedPreset.residential && "Residential",
                    pickedPreset.hospitality && "Hospitality",
                    pickedPreset.multifamily && "Multifamily",
                  ]
                    .filter(Boolean)
                    .join(", ")}
                </p>
                <p>{pickedPreset.description}</p>
                <Settings a={pickedPreset.appearance} />
                <div className="mb-actions">
                  <button className="btn-cyan" onClick={() => void addPreset(pickedPreset.id)}>
                    Add to Project
                  </button>
                  {targets.length > 0 && (
                    <button
                      className="btn-outline"
                      onClick={async () => {
                        const id = await addPreset(pickedPreset.id);
                        if (id) await applyTo(id);
                      }}
                    >
                      Add &amp; Apply to Selection
                    </button>
                  )}
                </div>
              </>
            )}
            {pickedMaterial && (
              <>
                <Thumb
                  src={
                    matchesPreset(pickedMaterial, sourcePreset)
                      ? presetThumb(sourcePreset!.id)
                      : null
                  }
                  material={pickedMaterial}
                  label={pickedMaterial.name}
                />
                <h3>{pickedMaterial.name}</h3>
                {sourcePreset && (
                  <p className="mb-meta">
                    From the library: {sourcePreset.name}
                    {matchesPreset(pickedMaterial, sourcePreset) ? "" : " (edited)"}
                  </p>
                )}
                <Settings a={pickedMaterial.appearance} />
                <div className="mb-actions">
                  {targets.length > 0 && (
                    <button className="btn-cyan" onClick={() => void applyTo(pickedMaterial.id)}>
                      Apply to Selection ({targets.length})
                    </button>
                  )}
                  <button
                    className="btn-outline"
                    onClick={() => {
                      useAppStore.getState().select([pickedMaterial.id]);
                      onClose();
                    }}
                  >
                    Edit in Properties
                  </button>
                  <button
                    className="btn-outline"
                    onClick={() => void apply(() => ipc.createMaterial(pickedMaterial.id))}
                  >
                    Duplicate
                  </button>
                </div>
              </>
            )}
            {!pickedPreset && !pickedMaterial && (
              <p className="muted">
                {scope === "library"
                  ? "Pick a material to see it, then add it to the project. Double-click adds it."
                  : "Pick a project material to apply it to the selection or edit it."}
                {targets.length > 0
                  ? ` ${targets.length} selected element${targets.length > 1 ? "s" : ""} will take the material on their outside finish (every element of their type).`
                  : ""}
              </p>
            )}
            <p className="muted mb-credit">
              Photo textures: Poly Haven (CC0), 2K, downloaded the first time they&apos;re used.
            </p>
          </aside>
        </div>
      </div>
    </div>
  );
}

function Settings({ a }: { a: Appearance }) {
  const rows: [string, string][] = [
    ["Reflection glossiness", (1 - a.roughness).toFixed(2)],
    ["Reflection", a.reflection.toFixed(2)],
  ];
  if (a.metalness > 0) rows.push(["Metalness", a.metalness.toFixed(2)]);
  if (a.refraction > 0)
    rows.push(["Refraction", `${a.refraction.toFixed(2)} (IOR ${a.ior.toFixed(2)})`]);
  if (a.coat > 0) rows.push(["Coat", a.coat.toFixed(2)]);
  if (a.sheen > 0) rows.push(["Sheen", a.sheen.toFixed(2)]);
  if (a.texture)
    rows.push([
      "Texture",
      `${a.texture.startsWith("proc:") ? "Procedural" : "Photo, 2K"} · ${Math.round(a.scale / 25.4)}" tile`,
    ]);
  return (
    <dl className="mb-settings">
      {rows.map(([k, v]) => (
        <div key={k}>
          <dt>{k}</dt>
          <dd>{v}</dd>
        </div>
      ))}
    </dl>
  );
}
