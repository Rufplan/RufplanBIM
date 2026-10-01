import { useEffect, useState } from "react";
import type { ElementId } from "../bindings/ElementId";
import type { FfeClass } from "../bindings/FfeClass";
import type { FfeLibrary } from "../bindings/FfeLibrary";
import type { FfePreset } from "../bindings/FfePreset";
import type { FfeSource } from "../bindings/FfeSource";
import type { FfeSpec } from "../bindings/FfeSpec";
import type { FfeUse } from "../bindings/FfeUse";
import { apply } from "../fileActions";
import { errorMessage, ipc } from "../ipc";
import { thumbnail } from "../render/thumbs";
import { useAppStore } from "../store";

// The furniture and equipment pickers (ADR-090), like the lighting fixture picker: the
// project's types as rendered thumbnails, and the library of the most common pieces in
// residential, multifamily and hospitality projects. Pick one to place it, or to change
// the selected pieces to it.

const libraries: Partial<Record<FfeClass, Promise<FfeLibrary>>> = {};
const loadLibrary = (cls: FfeClass) => {
  const p = (libraries[cls] ??= ipc.ffeLibrary(cls));
  p.catch(() => delete libraries[cls]);
  return p;
};

const MM_PER_IN = 25.4;
/** Feet-inches for sizes: 84" → 7'-0", 30" stays 30". */
export function ffeLength(mm: number): string {
  const i = Math.round((mm / MM_PER_IN) * 4) / 4;
  if (i < 48) return `${i}"`;
  const ft = Math.floor(i / 12);
  return `${ft}'-${Math.round((i - ft * 12) * 4) / 4}"`;
}

/** A piece's size and mounting, as (label, value). */
export function ffeSpecs(s: FfeSpec): [string, string][] {
  const rows: [string, string][] = [
    ["Width", ffeLength(s.width)],
    ["Depth", ffeLength(s.depth)],
    ["Height", ffeLength(s.height)],
    ["Mounting", s.mount === "Floor" ? "Floor" : s.mount === "Wall" ? "Wall" : "Counter"],
  ];
  if (s.mount !== "Floor") rows.push(["Mounting height", ffeLength(s.mount_height)]);
  return rows;
}

