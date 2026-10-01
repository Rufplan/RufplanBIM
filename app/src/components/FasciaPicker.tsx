import { useEffect, useState } from "react";
import type { FasciaSpec } from "../bindings/FasciaSpec";
import { apply } from "../fileActions";
import { errorMessage, ipc } from "../ipc";
import { useAppStore } from "../store";

// Fascia (ADR-095): the trim around a roof's edge, from 20 typical profiles. Applies to
// the selected roofs, or to every roof when none is selected.

const MM_PER_IN = 25.4;
const rgb = (c: [number, number, number]) => `rgb(${c.join(",")})`;

/** The profile in section: the roof's edge (hatched) and the fascia's parts beside it. */
export function FasciaProfile({ f, size = 120 }: { f: FasciaSpec; size?: number }) {
  // A 24" x 24" window: x from -10" (in the roof) to +14", z from -18" to +6".
  const s = size / (24 * MM_PER_IN);
  const X = (mm: number) => (mm + 10 * MM_PER_IN) * s;
  const Z = (mm: number) => (6 * MM_PER_IN - mm) * s;
  const roof = 10 * MM_PER_IN;
  return (
    <svg
      className="fascia-profile"
      width={size}
      height={size}
      viewBox={`0 0 ${size} ${size}`}
      aria-hidden
    >
      <defs>
        <pattern id="fascia-hatch" width="5" height="5" patternUnits="userSpaceOnUse">
          <path d="M0 5 L5 0" stroke="#b9b6ae" strokeWidth="1" />
        </pattern>
      </defs>
      {/* The roof deck, its top at z = 0. */}
      <rect
        x={X(-roof)}
        y={Z(0)}
        width={roof * s}
        height={10 * MM_PER_IN * s}
        fill="url(#fascia-hatch)"
        stroke="#8a877f"
      />
      {f.parts.map((p, i) => (
        <rect
          key={i}
          x={X(p.out0)}
          y={Z(p.z1)}
          width={(p.out1 - p.out0) * s}
          height={(p.z1 - p.z0) * s}
          fill={rgb(f.color)}
          stroke="#1c1c1c"
          strokeWidth={1.2}
        />
      ))}
    </svg>
  );
}

export function FasciaPicker({ onClose }: { onClose: () => void }) {
  const [list, setList] = useState<FasciaSpec[]>([]);
  const [picked, setPicked] = useState<string | null>(null);
  const selection = useAppStore((s) => s.selection);
  const [roofs, setRoofs] = useState<string[]>([]);
  useEffect(() => {
    ipc.fasciaCatalog().then(setList, (e) => useAppStore.getState().setError(errorMessage(e)));
    void ipc.elementCategories(selection).then((cats) => {
      const sel = cats.filter(([, c]) => c === "Roof").map(([id]) => id);
      setRoofs(sel);
    });
  }, [selection]);
  const target = roofs.length
    ? `${roofs.length} selected roof${roofs.length === 1 ? "" : "s"}`
    : "every roof in the project";
  const chosen = list.find((f) => f.name === picked);
  const go = async (name: string | null) => {
    if (await apply(() => ipc.setFascia(roofs.length ? roofs : null, name))) onClose();
  };
  return (
    <div className="modal-backdrop" role="dialog" aria-label="Fascia">
      <div className="modal material-browser fascia-picker">
        <div className="mb-head">
          <h2>Fascia</h2>
          <span className="tp-mode">Applies to {target}</span>
          <button className="btn-ghost" onClick={onClose} aria-label="Close">
            ×
          </button>
        </div>
        <div className="mb-body">
          <div className="mb-grid tp-grid" role="list" aria-label="Fascia profiles">
            {list.map((f) => (
              <button
                key={f.name}
                role="listitem"
                className={`mb-card${picked === f.name ? " active" : ""}`}
                onClick={() => setPicked(f.name)}
                onDoubleClick={() => void go(f.name)}
                title={f.description}
              >
                <FasciaProfile f={f} />
                <span className="mb-name">{f.name}</span>
              </button>
            ))}
          </div>
          <aside className="mb-detail">
            {chosen ? (
              <>
                <FasciaProfile f={chosen} size={220} />
                <h3>{chosen.name}</h3>
                <p className="muted">{chosen.description}</p>
                <div className="mb-actions">
                  <button className="btn-cyan" onClick={() => void go(chosen.name)}>
                    Apply to {roofs.length ? "Selected" : "All Roofs"}
                  </button>
                  <button className="btn-outline" onClick={() => void go(null)}>
                    Remove Fascia
                  </button>
                </div>
              </>
            ) : (
              <p className="muted">
                Pick a profile: 20 typical fascias, from modern bands and knife edges to wood boards
                with gutters and classical cornices.
              </p>
            )}
          </aside>
        </div>
      </div>
    </div>
  );
}
