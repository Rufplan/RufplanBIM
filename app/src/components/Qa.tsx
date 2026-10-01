import { useMemo, useState } from "react";
import { create } from "zustand";
import type { QaCategory } from "../bindings/QaCategory";
import type { QaFinding } from "../bindings/QaFinding";
import type { QaFix } from "../bindings/QaFix";
import type { QaFixPlan } from "../bindings/QaFixPlan";
import type { QaMilestone } from "../bindings/QaMilestone";
import type { QaReport } from "../bindings/QaReport";
import type { QaSeverity } from "../bindings/QaSeverity";
import type { QaSuggestion } from "../bindings/QaSuggestion";
import { dialogs, errorMessage, ipc } from "../ipc";
import { useAppStore } from "../store";
import { Icons } from "./Icons";

// The QA/QC tab (ADR-088): a milestone review of the set (studio-qa's rules, in Rust) and,
// on request, Claude's overall review. The results open in a floating list: filter by
// category and severity, Show takes you to the element, Resolve ticks it off, and the
// report exports to PDF.

export const MILESTONES: [QaMilestone, string][] = [
  ["SchematicDesign", "Schematic Design"],
  ["DesignDevelopment", "Design Development"],
  ["Cd50", "50% CDs"],
  ["Cd90", "90% CDs"],
  ["Cd100", "100% CDs"],
  ["Permit", "Permit Submittal"],
  ["Bid", "Bid Set"],
];

export const CATEGORIES: [QaCategory, string, keyof typeof Icons][] = [
  ["Coordination", "Coordination", "qaCoord"],
  ["Completeness", "Completeness", "qaReport"],
  ["Code", "Code", "qaCode"],
  ["Accessibility", "Accessibility", "qaCode"],
  ["Waterproofing", "Waterproofing", "qaWater"],
  ["DrawingSpec", "Drawings ↔ Specs", "qaReport"],
  ["Constructability", "Constructability", "qaCoord"],
  ["Consultants", "Consultants", "qaCoord"],
];

const SEVERITIES: QaSeverity[] = ["Critical", "Major", "Minor", "Info"];

interface QaUi {
  milestone: QaMilestone;
  categories: QaCategory[];
  report: QaReport | null;
  running: boolean;
  claude: boolean;
  open: boolean;
  resolved: string[];
  category: QaCategory | null;
  severity: QaSeverity | null;
  query: string;
  hideResolved: boolean;
  /** Fix Issues (ADR-089): apply everything, or approve each change. */
  fixMode: "auto" | "approve";
  plan: QaFixPlan | null;
  fixOpen: boolean;
  fixing: boolean;
  /** The last run: how many applied, and the score before and after. */
  fixResult: { applied: number; errors: string[]; before: number; after: number } | null;
}

export const useQa = create<QaUi>(() => ({
  milestone: "Cd90",
  categories: CATEGORIES.map((c) => c[0]),
  report: null,
  running: false,
  claude: false,
  open: false,
  resolved: [],
  category: null,
  severity: null,
  query: "",
  hideResolved: false,
  fixMode: "approve",
  plan: null,
  fixOpen: false,
  fixing: false,
  fixResult: null,
}));

export async function runReview() {
  const s = useQa.getState();
  useQa.setState({ running: true });
  try {
    const report = await ipc.qaReview({ milestone: s.milestone, categories: s.categories });
    useQa.setState({ report, open: true, category: null, severity: null });
    // Which findings can be fixed.
    const plan = await ipc.qaFixPlan(report).catch(() => null);
    useQa.setState({ plan });
  } catch (e) {
    useAppStore.getState().setError(errorMessage(e));
  } finally {
    useQa.setState({ running: false });
  }
}

async function claudeReview() {
  let r = useQa.getState().report;
  if (!r) {
    await runReview();
    r = useQa.getState().report;
  }
  if (!r) return;
  useQa.setState({ claude: true, open: true });
  try {
    const merged = await ipc.qaClaude(r);
    useQa.setState({ report: merged });
  } catch (e) {
    useAppStore.getState().setError(errorMessage(e));
  } finally {
    useQa.setState({ claude: false });
  }
}