/** A rendered thumbnail, drawn on demand. */
function Thumb({ source, label, big }: { source: FfeSource; label: string; big?: boolean }) {
  const revision = useAppStore((s) => s.app?.revision ?? 0);
  const key = "Type" in source ? `ffe:${source.Type}:${revision}` : `ffe:${JSON.stringify(source)}`;
  const [url, setUrl] = useState<{ key: string; url: string | null } | null>(null);
  useEffect(() => {
    let live = true;
    thumbnail(key, () => ipc.ffeThumbnail(source)).then(
      (u) => live && setUrl({ key, url: u }),
      () => live && setUrl({ key, url: null }),
    );
    return () => {
      live = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key]);
  const cls = `tp-thumb lp-thumb ffe-thumb${big ? " big" : ""}`;
  if (url?.key === key && url.url)
    return <img className={cls} src={url.url} alt={label} draggable={false} />;
  return (
    <div className={`${cls} tp-rendering`} role="img" aria-label={label}>
      {url?.key === key ? "" : "Rendering…"}
    </div>
  );
}

type Tab = "project" | "library";

export function FfePicker({ cls, onClose }: { cls: FfeClass; onClose: () => void }) {
  const app = useAppStore((s) => s.app);
  const picker = useAppStore((s) => s.picker);
  const targets = picker?.change ?? [];
  const [tab, setTab] = useState<Tab>(picker?.tab ?? "project");
  const [lib, setLib] = useState<FfeLibrary | null>(null);
  const [picked, setPicked] = useState<ElementId | null>(null);
  const [group, setGroup] = useState<string | null>(null);
  const [use, setUse] = useState<FfeUse | null>(null);
  const [preset, setPreset] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [layout, setLayout] = useState<"grid" | "list">("grid");
  const [busy, setBusy] = useState(false);
  const furniture = cls === "Furniture";
  const slot = furniture ? "furniture" : "equipment";
  const noun = furniture ? "furniture" : "equipment";
  const title = furniture ? "Furniture" : "Equipment";

  useEffect(() => {
    let live = true;
    loadLibrary(cls).then(
      (l) => live && setLib(l),
      (e) => useAppStore.getState().setError(errorMessage(e)),
    );
    return () => {
      live = false;
    };
  }, [cls]);

  const types = (furniture ? app?.furnitureTypes : app?.equipmentTypes) ?? [];
  const toolType = useAppStore((s) => s.toolTypes[slot]);
  const q = query.trim().toLowerCase();
  const shownTypes = types
    .filter((t) => !q || t.name.toLowerCase().includes(q))
    .sort((a, b) => a.name.localeCompare(b.name));
  const current = picked ?? toolType;
  const presetOf = (name: string) => lib?.presets.find((p) => p.name === name);
  const shownPresets: FfePreset[] = (lib?.presets ?? []).filter(
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
    s.setToolType(slot, id);
    s.setTool(slot);
    onClose();
  };
  const loadChosen = async (change: boolean, andUse = true) => {
    if (!chosen) return;
    setBusy(true);
    try {
      const r = await ipc.loadFfeTypes([chosen.name]);
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
    source: (t: T) => FfeSource,
    active: (t: T) => boolean,
    pick: (t: T) => void,
    go: (t: T) => void,
    note?: (t: T) => string,
    badge?: (t: T) => boolean,
  ) => (
    <div
      className={layout === "grid" ? "mb-grid tp-grid" : "lp-list"}
      role="list"
      aria-label={tab === "project" ? `Project ${noun}` : `Library ${noun}`}
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
      {items.length === 0 && <p className="muted">Nothing matches.</p>}
    </div>
  );

  const usesOf = (p: FfePreset) =>
    p.uses.map((u) => lib?.uses.find((x) => x.id === u)?.label ?? u).join(", ");

  return (
    <div className="modal-backdrop" role="dialog" aria-label={`${title} Types`}>
      <div className="modal material-browser type-picker light-picker">
        <div className="mb-head">
          <h2>{title}</h2>
          <div className="mb-scope" role="tablist">
            {(["project", "library"] as const).map((t) => (
              <button
                key={t}
                role="tab"
                aria-selected={tab === t}
                className={`mb-tab${tab === t ? " active" : ""}`}
                onClick={() => setTab(t)}
              >
                {t === "project" ? `In This Project (${types.length})` : `${title} Library`}
              </button>
            ))}
          </div>
          <input
            className="mb-search"
            aria-label={`Search ${noun}`}
            placeholder={
              furniture ? "Search sofa, bed, dining, lounge…" : "Search range, washer, fitness…"
            }
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
          {targets.length > 0 && (
            <span className="tp-mode">
              Changing {targets.length} selected {targets.length === 1 ? "piece" : "pieces"}
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
                      Place {title}
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
                    Double-click a type to {targets.length ? "apply it" : "place it"}. The most
                    common {noun} for homes, apartments and hotels is in the {title} Library.
                  </p>
                </>
              ) : (
                <p className="muted">
                  {types.length ? "Pick a type." : `Load ${noun} from the ${title} Library.`}
                </p>
              )}
            </aside>
          </div>
        ) : (
          <div className="mb-body">
            <nav className="mb-filters wl-families" aria-label="Groups">
              <h3>{title}</h3>
              {[null, ...(lib?.groups ?? [])].map((g) => (
                <button
                  key={g ?? "all"}
                  className={`mb-cat${group === g ? " active" : ""}`}
                  onClick={() => setGroup(g)}
                >
                  {g ?? `All ${title}`}
                </button>
              ))}
              <h3>Project Type</h3>
              <select
                aria-label="Project type"
                value={use ?? ""}
                onChange={(e) => setUse((e.target.value || null) as FfeUse | null)}
              >
                <option value="">All project types</option>
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
                  <table className="lp-specs" aria-label="Size">
                    <tbody>
                      {ffeSpecs(chosen.spec).map(([k, v]) => (
                        <tr key={k}>
                          <th>{k}</th>
                          <td>{v}</td>
                        </tr>
                      ))}
                      <tr>
                        <th>Group</th>
                        <td>{chosen.group}</td>
                      </tr>
                      <tr>
                        <th>Typical in</th>
                        <td>{usesOf(chosen)}</td>
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
                  Pick a piece: {lib?.presets.length ?? "…"} of the most common {noun} pieces in
                  residential, multifamily and hospitality projects.
                </p>
              )}
            </aside>
          </div>
        )}
      </div>
    </div>
  );
}
