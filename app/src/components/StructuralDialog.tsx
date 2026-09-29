import { useEffect, useState } from "react";
import type { LateralKind } from "../bindings/LateralKind";
import type { SchemeProposal } from "../bindings/SchemeProposal";
import type { SchemeSettings } from "../bindings/SchemeSettings";
import type { Seismic } from "../bindings/Seismic";
import type { SpanDir } from "../bindings/SpanDir";
import type { StructuralProposal } from "../bindings/StructuralProposal";
import { apply } from "../fileActions";
import { errorMessage, ipc } from "../ipc";
import { useAppStore } from "../store";

// Structure > Suggest Structure (ADR-080): the model's features scored against six
// structural systems, the top three with their reasons, grid, depths and red flags, and a
// Generate overlay action for each. Nothing is generated until you pick one.

const MM_PER_FT = 304.8;

export const DISCLAIMER =
  "Preliminary — not engineered. Requires review by a licensed structural engineer.";

const LATERAL_LABELS: Record<LateralKind, string> = {
  WoodShearWalls: "Wood shear walls",
  CfsShearWalls: "CFS shear walls",
  ConcreteShearWalls: "Concrete shear walls/core",
  BracedFrames: "Braced frames",
  MomentFrames: "Moment frames",
  CltShearWalls: "CLT shear walls",
};

function SchemeCard({ s, rank }: { s: SchemeProposal; rank: number }) {
  const [settings, setSettings] = useState<SchemeSettings>(s.settings);
  const [open, setOpen] = useState(rank === 1);
  const [busy, setBusy] = useState(false);
  const ft = (mm: number) => Math.round(mm / MM_PER_FT);
  const generate = async () => {
    setBusy(true);
    const ok = await apply(() => ipc.structuralGenerate(settings));
    setBusy(false);
    if (ok) {
      const st = useAppStore.getState();
      st.setStructuralOverlay(true);
      st.setUi({ viewDialog: null });
      st.setPrompt(`Structural layer generated: ${s.label}. ${DISCLAIMER}`);
    }
  };
  return (
    <section className={`st-card${s.ruledOut ? " out" : ""}`} aria-label={s.label}>
      <header>
        <span className="st-rank">{rank}</span>
        <h3>{s.label}</h3>
        <span className="st-score" title="Score out of 100">
          <span className="st-bar" style={{ width: `${Math.max(4, s.score)}%` }} />
          {Math.round(s.score)}
        </span>
        {s.ruledOut && <span className="st-badge">Beyond its limits</span>}
      </header>
      <p>{s.rationale}</p>
      <dl className="st-facts">
        <dt>Typical grid</dt>
        <dd>{s.grid}</dd>
        <dt>Member depths</dt>
        <dd>
          <ul>
            {s.memberDepths.map((d) => (
              <li key={d}>{d}</li>
            ))}
          </ul>
        </dd>
        {s.redFlags.length > 0 && (
          <>
            <dt>Red flags</dt>
            <dd>
              <ul className="st-flags">
                {s.redFlags.map((f) => (
                  <li key={f}>{f}</li>
                ))}
              </ul>
            </dd>
          </>
        )}
      </dl>
      <button className="btn-ghost st-more" onClick={() => setOpen(!open)} aria-expanded={open}>
        {open ? "Hide scoring and settings" : "Scoring and settings"}
      </button>
      {open && (
        <div className="st-open">
          <table className="st-criteria">
            <tbody>
              {s.criteria.map((c) => (
                <tr key={c.name}>
                  <th>{c.name}</th>
                  <td className="st-c">{Math.round(c.score * 100)}</td>
                  <td>{c.note}</td>
                </tr>
              ))}
            </tbody>
          </table>
          <div className="st-settings" role="group" aria-label={`${s.label} settings`}>
            <label>
              Grid E–W (ft)
              <input
                type="number"
                min={8}
                max={60}
                aria-label="Grid east–west"
                value={ft(settings.gridX)}
                onChange={(e) =>
                  setSettings({ ...settings, gridX: Number(e.target.value) * MM_PER_FT })
                }
              />
            </label>
            <label>
              Grid N–S (ft)
              <input
                type="number"
                min={8}
                max={60}
                aria-label="Grid north–south"
                value={ft(settings.gridY)}
                onChange={(e) =>
                  setSettings({ ...settings, gridY: Number(e.target.value) * MM_PER_FT })
                }
              />
            </label>
            <label>
              Lateral system
              <select
                aria-label="Lateral system"
                value={settings.lateral}
                onChange={(e) =>
                  setSettings({ ...settings, lateral: e.target.value as LateralKind })
                }
              >
                {s.laterals.map((l) => (
                  <option key={l} value={l}>
                    {LATERAL_LABELS[l]}
                  </option>
                ))}
              </select>
            </label>
            <label>
              Joists/deck span
              <select
                aria-label="Joist direction"
                value={settings.spanDir}
                onChange={(e) => setSettings({ ...settings, spanDir: e.target.value as SpanDir })}
              >
                <option value="Auto">Shorter way (auto)</option>
                <option value="X">East–west</option>
                <option value="Y">North–south</option>
              </select>
            </label>
          </div>
        </div>
      )}
      <div className="st-actions">
        <button className="btn-cyan" disabled={busy} onClick={() => void generate()}>
          {busy ? "Generating…" : "Generate overlay"}
        </button>
      </div>
    </section>
  );
}