async function exportPdf() {
  const { report, resolved } = useQa.getState();
  if (!report) return;
  const project = useAppStore.getState().app?.projectName || "Project";
  const path = await dialogs.pickPdfLocation(`${project} - QA-QC Review`);
  if (!path) return;
  try {
    await ipc.qaExportPdf(path, report, resolved);
  } catch (e) {
    useAppStore.getState().setError(errorMessage(e));
  }
}

/** Takes you to what a finding is about: its view, with its elements selected. */
function show(f: QaFinding) {
  const s = useAppStore.getState();
  if (f.view && s.app?.views.some((v) => v.id === f.view)) s.openView(f.view);
  s.select(f.elements);
}

function Stroke({ d }: { d: string }) {
  return (
    <svg
      width="18"
      height="18"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.7"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden
    >
      <path d={d} />
    </svg>
  );
}

export function QaRibbon() {
  const { milestone, categories, running, claude, report, fixing, fixMode } = useQa();
  const toggle = (c: QaCategory) =>
    useQa.setState({
      categories: categories.includes(c) ? categories.filter((x) => x !== c) : [...categories, c],
    });
  return (
    <>
      <div className="rb-group">
        <div className="rb-items">
          <button
            className="rb-btn qa-run"
            disabled={running || categories.length === 0}
            title="Review the set for the milestone"
            onClick={() => void runReview()}
          >
            {Icons.qaReview}
            <span>{running ? "Reviewing…" : "Run Review"}</span>
          </button>
          <div className="std-library qa-milestone">
            <label>
              <span>Milestone</span>
              <select
                aria-label="Milestone"
                value={milestone}
                onChange={(e) => useQa.setState({ milestone: e.target.value as QaMilestone })}
              >
                {MILESTONES.map(([m, l]) => (
                  <option key={m} value={m}>
                    {l}
                  </option>
                ))}
              </select>
            </label>
          </div>
        </div>
        <div className="rb-title">QUALITY CONTROL</div>
      </div>
      <div className="rb-group">
        <div className="rb-items qa-checks" role="group" aria-label="Checks">
          {CATEGORIES.map(([c, label]) => (
            <button
              key={c}
              role="checkbox"
              aria-checked={categories.includes(c)}
              className={`qa-check${categories.includes(c) ? " on" : ""}`}
              onClick={() => toggle(c)}
            >
              <span className="qa-box" aria-hidden>
                {categories.includes(c) ? "✓" : ""}
              </span>
              {label}
            </button>
          ))}
        </div>
        <div className="rb-title">CHECKS</div>
      </div>
      <div className="rb-group">
        <div className="rb-items">
          <button
            className="rb-btn rb-claude"
            disabled={claude}
            title="Claude's overall review: readiness, risks and what the rules can't see"
            onClick={() => void claudeReview()}
          >
            {Icons.sparkle}
            <span>{claude ? "Reviewing…" : "Overall Review"}</span>
          </button>
        </div>
        <div className="rb-title">CLAUDE</div>
      </div>
      <div className="rb-group">
        <div className="rb-items">
          <button
            className="rb-btn"
            disabled={!report}
            title="Show the findings"
            onClick={() => useQa.setState({ open: true })}
          >
            <Stroke d="M4 5h16M4 10h16M4 15h10M4 20h7" />
            <span>Findings{report ? ` (${report.findings.length})` : ""}</span>
          </button>
          <button
            className="rb-btn"
            disabled={!report}
            title="Export the review as a PDF report"
            onClick={() => void exportPdf()}
          >
            {Icons.qaReport}
            <span>Export PDF</span>
          </button>
        </div>
        <div className="rb-title">RESULTS</div>
      </div>
      <div className="rb-group">
        <div className="rb-items">
          <button
            className="rb-btn qa-fix-btn"
            disabled={!report || fixing}
            title="Fix Issues: repair what the model can fix on its own"
            onClick={() => void openFix()}
          >
            <Stroke d="M14.5 6.5a4 4 0 0 0-5.3 5.3L4 17l3 3 5.2-5.2a4 4 0 0 0 5.3-5.3l-2.4 2.4-2.6-.6-.6-2.6z" />
            <span>{fixing ? "Fixing…" : "Fix Issues"}</span>
          </button>
          <div className="qa-mode" role="radiogroup" aria-label="Fix mode">
            {(
              [
                ["auto", "Auto", "Apply every fix at once (one undo step)"],
                ["approve", "Approve each", "Approve every change, one at a time"],
              ] as const
            ).map(([m, l, t]) => (
              <button
                key={m}
                role="radio"
                aria-checked={fixMode === m}
                title={t}
                className={fixMode === m ? "on" : undefined}
                onClick={() => useQa.setState({ fixMode: m })}
              >
                <span className="qa-dot" aria-hidden />
                {l}
              </button>
            ))}
          </div>
        </div>
        <div className="rb-title">FIX</div>
      </div>
    </>
  );
}

