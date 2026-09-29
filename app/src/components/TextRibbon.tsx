import type { ReactNode } from "react";
import { useAppStore } from "../store";
import { LEADER_MODES, TEXT_ALIGNS, TEXT_TYPES, type LeaderMode } from "../text";
import type { TextAlign } from "../bindings/TextAlign";
import { Icons } from "./Icons";

// Revit's "Modify | Place Text" contextual tab (ADR-070): the text type, the leader
// (none, one segment, two segments or curved) and the paragraph's alignment.

const I = ({ children }: { children: ReactNode }) => (
  <svg
    viewBox="0 0 24 24"
    width="22"
    height="22"
    fill="none"
    stroke="currentColor"
    strokeWidth="1.6"
    aria-hidden
  >
    {children}
  </svg>
);

const A = (
  <text x="15" y="18" fontSize="11" fontWeight="700" stroke="none" fill="currentColor">
    A
  </text>
);

const LEADER_ICONS: Record<LeaderMode, ReactNode> = {
  None: (
    <I>
      <text x="7" y="17" fontSize="13" fontWeight="700" stroke="none" fill="currentColor">
        A
      </text>
    </I>
  ),
  One: (
    <I>
      <path d="M3 21l10-8" />
      <path d="M3 21l1.2-3.4 2 2.4z" fill="currentColor" />
      {A}
    </I>
  ),
  Two: (
    <I>
      <path d="M3 21l5-8h5" />
      <path d="M3 21l.6-3.6 2.4 1.5z" fill="currentColor" />
      {A}
    </I>
  ),
  Curved: (
    <I>
      <path d="M3 21c1-6 4-8 10-8" />
      <path d="M3 21l-.2-3.6 2.8.5z" fill="currentColor" />
      {A}
    </I>
  ),
};

const ALIGN_ICONS: Record<TextAlign, ReactNode> = {
  Left: (
    <I>
      <path d="M4 6h16M4 10h10M4 14h16M4 18h10" />
    </I>
  ),
  Center: (
    <I>
      <path d="M4 6h16M7 10h10M4 14h16M7 18h10" />
    </I>
  ),
  Right: (
    <I>
      <path d="M4 6h16M10 10h10M4 14h16M10 18h10" />
    </I>
  ),
};

export function TextRibbon() {
  const o = useAppStore((s) => s.options);
  const setOption = useAppStore((s) => s.setOption);
  const setTool = useAppStore((s) => s.setTool);
  return (
    <div className="ribbon text-ribbon" role="toolbar" aria-label="Tools">
      <div className="rb-tabs" role="tablist" aria-label="Ribbon tabs">
        <button role="tab" aria-selected className="rb-tab active contextual">
          Modify | Place Text
        </button>
      </div>
      <div className="rb-body">
        <div className="rb-group">
          <div className="rb-items">
            <button className="rb-btn" onClick={() => setTool("select")} title="Modify (Esc)">
              {Icons.select}
              <span>Modify</span>
            </button>
          </div>
          <div className="rb-title">Select</div>
        </div>
        <div className="rb-group">
          <div className="rb-items">
            <label className="rb-field">
              <span>Type</span>
              <select
                aria-label="Text Type"
                value={o.textSize}
                onChange={(e) => setOption("textSize", Number(e.target.value))}
              >
                {TEXT_TYPES.map(([mm, label]) => (
                  <option key={mm} value={mm}>
                    {label}
                  </option>
                ))}
              </select>
            </label>
          </div>
          <div className="rb-title">Properties</div>
        </div>
        <div className="rb-group">
          <div className="rb-items">
            <div className="rb-seg" role="radiogroup" aria-label="Leader">
              {LEADER_MODES.map(([m, label]) => (
                <button
                  key={m}
                  role="radio"
                  aria-checked={o.textLeader === m}
                  aria-label={label}
                  title={label}
                  className={`rb-btn rb-small${o.textLeader === m ? " active" : ""}`}
                  onClick={() => setOption("textLeader", m)}
                >
                  {LEADER_ICONS[m]}
                </button>
              ))}
            </div>
            <div className="rb-seg" role="radiogroup" aria-label="Alignment">
              {TEXT_ALIGNS.map(([a, label]) => (
                <button
                  key={a}
                  role="radio"
                  aria-checked={o.textAlign === a}
                  aria-label={label}
                  title={label}
                  className={`rb-btn rb-small${o.textAlign === a ? " active" : ""}`}
                  onClick={() => setOption("textAlign", a)}
                >
                  {ALIGN_ICONS[a]}
                </button>
              ))}
            </div>
          </div>
          <div className="rb-title">Format</div>
        </div>
      </div>
    </div>
  );
}
