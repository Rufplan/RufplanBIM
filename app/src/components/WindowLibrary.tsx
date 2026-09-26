import { useEffect, useMemo, useState } from "react";
import type { FrameFinish } from "../bindings/FrameFinish";
import type { Grille } from "../bindings/Grille";
import type { WindowFamily } from "../bindings/WindowFamily";
import type { WindowLibrary as Library } from "../bindings/WindowLibrary";
import type { WindowPreview } from "../bindings/WindowPreview";
import type { WindowSpec } from "../bindings/WindowSpec";
import { errorMessage, ipc } from "../ipc";
import { useAppStore } from "../store";

// The Window Library (ADR-031): the common US window families at their standard sizes,
// or a custom size, loaded into the project as types — Revit's Load Family.

let library: Promise<Library> | null = null;
function loadLibrary(): Promise<Library> {
  if (!library) {
    library = ipc.windowLibrary();
    library.catch(() => (library = null));
  }
  return library;
}

const MM_PER_IN = 25.4;
const hex = (c: [number, number, number]) =>
  `#${c.map((v) => v.toString(16).padStart(2, "0")).join("")}`;

/** A window's elevation, drawn from the lines Rust computes. */
export function WindowThumb({
  preview,
  frame = "#f0f0ec",
  label,
}: {
  preview: WindowPreview;
  frame?: string;
  label: string;
}) {
  const { width: w, height: h } = preview;
  const pad = Math.max(w, h) * 0.08;
  const toPath = (pts: [number, number][], closed: boolean) =>
    `M${pts.map(([x, y]) => `${x} ${h - y}`).join("L")}${closed ? "Z" : ""}`;
  return (
    <svg
      className="wl-thumb"
      viewBox={`${-pad} ${-pad} ${w + 2 * pad} ${h + 2 * pad}`}
      role="img"
      aria-label={label}
    >
      <rect x={0} y={0} width={w} height={h} fill={frame} />
      {preview.lines.map((l, i) => (
        <path
          key={i}
          d={toPath(l.pts, l.closed)}
          fill={l.glass ? "#bfe3ee" : "none"}
          stroke="#1c1c1c"
          strokeWidth={l.w > 1 ? 1.4 : 0.8}
          strokeDasharray={l.dashed ? "4 3" : undefined}
          vectorEffect="non-scaling-stroke"
        />
      ))}
    </svg>
  );
}

const key = (s: WindowSpec) => `${s.family}|${s.units}|${s.width}|${s.height}`;