function ScoreRing({ score }: { score: number }) {
  const r = 26;
  const c = 2 * Math.PI * r;
  const color = score >= 85 ? "#2e7d32" : score >= 60 ? "#e6a23c" : "#c0352b";
  return (
    <svg width="64" height="64" viewBox="0 0 64 64" aria-label={`Score ${score} of 100`}>
      <circle cx="32" cy="32" r={r} fill="none" stroke="#e8e8e8" strokeWidth="6" />
      <circle
        cx="32"
        cy="32"
        r={r}
        fill="none"
        stroke={color}
        strokeWidth="6"
        strokeDasharray={`${(score / 100) * c} ${c}`}
        transform="rotate(-90 32 32)"
        strokeLinecap="round"
      />
      <text
        x="32"
        y="37"
        textAnchor="middle"
        fontSize="17"
        fontWeight="800"
        fontFamily="Barlow Condensed, sans-serif"
        fill="#111"
      >
        {score}
      </text>
    </svg>
  );
}

/** The pop-up list of everything the review found. */
export function QaPanel() {
  const { report, open, resolved, category, severity, query, hideResolved, claude, plan } = useQa();
  const [expanded, setExpanded] = useState<string | null>(null);
  const list = useMemo(() => {
    if (!report) return [];
    const q = query.trim().toLowerCase();
    return report.findings.filter(
      (f) =>
        (!category || f.category === category) &&
        (!severity || f.severity === severity) &&
        (!hideResolved || !resolved.includes(f.id)) &&
        (!q || `${f.title} ${f.detail} ${f.reference}`.toLowerCase().includes(q)),
    );
  }, [report, category, severity, query, hideResolved, resolved]);
  if (!open || !report) return null;
  const total = (c: QaCategory) => report.findings.filter((f) => f.category === c).length;
  const bySev = (s: QaSeverity) => report.findings.filter((f) => f.severity === s).length;
  const toggleResolved = (id: string) =>
    useQa.setState({
      resolved: resolved.includes(id) ? resolved.filter((x) => x !== id) : [...resolved, id],
    });
  const done = report.findings.filter((f) => resolved.includes(f.id)).length;
  return (
    <aside className="qa-panel" role="dialog" aria-label="QA/QC findings">
      <div className="qa-head">
        <div className="qa-head-title">
          {Icons.qaReview}
          <span>QA/QC REVIEW</span>
          <em>{report.milestoneLabel}</em>
        </div>
        <button
          className="em-close"
          aria-label="Close"
          onClick={() => useQa.setState({ open: false })}
        >
          ×
        </button>
      </div>
      <div className="qa-summary">
        <ScoreRing score={report.score} />
        <div className="qa-summary-text">
          <strong>{report.project}</strong>
          <span>{report.summary}</span>
          <em>{report.codeBasis}</em>
        </div>
      </div>
      <div className="qa-sevs" role="radiogroup" aria-label="Severity">
        <button
          role="radio"
          aria-checked={!severity}
          className={!severity ? "on" : undefined}
          onClick={() => useQa.setState({ severity: null })}
        >
          All <b>{report.findings.length}</b>
        </button>
        {SEVERITIES.map((s) => (
          <button
            key={s}
            role="radio"
            aria-checked={severity === s}
            className={`${s.toLowerCase()}${severity === s ? " on" : ""}`}
            onClick={() => useQa.setState({ severity: severity === s ? null : s })}
          >
            {s} <b>{bySev(s)}</b>
          </button>
        ))}
      </div>
      <div className="qa-cats">
        <button
          className={!category ? "on" : undefined}
          onClick={() => useQa.setState({ category: null })}
        >
          Everything
        </button>
        {CATEGORIES.filter(([c]) => total(c) > 0).map(([c, label]) => (
          <button
            key={c}
            className={category === c ? "on" : undefined}
            onClick={() => useQa.setState({ category: category === c ? null : c })}
          >
            {label} <b>{total(c)}</b>
          </button>
        ))}
      </div>
      <div className="qa-tools">
        <input
          aria-label="Search findings"
          placeholder="Search findings…"
          value={query}
          onChange={(e) => useQa.setState({ query: e.target.value })}
        />
        <label className="sp-check">
          <input
            type="checkbox"
            checked={hideResolved}
            onChange={(e) => useQa.setState({ hideResolved: e.target.checked })}
          />
          Hide resolved ({done})
        </label>
      </div>
      <div className="qa-list" role="list">
        {(report.overview || claude) && (
          <div className="qa-overview">
            <span className="em-eyebrow">OVERALL REVIEW · CLAUDE</span>
            {claude ? (
              <p className="qa-muted">Claude is reviewing the set…</p>
            ) : (
              report.overview
                ?.split("\n")
                .filter(Boolean)
                .map((p, i) => <p key={i}>{p}</p>)
            )}
          </div>
        )}
        {list.length === 0 && (
          <div className="qa-empty">
            {report.findings.length
              ? "Nothing matches the filters."
              : "No issues found by the automated checks."}
          </div>
        )}
        {list.map((f) => {
          const isOpen = expanded === f.id;
          const res = resolved.includes(f.id);
          return (
            <div
              key={f.id}
              role="listitem"
              className={`qa-item ${f.severity.toLowerCase()}${res ? " resolved" : ""}`}
            >
              <button
                className="qa-item-head"
                aria-expanded={isOpen}
                onClick={() => setExpanded(isOpen ? null : f.id)}
              >
                <span className={`qa-sev ${f.severity.toLowerCase()}`}>
                  {f.severity.toUpperCase()}
                </span>
                <span className="qa-title">{f.title}</span>
                {f.source === "claude" && <span className="sp-badge claude">AI</span>}
              </button>
              <div className="qa-meta">
                <span>{CATEGORIES.find((c) => c[0] === f.category)?.[1]}</span>
                {f.reference && <span className="qa-ref">{f.reference}</span>}
              </div>
              {isOpen && (
                <div className="qa-body">
                  <p>{f.detail}</p>
                  <p className="qa-fix">
                    <b>Fix:</b> {f.fix}
                  </p>
                </div>
              )}
              <div className="qa-actions">
                {plan?.fixes.some((x) => x.finding === f.id) && (
                  <button
                    className="qa-fix-one"
                    title={plan.fixes
                      .filter((x) => x.finding === f.id)
                      .map((x) => x.change)
                      .join("; ")}
                    onClick={() => void applyFixes(plan.fixes.filter((x) => x.finding === f.id))}
                  >
                    Fix
                  </button>
                )}
                <button disabled={!f.elements.length && !f.view} onClick={() => show(f)}>
                  Show
                </button>
                <button onClick={() => toggleResolved(f.id)}>{res ? "Reopen" : "Resolve"}</button>
              </div>
            </div>
          );
        })}
      </div>
      <div className="qa-foot">
        <span>{report.disclaimer}</span>
        <button className="pi-btn" onClick={() => void exportPdf()}>
          Export PDF
        </button>
      </div>
    </aside>
  );
}

