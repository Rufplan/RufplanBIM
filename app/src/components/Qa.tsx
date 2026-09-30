import { useMemo, useState } from "react";
import { create } from "zustand";
import type { QaCategory } from "../bindings/QaCategory";
import type { QaFinding } from "../bindings/QaFinding";
import type { QaMilestone } from "../bindings/QaMilestone";
import type { QaReport } from "../bindings/QaReport";
import type { QaSeverity } from "../bindings/QaSeverity";
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
}));

export async function runReview() {
  const s = useQa.getState();
  useQa.setState({ running: true });
  try {
    const report = await ipc.qaReview({ milestone: s.milestone, categories: s.categories });
    useQa.setState({ report, open: true, category: null, severity: null });
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
  const { milestone, categories, running, claude, report } = useQa();
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
  const { report, open, resolved, category, severity, query, hideResolved, claude } = useQa();
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
