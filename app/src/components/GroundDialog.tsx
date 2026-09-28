import { useEffect, useState } from "react";
import type { GroundChoice } from "../bindings/GroundChoice";
import { apply } from "../fileActions";
import { errorMessage, ipc } from "../ipc";
import { useAppStore } from "../store";

// Base Ground (ADR-064): what the site is finished in, around and under the model — the
// topography, or the ground in 3D and renders without one. Enscape-style swatches of the
// Site & Landscape materials (lawns grow real 3D grass in Realistic views and renders), or
// any project material.

export const groundSwatch = (id: string) => `/ground/${id}.png`;

export function GroundDialog({ onClose }: { onClose: () => void }) {
  const app = useAppStore((s) => s.app);
  const [list, setList] = useState<GroundChoice[] | null>(null);
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    let live = true;
    ipc.groundLibrary().then(
      (l) => live && setList(l),
      (e) => useAppStore.getState().setError(errorMessage(e)),
    );
    return () => {
      live = false;
    };
  }, []);
  const current = app?.ground ?? null;
  const currentName = app?.materials.find((m) => m.id === current)?.name ?? null;
  const choose = async (preset: string | null, material: string | null = null) => {
    setBusy(true);
    try {
      if (await apply(() => ipc.setBaseGround(material, preset))) onClose();
    } finally {
      setBusy(false);
    }
  };
  return (
    <div className="modal-backdrop" role="dialog" aria-label="Base Ground">
      <div className="modal ground-dialog">
        <div className="al-head">
          <h2>Base Ground</h2>
          <span className="muted">
            {currentName ? `Now: ${currentName}` : "Now: plain lawn"}. The topography, or the ground
            around the model, in 3D and renders.
          </span>
          <button className="btn-ghost" onClick={onClose} aria-label="Close">
            ×
          </button>
        </div>
        <div className="gd-grid" role="list" aria-label="Ground materials">
          {(list ?? []).map((g) => (
            <button
              key={g.id}
              role="listitem"
              className={`gd-card${currentName === g.name ? " active" : ""}`}
              disabled={busy}
              onClick={() => void choose(g.id)}
              title={g.description}
            >
              <img
                src={groundSwatch(g.id)}
                alt=""
                draggable={false}
                style={{ background: `rgb(${g.color.join(",")})` }}
              />
              <span className="al-name">{g.name}</span>
            </button>
          ))}
        </div>
        <div className="mb-actions">
          <label>
            Project material:{" "}
            <select
              aria-label="Project material"
              value={current ?? ""}
              disabled={busy}
              onChange={(e) => void choose(null, e.target.value || null)}
            >
              <option value="">Plain lawn</option>
              {(app?.materials ?? []).map((m) => (
                <option key={m.id} value={m.id}>
                  {m.name}
                </option>
              ))}
            </select>
          </label>
        </div>
      </div>
    </div>
  );
}
