import { useEffect, useMemo, useState } from "react";
import type { DoorFamily } from "../bindings/DoorFamily";
import type { DoorFinish } from "../bindings/DoorFinish";
import type { DoorLibrary } from "../bindings/DoorLibrary";
import type { DoorSpec } from "../bindings/DoorSpec";
import type { ElementId } from "../bindings/ElementId";
import type { FrameFinish } from "../bindings/FrameFinish";
import type { Grille } from "../bindings/Grille";
import type { LeafStyle } from "../bindings/LeafStyle";
import type { ThumbSource } from "../bindings/ThumbSource";
import type { WindowFamily } from "../bindings/WindowFamily";
import type { WindowLibrary } from "../bindings/WindowLibrary";
import type { WindowSpec } from "../bindings/WindowSpec";
import { apply } from "../fileActions";
import { errorMessage, ipc } from "../ipc";
import { thumbnail } from "../render/thumbs";
import { useAppStore, type PickerCategory } from "../store";

// The type picker (ADR-033): the project's door or window types as rendered thumbnails, and
// the library of US families and sizes. Pick one to place it, or to change the selected
// doors or windows to it.

const MM_PER_IN = 25.4;
const hex = (c: [number, number, number]) =>
  `#${c.map((v) => v.toString(16).padStart(2, "0")).join("")}`;

let doorLib: Promise<DoorLibrary> | null = null;
let windowLib: Promise<WindowLibrary> | null = null;
const libraries = () => {
  doorLib ??= ipc.doorLibrary();
  windowLib ??= ipc.windowLibrary();
  doorLib.catch(() => (doorLib = null));
  windowLib.catch(() => (windowLib = null));
  return Promise.all([doorLib, windowLib]);
};