// ---- Fix Issues (ADR-089) ----

async function openFix() {
  const { report } = useQa.getState();
  if (!report) return;
  useQa.setState({ fixing: true, fixResult: null });
  try {
    const plan = await ipc.qaFixPlan(report);
    useQa.setState({ plan, fixOpen: true });
  } catch (e) {
    useAppStore.getState().setError(errorMessage(e));
  } finally {
    useQa.setState({ fixing: false });
  }
}

/** Applies fixes as one undo step, then reviews again. */
export async function applyFixes(fixes: QaFix[], label?: string) {
  if (fixes.length === 0) return 0;
  const before = useQa.getState().report?.score ?? 0;
  useQa.setState({ fixing: true });
  try {
    const name =
      label ??
      (fixes.length === 1 ? `QA/QC Fix: ${fixes[0]!.title}` : `QA/QC: Fix ${fixes.length} issues`);
    const [applied, errors, state] = await ipc.qaFixApply(
      fixes.map((f) => f.action),
      name,
    );
    if (state) useAppStore.getState().setApp(state);
    await runReview();
    const after = useQa.getState().report?.score ?? before;
    useQa.setState({ fixResult: { applied, errors, before, after } });
    return applied;
  } catch (e) {
    useAppStore.getState().setError(errorMessage(e));
    return 0;
  } finally {
    useQa.setState({ fixing: false });
  }
}