export function WindowLibrary({ onClose }: { onClose: () => void }) {
  const app = useAppStore((s) => s.app);
  const [lib, setLib] = useState<Library | null>(null);
  const [family, setFamily] = useState<WindowFamily>("DoubleHung");
  const [picked, setPicked] = useState<string | null>(null);
  const [checked, setChecked] = useState<Set<string>>(new Set());
  const [grille, setGrille] = useState<Grille>("None");
  const [finish, setFinish] = useState<FrameFinish>("White");
  const [custom, setCustom] = useState({ width: "", height: "", sill: "", units: 1 });
  const [fetched, setFetched] = useState<{
    key: string;
    name: string;
    preview: WindowPreview;
  } | null>(null);
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

  const info = lib?.families.find((f) => f.family === family);
  const sizes = useMemo(
    () => lib?.presets.filter((p) => p.spec.family === family) ?? [],
    [lib, family],
  );
  const inProject = new Set(app?.windowTypes.map((t) => t.name) ?? []);
  const withOptions = (s: WindowSpec): WindowSpec => ({
    ...s,
    grille: info?.grilles ? grille : "None",
    finish,
  });

  // The custom size, when all three are filled in (inches).
  const customSpec: WindowSpec | null = (() => {
    const [w, h, sill] = [custom.width, custom.height, custom.sill].map(Number);
    if (!custom.width || !custom.height || !(w! > 0) || !(h! > 0)) return null;
    return withOptions({
      family,
      units: info?.mullable ? custom.units : 1,
      width: w! * MM_PER_IN,
      height: h! * MM_PER_IN,
      sill: (custom.sill && Number.isFinite(sill) ? sill! : Math.max(84 - h!, 12)) * MM_PER_IN,
      grille: "None",
      finish: "White",
    });
  })();
  const pickedPreset = sizes.find((p) => key(p.spec) === picked);
  const shown = pickedPreset ? withOptions(pickedPreset.spec) : customSpec;

  // The big preview follows the picked size and the options (named as it would load).
  const shownKey = shown ? JSON.stringify(shown) : "";
  useEffect(() => {
    if (!shown) return;
    let live = true;
    ipc.windowPreview(shown).then(
      (p) => live && setFetched({ key: shownKey, ...p }),
      () => {},
    );
    return () => {
      live = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [shownKey]);

  const preview = shown && fetched?.key === shownKey ? fetched : null;

  const toggle = (k: string) => {
    const next = new Set(checked);
    if (next.has(k)) next.delete(k);
    else next.add(k);
    setChecked(next);
  };

  const load = async (specs: WindowSpec[], place: boolean) => {
    if (specs.length === 0) return;
    setBusy(true);
    try {
      const r = await ipc.loadWindowTypes(specs);
      const s = useAppStore.getState();
      s.setApp(r.state);
      if (place && r.ids[0]) {
        s.setToolType("window", r.ids[0]);
        s.setTool("window");
        onClose();
      } else {
        setChecked(new Set());
      }
    } catch (e) {
      useAppStore.getState().setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  };
  const checkedSpecs = (lib?.presets ?? [])
    .filter((p) => checked.has(key(p.spec)))
    .map((p) => withOptions(p.spec));

  const familyThumb = (f: WindowFamily) =>
    lib?.presets.find((p) => p.spec.family === f && p.spec.units === 1)?.preview;
  const frameColor = hex(lib?.finishes.find((f) => f.id === finish)?.color ?? [240, 240, 236]);

  return (
    <div className="modal-backdrop" role="dialog" aria-label="Window Library">
      <div className="modal material-browser window-library">
        <div className="mb-head">
          <h2>Window Library</h2>
          <span className="muted">
            Common US window families at standard sizes. Load them as types, then place with Window
            (WN).
          </span>
          <button className="btn-ghost" onClick={onClose} aria-label="Close">
            ×
          </button>
        </div>
        <div className="mb-body">
          <nav className="mb-filters wl-families" aria-label="Families">
            <h3>Family</h3>
            {lib?.families.map((f) => {
              const t = familyThumb(f.family);
              return (
                <button
                  key={f.family}
                  className={`mb-cat wl-family${family === f.family ? " active" : ""}`}
                  onClick={() => {
                    setFamily(f.family);
                    setPicked(null);
                  }}
                  title={f.description}
                >
                  {t && <WindowThumb preview={t} label="" />}
                  <span>{f.label}</span>
                </button>
              );
            })}
          </nav>
          <div className="mb-grid wl-grid" role="list" aria-label={`${info?.label ?? ""} sizes`}>
            {sizes.map((p) => {
              const k = key(p.spec);
              const loaded = inProject.has(p.name);
              return (
                <div
                  key={k}
                  role="listitem"
                  className={`mb-card wl-card${picked === k ? " active" : ""}`}
                  onClick={() => setPicked(k)}
                  onDoubleClick={() => void load([withOptions(p.spec)], false)}
                >
                  <WindowThumb preview={p.preview} frame={frameColor} label={p.name} />
                  <label className="wl-check" onClick={(e) => e.stopPropagation()}>
                    <input
                      type="checkbox"
                      checked={checked.has(k)}
                      onChange={() => toggle(k)}
                      aria-label={`Select ${p.name}`}
                    />
                    <span className="mb-name">{p.name}</span>
                  </label>
                  {loaded && <span className="mb-tier wl-loaded">In project</span>}
                </div>
              );
            })}
          </div>
          <aside className="mb-detail">
            {info && (
              <>
                {preview && (
                  <WindowThumb preview={preview.preview} frame={frameColor} label={preview.name} />
                )}
                <h3>{preview?.name ?? info.label}</h3>
                <p>{info.description}</p>
                {shown && (
                  <p className="mb-meta">
                    Default sill {Math.round(shown.sill / MM_PER_IN)}" · head{" "}
                    {Math.round((shown.sill + shown.height) / MM_PER_IN)}"
                    {inProject.has(preview?.name ?? "") ? " · already in the project" : ""}
                  </p>
                )}
                <div className="wl-options">
                  <label>
                    Grille
                    <select
                      value={info.grilles ? grille : "None"}
                      disabled={!info.grilles}
                      onChange={(e) => setGrille(e.target.value as Grille)}
                    >
                      {lib?.grilles.map((g) => (
                        <option key={g.id} value={g.id}>
                          {g.label}
                        </option>
                      ))}
                    </select>
                  </label>
                  <div className="wl-finishes" role="radiogroup" aria-label="Frame finish">
                    {lib?.finishes.map((f) => (
                      <button
                        key={f.id}
                        role="radio"
                        aria-checked={finish === f.id}
                        className={`wl-swatch${finish === f.id ? " active" : ""}`}
                        style={{ background: hex(f.color) }}
                        title={f.label}
                        aria-label={f.label}
                        onClick={() => setFinish(f.id)}
                      />
                    ))}
                  </div>
                </div>
                <h3>Custom size</h3>
                <div className="wl-custom">
                  {(["width", "height", "sill"] as const).map((k) => (
                    <label key={k}>
                      {k === "sill" ? "Sill" : k[0]!.toUpperCase() + k.slice(1)} (in)
                      <input
                        type="number"
                        min={0}
                        value={custom[k]}
                        placeholder={k === "sill" ? "head 84" : ""}
                        onChange={(e) => {
                          setCustom({ ...custom, [k]: e.target.value });
                          setPicked(null);
                        }}
                      />
                    </label>
                  ))}
                  {info.mullable && (
                    <label>
                      Units
                      <select
                        value={custom.units}
                        onChange={(e) => {
                          setCustom({ ...custom, units: Number(e.target.value) });
                          setPicked(null);
                        }}
                      >
                        {[1, 2, 3, 4].map((n) => (
                          <option key={n} value={n}>
                            {n}
                          </option>
                        ))}
                      </select>
                    </label>
                  )}
                </div>
                <div className="mb-actions">
                  <button
                    className="btn-cyan"
                    disabled={!shown || busy}
                    onClick={() => shown && void load([shown], true)}
                  >
                    Load &amp; Place
                  </button>
                  <button
                    className="btn-outline"
                    disabled={!shown || busy}
                    onClick={() => shown && void load([shown], false)}
                  >
                    Load
                  </button>
                  <button
                    className="btn-outline"
                    disabled={checkedSpecs.length === 0 || busy}
                    onClick={() => void load(checkedSpecs, false)}
                  >
                    Load Checked ({checkedSpecs.length})
                  </button>
                </div>
                <p className="muted mb-credit">
                  Sizes are nominal rough openings, heads at 7&apos;-0&quot; to line up with doors.
                  Double-click a size to load it.
                </p>
              </>
            )}
          </aside>
        </div>
      </div>
    </div>
  );
}