/** A rendered thumbnail, drawn on demand. */
export function Thumb({
  source,
  label,
  big,
}: {
  source: ThumbSource;
  label: string;
  big?: boolean;
}) {
  const revision = useAppStore((s) => s.app?.revision ?? 0);
  const key = "Type" in source ? `type:${source.Type}:${revision}` : JSON.stringify(source);
  const [url, setUrl] = useState<{ key: string; url: string | null } | null>(null);
  useEffect(() => {
    let live = true;
    thumbnail(key, () => ipc.openingThumbnail(source)).then(
      (u) => live && setUrl({ key, url: u }),
      () => live && setUrl({ key, url: null }),
    );
    return () => {
      live = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key]);
  const cls = `tp-thumb${big ? " big" : ""}`;
  if (url?.key === key && url.url)
    return <img className={cls} src={url.url} alt={label} draggable={false} />;
  return (
    <div className={`${cls} tp-rendering`} role="img" aria-label={label}>
      {url?.key === key ? "" : "Rendering…"}
    </div>
  );
}

type Tab = "project" | "library";

export function TypePicker({ onClose }: { onClose: () => void }) {
  const app = useAppStore((s) => s.app);
  const picker = useAppStore((s) => s.picker);
  const category: PickerCategory = picker?.category ?? "Door";
  const targets = picker?.change ?? [];
  const [tab, setTab] = useState<Tab>(picker?.tab ?? "project");
  const [libs, setLibs] = useState<[DoorLibrary, WindowLibrary] | null>(null);
  const [picked, setPicked] = useState<ElementId | null>(null);
  const [family, setFamily] = useState<string | null>(null);
  const [preset, setPreset] = useState<number | null>(null);
  const [query, setQuery] = useState("");
  // Library options.
  const [leaf, setLeaf] = useState<LeafStyle | null>(null);
  const [panels, setPanels] = useState<number | null>(null);
  const [doorFinish, setDoorFinish] = useState<DoorFinish | null>(null);
  const [grille, setGrille] = useState<Grille>("None");
  const [winFinish, setWinFinish] = useState<FrameFinish>("White");
  const [custom, setCustom] = useState({ width: "", height: "", sill: "" });
  const [name, setName] = useState<string>("");
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    let live = true;
    libraries().then(
      (l) => live && setLibs(l),
      (e) => useAppStore.getState().setError(errorMessage(e)),
    );
    return () => {
      live = false;
    };
  }, []);

  const types = category === "Door" ? (app?.doorTypes ?? []) : (app?.windowTypes ?? []);
  const toolType = useAppStore((s) =>
    category === "Door" ? s.toolTypes.door : s.toolTypes.window,
  );
  const q = query.trim().toLowerCase();
  const shownTypes = types
    .filter((t) => !q || t.name.toLowerCase().includes(q))
    .sort((a, b) => a.name.localeCompare(b.name));
  const current = picked ?? toolType;

  // Library: families and sizes of the category.
  const doorLibrary = libs?.[0];
  const windowLibrary = libs?.[1];
  const families = useMemo(
    () =>
      category === "Door"
        ? (doorLibrary?.families.map((f) => ({
            id: f.family as string,
            label: f.label,
            description: f.description,
          })) ?? [])
        : (windowLibrary?.families.map((f) => ({
            id: f.family as string,
            label: f.label,
            description: f.description,
          })) ?? []),
    [category, doorLibrary, windowLibrary],
  );
  const fam = family ?? families[0]?.id ?? null;
  const doorFam = doorLibrary?.families.find((f) => f.family === fam);
  const winFam = windowLibrary?.families.find((f) => f.family === fam);
  const sizes =
    category === "Door"
      ? (doorLibrary?.presets.filter((p) => p.spec.family === fam) ?? []).map((p) => ({
          name: p.name,
          door: p.spec,
          window: null,
        }))
      : (windowLibrary?.presets.filter((p) => p.spec.family === fam) ?? []).map((p) => ({
          name: p.name,
          door: null,
          window: p.spec,
        }));

  const withDoor = (s: DoorSpec): DoorSpec => ({
    ...s,
    leaf: doorFam?.leaves.some((l) => l.id === leaf) ? leaf! : s.leaf,
    panels: panels ?? s.panels,
    finish: doorFinish ?? s.finish,
  });
  const withWindow = (s: WindowSpec): WindowSpec => ({
    ...s,
    grille: winFam?.grilles ? grille : "None",
    finish: winFinish,
  });
  const customSpec = (): ThumbSource | null => {
    const [w, h, sill] = [custom.width, custom.height, custom.sill].map(Number);
    if (!(w! > 0) || !(h! > 0) || !fam) return null;
    if (category === "Door")
      return {
        Door: withDoor({
          family: fam as DoorFamily,
          leaf: doorFam?.leaves[0]?.id ?? "Flush",
          panels: 0,
          width: w! * MM_PER_IN,
          height: h! * MM_PER_IN,
          finish: null,
        }),
      };
    return {
      Window: withWindow({
        family: fam as WindowFamily,
        units: 1,
        width: w! * MM_PER_IN,
        height: h! * MM_PER_IN,
        sill: (custom.sill && Number.isFinite(sill) ? sill! : Math.max(84 - h!, 12)) * MM_PER_IN,
        grille: "None",
        finish: "White",
      }),
    };
  };
  const chosen: ThumbSource | null =
    preset !== null && sizes[preset]
      ? sizes[preset].door
        ? { Door: withDoor(sizes[preset].door!) }
        : { Window: withWindow(sizes[preset].window!) }
      : customSpec();
  const chosenKey = chosen ? JSON.stringify(chosen) : "";
  useEffect(() => {
    if (!chosen) return;
    let live = true;
    const p =
      "Door" in chosen
        ? ipc.doorPreview(chosen.Door)
        : "Window" in chosen
          ? ipc.windowPreview(chosen.Window)
          : null;
    p?.then(
      (r) => live && setName(r.name),
      () => {},
    );
    return () => {
      live = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [chosenKey]);

  const kind = category === "Door" ? "door" : "window";
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
    s.setToolType(kind, id);
    s.setTool(kind);
    onClose();
  };
  const loadChosen = async (change: boolean, andUse = true) => {
    if (!chosen) return;
    setBusy(true);
    try {
      const r =
        "Door" in chosen
          ? await ipc.loadDoorTypes([chosen.Door])
          : "Window" in chosen
            ? await ipc.loadWindowTypes([chosen.Window])
            : null;
      if (!r) return;
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

  const noun = category === "Door" ? "Doors" : "Windows";
  const finishes =
    category === "Door"
      ? (doorLibrary?.finishes.map((f) => ({
          id: f.id as string,
          label: f.label,
          color: f.color,
        })) ?? [])
      : (windowLibrary?.finishes.map((f) => ({
          id: f.id as string,
          label: f.label,
          color: f.color,
        })) ?? []);
  const activeFinish =
    category === "Door"
      ? (doorFinish ??
        (chosen && "Door" in chosen ? chosen.Door.finish : null) ??
        doorFam?.defaultFinish ??
        null)
      : winFinish;

  return (
    <div className="modal-backdrop" role="dialog" aria-label={`${category} Types`}>
      <div className="modal material-browser type-picker">
        <div className="mb-head">
          <h2>{category} Types</h2>
          <div className="mb-scope" role="tablist">
            {(["project", "library"] as const).map((t) => (
              <button
                key={t}
                role="tab"
                aria-selected={tab === t}
                className={`mb-tab${tab === t ? " active" : ""}`}
                onClick={() => setTab(t)}
              >
                {t === "project" ? `In This Project (${types.length})` : `${noun} Library`}
              </button>
            ))}
          </div>
          {tab === "project" && (
            <input
              className="mb-search"
              aria-label={`Search ${noun.toLowerCase()}`}
              placeholder={
                category === "Door" ? "Search shaker, sliding, 36…" : "Search double hung, 36…"
              }
              value={query}
              onChange={(e) => setQuery(e.target.value)}
            />
          )}
          {targets.length > 0 && (
            <span className="tp-mode">
              Changing {targets.length} selected {targets.length === 1 ? kind : `${kind}s`}
            </span>
          )}
          <button className="btn-ghost" onClick={onClose} aria-label="Close">
            ×
          </button>
        </div>
        {tab === "project" ? (
          <div className="mb-body">
            <div
              className="mb-grid tp-grid"
              role="list"
              aria-label={`Project ${noun.toLowerCase()}`}
            >
              {shownTypes.map((t) => (
                <button
                  key={t.id}
                  role="listitem"
                  className={`mb-card${current === t.id ? " active" : ""}`}
                  onClick={() => setPicked(t.id)}
                  onDoubleClick={() => void finishWith(t.id, targets.length > 0)}
                  title={t.name}
                >
                  <Thumb source={{ Type: t.id }} label={t.name} />
                  <span className="mb-name">{t.name}</span>
                </button>
              ))}
              {shownTypes.length === 0 && <p className="muted">No {noun.toLowerCase()} match.</p>}
            </div>
            <aside className="mb-detail">
              {current ? (
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
                      Place {category}
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
                    Double-click a type to {targets.length ? "apply it" : "place it"}. More sizes
                    and styles are in the {noun} Library.
                  </p>
                </>
              ) : (
                <p className="muted">Pick a type.</p>
              )}
            </aside>
          </div>
        ) : (
          <div className="mb-body">
            <nav className="mb-filters wl-families" aria-label="Families">
              <h3>Family</h3>
              {families.map((f) => (
                <button
                  key={f.id}
                  className={`mb-cat${fam === f.id ? " active" : ""}`}
                  onClick={() => {
                    setFamily(f.id);
                    setPreset(null);
                    setLeaf(null);
                    setPanels(null);
                  }}
                  title={f.description}
                >
                  {f.label}
                </button>
              ))}
            </nav>
            <div className="mb-grid tp-grid" role="list" aria-label="Sizes">
              {sizes.map((s, i) => {
                const src: ThumbSource = s.door
                  ? { Door: withDoor(s.door) }
                  : { Window: withWindow(s.window!) };
                return (
                  <button
                    key={s.name}
                    role="listitem"
                    className={`mb-card${preset === i ? " active" : ""}`}
                    onClick={() => setPreset(i)}
                    onDoubleClick={() => {
                      setPreset(i);
                      void loadChosen(targets.length > 0);
                    }}
                    title={s.name}
                  >
                    <Thumb source={src} label={s.name} />
                    <span className="mb-name">{s.name}</span>
                    {types.some((t) => t.name === s.name) && (
                      <span className="mb-tier wl-loaded">In project</span>
                    )}
                  </button>
                );
              })}
            </div>
            <aside className="mb-detail">
              {chosen && <Thumb source={chosen} label={name} big />}
              <h3>{chosen ? name : (families.find((f) => f.id === fam)?.label ?? "")}</h3>
              <p>{families.find((f) => f.id === fam)?.description}</p>
              <div className="wl-options">
                {category === "Door" && doorFam && doorFam.leaves.length > 0 && (
                  <label>
                    Leaf style
                    <select
                      value={
                        leaf ??
                        (chosen && "Door" in chosen ? chosen.Door.leaf : doorFam.leaves[0]!.id)
                      }
                      onChange={(e) => setLeaf(e.target.value as LeafStyle)}
                    >
                      {doorFam.leaves.map((l) => (
                        <option key={l.id} value={l.id}>
                          {l.label}
                        </option>
                      ))}
                    </select>
                  </label>
                )}
                {category === "Door" && doorFam?.panels && (
                  <label>
                    {doorFam.panelsLabel}
                    <select
                      value={
                        panels ??
                        (chosen && "Door" in chosen ? chosen.Door.panels : doorFam.panels[0])
                      }
                      onChange={(e) => setPanels(Number(e.target.value))}
                    >
                      {Array.from(
                        { length: doorFam.panels[1] - doorFam.panels[0] + 1 },
                        (_, i) => doorFam.panels![0] + i,
                      ).map((n) => (
                        <option key={n} value={n}>
                          {n}
                        </option>
                      ))}
                    </select>
                  </label>
                )}
                {category === "Window" && (
                  <label>
                    Grille
                    <select
                      value={winFam?.grilles ? grille : "None"}
                      disabled={!winFam?.grilles}
                      onChange={(e) => setGrille(e.target.value as Grille)}
                    >
                      {windowLibrary?.grilles.map((g) => (
                        <option key={g.id} value={g.id}>
                          {g.label}
                        </option>
                      ))}
                    </select>
                  </label>
                )}
                <div className="wl-finishes" role="radiogroup" aria-label="Finish">
                  {finishes.map((f) => (
                    <button
                      key={f.id}
                      role="radio"
                      aria-checked={activeFinish === f.id}
                      className={`wl-swatch${activeFinish === f.id ? " active" : ""}`}
                      style={{ background: hex(f.color) }}
                      title={f.label}
                      aria-label={f.label}
                      onClick={() =>
                        category === "Door"
                          ? setDoorFinish(f.id as DoorFinish)
                          : setWinFinish(f.id as FrameFinish)
                      }
                    />
                  ))}
                </div>
              </div>
              <h3>Custom size</h3>
              <div className="wl-custom">
                {(category === "Door"
                  ? (["width", "height"] as const)
                  : (["width", "height", "sill"] as const)
                ).map((k) => (
                  <label key={k}>
                    {k === "sill" ? "Sill" : k[0]!.toUpperCase() + k.slice(1)} (in)
                    <input
                      type="number"
                      min={0}
                      value={custom[k]}
                      placeholder={k === "sill" ? "head 84" : ""}
                      onChange={(e) => {
                        setCustom({ ...custom, [k]: e.target.value });
                        setPreset(null);
                      }}
                    />
                  </label>
                ))}
              </div>
              <div className="mb-actions">
                {targets.length > 0 && (
                  <button
                    className="btn-cyan"
                    disabled={!chosen || busy}
                    onClick={() => void loadChosen(true)}
                  >
                    Load &amp; Change Selected
                  </button>
                )}
                <button
                  className={targets.length ? "btn-outline" : "btn-cyan"}
                  disabled={!chosen || busy}
                  onClick={() => void loadChosen(false)}
                >
                  Load &amp; Place
                </button>
                <button
                  className="btn-outline"
                  disabled={!chosen || busy}
                  onClick={() => void loadChosen(false, false)}
                >
                  Load
                </button>
              </div>
              <p className="muted mb-credit">
                Sizes are nominal rough openings. Double-click a size to load it and{" "}
                {targets.length ? "apply it" : "place it"}.
              </p>
            </aside>
          </div>
        )}
      </div>
    </div>
  );
}
