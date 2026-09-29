import { useEffect, useState } from "react";
import type { Discipline } from "../bindings/Discipline";
import type { MepClimate } from "../bindings/MepClimate";
import type { MepProposal } from "../bindings/MepProposal";
import type { SystemProposal } from "../bindings/SystemProposal";
import { apply } from "../fileActions";
import { errorMessage, ipc } from "../ipc";
import { useAppStore } from "../store";

// MEPT > Suggest (ADR-082): one dialog for each of the four disciplines, like Suggest
// Structure. The model's rooms read into loads and fixtures, the candidate systems ranked
// with their reasons, key numbers and red flags, and Generate overlay for any of them.

export const DISCIPLINES: Discipline[] = ["Mechanical", "Electrical", "Plumbing", "Technology"];

function SystemCard({
  s,
  rank,
  discipline,
}: {
  s: SystemProposal;
  rank: number;
  discipline: Discipline;
}) {
  const [open, setOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const generate = async () => {
    setBusy(true);
    const ok = await apply(() => ipc.mepGenerate(s.settings));
    setBusy(false);
    if (ok) {
      const st = useAppStore.getState();
      st.setMepOverlay(discipline, true);
      st.setUi({ viewDialog: null });
      st.setPrompt(`${discipline} layer generated: ${s.label}. Preliminary — not engineered.`);
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
      <p className="muted">{s.description}</p>
      <p>{s.rationale}</p>
      <dl className="st-facts">
        <dt>Key numbers</dt>
        <dd>
          <ul>
            {s.highlights.map((h) => (
              <li key={h}>{h}</li>
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
        {open ? "Hide scoring" : "Scoring"}
      </button>
      {open && (
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
      )}
      <div className="st-actions">
        <button className="btn-cyan" disabled={busy} onClick={() => void generate()}>
          {busy ? "Generating…" : "Generate overlay"}
        </button>
      </div>
    </section>
  );
}

export function MepDialog({ onClose }: { onClose: () => void }) {
  const discipline = useAppStore((s) => s.mepDiscipline);
  const setDiscipline = useAppStore((s) => s.setMepDiscipline);
  const [climate, setClimate] = useState<MepClimate>("Mixed");
  const [prop, setProp] = useState<MepProposal | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [all, setAll] = useState(false);
  useEffect(() => {
    let live = true;
    ipc.mepSuggest(discipline, climate).then(
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
  }, [discipline, climate]);
  const current = prop?.discipline === discipline ? prop : null;
  const shown = current ? (all ? current.systems : current.systems.slice(0, 3)) : [];
  return (
    <div className="modal-backdrop" role="dialog" aria-label={`Suggest ${discipline}`}>
      <div className="modal structural-dialog">
        <h2>Suggest {discipline}</h2>
        <div className="kn-tabs mep-tabs" role="tablist" aria-label="Disciplines">
          {DISCIPLINES.map((d) => (
            <button
              key={d}
              role="tab"
              aria-selected={d === discipline}
              className={d === discipline ? "on" : ""}
              onClick={() => {
                setDiscipline(d);
                setAll(false);
              }}
            >
              {d}
            </button>
          ))}
        </div>
        {current && (
          <p className="st-disclaimer" role="note">
            {current.disclaimer} It does not check code compliance.
          </p>
        )}
        <div className="st-top">
          {discipline === "Mechanical" && (
            <label className="ob-field">
              Climate
              <select
                aria-label="Climate"
                value={climate}
                onChange={(e) => setClimate(e.target.value as MepClimate)}
              >
                <option value="Hot">Hot</option>
                <option value="Mixed">Mixed</option>
                <option value="Cold">Cold</option>
              </select>
            </label>
          )}
          <button
            className="btn-ghost"
            onClick={() => void ipc.mepEditRules().catch(() => {})}
            title="Open mep_rules.toml: loads, fixtures, systems and sizes"
          >
            Edit Rules…
          </button>
        </div>
        {error && <p className="error">{error}</p>}
        {!current && !error && <p className="muted">Reading the model…</p>}
        {current && (
          <div className="st-body">
            <aside className="st-side">
              <h4>What the model shows</h4>
              <ul>
                {current.summary.map((s) => (
                  <li key={s}>{s}</li>
                ))}
              </ul>
              <h4>Assumptions</h4>
              <ul>
                {current.assumptions.map((s) => (
                  <li key={s}>{s}</li>
                ))}
              </ul>
              {current.questions.length > 0 && (
                <>
                  <h4>Questions for you</h4>
                  <ul className="st-questions">
                    {current.questions.map((s) => (
                      <li key={s}>{s}</li>
                    ))}
                  </ul>
                </>
              )}
            </aside>
            <div className="st-cards">
              {shown.map((s, i) => (
                <SystemCard
                  key={`${discipline}-${s.key}`}
                  s={s}
                  rank={i + 1}
                  discipline={discipline}
                />
              ))}
              <button className="btn-ghost" onClick={() => setAll(!all)}>
                {all ? "Show the top 3" : `Show all ${current.systems.length} systems`}
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