export function StructuralDialog({ onClose }: { onClose: () => void }) {
  const [seismic, setSeismic] = useState<Seismic>("Moderate");
  const [prop, setProp] = useState<StructuralProposal | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [all, setAll] = useState(false);
  useEffect(() => {
    let live = true;
    ipc.structuralSuggest(seismic).then(
      (p) => {
        if (!live) return;
        setProp(p);
        setError(null);
      },
      (e) => live && setError(errorMessage(e)),
    );
    return () => {
      live = false;
    };
  }, [seismic]);
  const shown = prop ? (all ? prop.schemes : prop.schemes.slice(0, 3)) : [];
  return (
    <div className="modal-backdrop" role="dialog" aria-label="Suggest Structure">
      <div className="modal structural-dialog">
        <h2>Suggest Structure</h2>
        <p className="st-disclaimer" role="note">
          {DISCLAIMER} It does not check code compliance.
        </p>
        <div className="st-top">
          <label className="ob-field">
            Seismic region
            <select
              aria-label="Seismic region"
              value={seismic}
              onChange={(e) => setSeismic(e.target.value as Seismic)}
            >
              <option value="Low">Low</option>
              <option value="Moderate">Moderate</option>
              <option value="High">High</option>
            </select>
          </label>
          <button
            className="btn-ghost"
            onClick={() => void ipc.structuralEditRules().catch(() => {})}
            title="Open structural_rules.toml: thresholds, weights and sizing tables"
          >
            Edit Rules…
          </button>
        </div>
        {error && <p className="error">{error}</p>}
        {!prop && !error && <p className="muted">Reading the model…</p>}
        {prop && (
          <div className="st-body">
            <aside className="st-side">
              <h4>What the model shows</h4>
              <ul>
                {prop.summary.map((s) => (
                  <li key={s}>{s}</li>
                ))}
              </ul>
              <h4>Assumptions</h4>
              <ul>
                {prop.assumptions.map((s) => (
                  <li key={s}>{s}</li>
                ))}
              </ul>
              {prop.questions.length > 0 && (
                <>
                  <h4>Questions for you</h4>
                  <ul className="st-questions">
                    {prop.questions.map((s) => (
                      <li key={s}>{s}</li>
                    ))}
                  </ul>
                </>
              )}
            </aside>
            <div className="st-cards">
              {shown.map((s, i) => (
                <SchemeCard key={`${s.kind}-${prop.seismic}`} s={s} rank={i + 1} />
              ))}
              <button className="btn-ghost" onClick={() => setAll(!all)}>
                {all ? "Show the top 3" : `Show all ${prop.schemes.length} schemes`}
              </button>
            </div>
          </div>
        )}
        <div className="modal-actions">
          <button className="btn-outline" onClick={onClose}>
            Close
          </button>
        </div>
      </div>
    </div>
  );
}