function FixRow({ f, checked, onToggle }: { f: QaFix; checked?: boolean; onToggle?: () => void }) {
  return (
    <label className={`qa-fix-row${f.designChange ? " design" : ""}`}>
      {onToggle && <input type="checkbox" checked={checked} onChange={onToggle} />}
      <span className="qa-fix-text">
        <b>{f.title}</b>
        <em>{f.change}</em>
      </span>
      {f.designChange && <span className="qa-design">DESIGN CHANGE</span>}
    </label>
  );
}

/** Fix Issues: every fix at once (Auto), or each change approved in turn. */
export function QaFixDialog() {
  const { plan, fixOpen, fixMode, fixing, fixResult, report } = useQa();
  const [off, setOff] = useState<number[]>([]);
  const [step, setStep] = useState(0);
  const [done, setDone] = useState<{ applied: number; skipped: number }>({
    applied: 0,
    skipped: 0,
  });
  // Fix with Claude (ADR-094): suggestions for the findings that need you.
  const [asking, setAsking] = useState(false);
  const [suggestions, setSuggestions] = useState<QaSuggestion[] | null>(null);
  const [claudeOff, setClaudeOff] = useState<string[]>([]);
  if (!fixOpen || !plan) return null;
  const close = () => {
    useQa.setState({ fixOpen: false });
    setStep(0);
    setOff([]);
    setDone({ applied: 0, skipped: 0 });
    setSuggestions(null);
    setClaudeOff([]);
  };
  const ruleOf = (id: string) => report?.findings.find((x) => x.id === id)?.rule ?? "";
  // Only the architect knows the project's people and addresses: those open Project Info.
  const askable = plan.manual.filter(([id]) => ruleOf(id) !== "project-info");
  const askClaude = async () => {
    if (!report || askable.length === 0) return;
    setAsking(true);
    try {
      const s = await ipc.qaFixClaude(
        report,
        askable.map(([id]) => id),
      );
      setSuggestions(s);
      setClaudeOff([]);
    } catch (e) {
      useAppStore.getState().setError(errorMessage(e));
    } finally {
      setAsking(false);
    }
  };
  const claudeFixes = (suggestions ?? []).flatMap((s, i) =>
    s.fixes.map((f, j) => ({ key: `${i}:${j}`, f })),
  );
  const chosenClaude = claudeFixes.filter((x) => !claudeOff.includes(x.key)).map((x) => x.f);
  const applyClaude = async () => {
    const n = await applyFixes(chosenClaude, `QA/QC: Claude's fixes (${chosenClaude.length})`);
    if (n > 0) {
      setSuggestions(null);
      const r = useQa.getState().report;
      if (r) useQa.setState({ plan: await ipc.qaFixPlan(r) });
    }
  };
  const openProjectInfo = () => {
    close();
    useAppStore.getState().setRibbonTab("Project Info");
  };
  const fixes = plan.fixes;
  const titleOf = (id: string) => report?.findings.find((x) => x.id === id)?.title ?? id;
  const design = fixes.filter((f) => f.designChange).length;
  const current = fixes[step];
  const finished = fixMode === "approve" ? step >= fixes.length : !!fixResult;
  const again = async () => {
    const r = useQa.getState().report;
    if (!r) return;
    const next = await ipc.qaFixPlan(r);
    useQa.setState({ plan: next, fixResult: null });
    setStep(0);
    setOff([]);
    setDone({ applied: 0, skipped: 0 });
  };
  return (
    <div
      className="edit-model-backdrop"
      role="presentation"
      onMouseDown={(e) => e.target === e.currentTarget && close()}
    >
      <div className="edit-model qa-fixer" role="dialog" aria-modal aria-label="Fix Issues">
        <div className="em-head">
          <div className="em-title">
            <span className="em-name">FIX ISSUES</span>
            <span className="em-sub">
              {fixMode === "auto" ? "Auto: every fix at once" : "Approve each change"}
            </span>
          </div>
          <button className="em-close" aria-label="Close" onClick={close}>
            ×
          </button>
        </div>
        <div className="qa-fix-body">
          <div className="qa-fix-sum">
            <strong>{fixes.length}</strong> fixable
            {design > 0 && (
              <>
                {" "}
                · <strong>{design}</strong> change the design
              </>
            )}{" "}
            · <strong>{plan.manual.length}</strong> need you
          </div>
          {fixes.length === 0 && !finished && (
            <div className="qa-empty">Nothing the model can fix on its own.</div>
          )}
          {fixMode === "auto" && !finished && fixes.length > 0 && (
            <div className="qa-fix-list">
              {fixes.map((f, i) => (
                <FixRow
                  key={i}
                  f={f}
                  checked={!off.includes(i)}
                  onToggle={() =>
                    setOff(off.includes(i) ? off.filter((x) => x !== i) : [...off, i])
                  }
                />
              ))}
            </div>
          )}
          {fixMode === "approve" && !finished && current && (
            <div className="qa-step">
              <div className="qa-step-count">
                CHANGE {step + 1} OF {fixes.length}
                <div className="std-bar">
                  <div style={{ width: `${(step / fixes.length) * 100}%` }} />
                </div>
              </div>
              <div className="qa-step-finding">For: {titleOf(current.finding)}</div>
              <FixRow f={current} />
            </div>
          )}
          {finished && (
            <div className="qa-fix-done">
              <strong>
                {fixResult ? fixResult.applied : done.applied} fixed
                {done.skipped ? ` · ${done.skipped} skipped` : ""}
              </strong>
              {fixResult && (
                <span>
                  Score {fixResult.before} → {fixResult.after}. Undo (Ctrl+Z) takes{" "}
                  {fixMode === "auto" ? "it all" : "each"} back.
                </span>
              )}
              {fixResult?.errors.map((e, i) => (
                <span key={i} className="group-error">
                  {e}
                </span>
              ))}
            </div>
          )}
          {plan.manual.length > 0 && (
            <section className="qa-manual" aria-label="Needs you">
              <div className="qa-manual-head">
                <h3>Needs you ({plan.manual.length})</h3>
                {askable.length > 0 && (
                  <button
                    className="qa-claude-fix"
                    disabled={asking || fixing}
                    onClick={() => void askClaude()}
                    title="Claude suggests a fix for each one and makes the changes for you to approve"
                  >
                    {Icons.sparkle}
                    {asking
                      ? "ASKING CLAUDE…"
                      : suggestions
                        ? "ASK CLAUDE AGAIN"
                        : `FIX ${askable.length === 1 ? "IT" : `ALL ${askable.length}`} WITH CLAUDE`}
                  </button>
                )}
              </div>
              {plan.manual.map(([id, why]) => {
                const i = (suggestions ?? []).findIndex((s) => s.finding === id);
                const s = i >= 0 ? suggestions![i] : undefined;
                return (
                  <div key={id} className="qa-manual-item">
                    <div>
                      <b>{titleOf(id)}</b> — {why}
                      {ruleOf(id) === "project-info" && (
                        <button className="link-btn" onClick={openProjectInfo}>
                          Open Project Info
                        </button>
                      )}
                    </div>
                    {s && (
                      <div className="qa-suggestion">
                        <p className="qa-advice">
                          <span>CLAUDE</span> {s.advice}
                        </p>
                        {s.fixes.map((f, j) => {
                          const key = `${i}:${j}`;
                          return (
                            <FixRow
                              key={key}
                              f={f}
                              checked={!claudeOff.includes(key)}
                              onToggle={() =>
                                setClaudeOff(
                                  claudeOff.includes(key)
                                    ? claudeOff.filter((x) => x !== key)
                                    : [...claudeOff, key],
                                )
                              }
                            />
                          );
                        })}
                        {s.dropped.map((d, k) => (
                          <span key={k} className="qa-dropped">
                            Skipped: {d}
                          </span>
                        ))}
                      </div>
                    )}
                  </div>
                );
              })}
            </section>
          )}
        </div>
        <div className="em-foot">
          <span className="em-hint">
            {fixMode === "auto"
              ? "One undo step for everything."
              : "Each approved change is its own undo step."}
          </span>
          {claudeFixes.length > 0 && (
            <button
              className="em-apply qa-apply-claude"
              disabled={fixing || chosenClaude.length === 0}
              onClick={() => void applyClaude()}
            >
              {fixing ? "FIXING…" : `APPLY CLAUDE'S ${chosenClaude.length} FIXES`}
            </button>
          )}
          {finished ? (
            <div className="em-actions">
              <button className="em-cancel" onClick={() => void again()}>
                LOOK AGAIN
              </button>
              <button className="em-apply" onClick={close}>
                DONE
              </button>
            </div>
          ) : fixMode === "auto" ? (
            <button
              className="em-apply"
              disabled={fixing || fixes.length - off.length === 0}
              onClick={() => void applyFixes(fixes.filter((_, i) => !off.includes(i)))}
            >
              {fixing ? "FIXING…" : `APPLY ${fixes.length - off.length} FIXES`}
            </button>
          ) : (
            current && (
              <div className="em-actions">
                <button className="em-cancel" onClick={() => setStep(fixes.length)}>
                  STOP
                </button>
                <button
                  className="em-cancel"
                  onClick={() => {
                    setDone({ ...done, skipped: done.skipped + 1 });
                    setStep(step + 1);
                  }}
                >
                  SKIP
                </button>
                <button
                  className="em-cancel"
                  disabled={fixing}
                  onClick={async () => {
                    const rest = fixes.slice(step);
                    const n = await applyFixes(rest, `QA/QC: Fix ${rest.length} issues`);
                    setDone({ ...done, applied: done.applied + n });
                    setStep(fixes.length);
                  }}
                >
                  APPLY ALL REMAINING
                </button>
                <button
                  className="em-apply"
                  disabled={fixing}
                  onClick={async () => {
                    // Applied on its own; the plan's other fixes stay as planned.
                    const [applied, , state] = await ipc.qaFixApply(
                      [current.action],
                      `QA/QC Fix: ${current.title}`,
                    );
                    if (state) useAppStore.getState().setApp(state);
                    setDone({ ...done, applied: done.applied + applied });
                    const next = step + 1;
                    setStep(next);
                    if (next >= fixes.length) {
                      const before = useQa.getState().report?.score ?? 0;
                      await runReview();
                      useQa.setState({
                        fixResult: {
                          applied: done.applied + applied,
                          errors: [],
                          before,
                          after: useQa.getState().report?.score ?? before,
                        },
                      });
                    }
                  }}
                >
                  APPLY
                </button>
              </div>
            )
          )}
        </div>
      </div>
    </div>
  );
}
