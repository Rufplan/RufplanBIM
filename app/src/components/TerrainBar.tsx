// The terrain's controls in a 3D view (ADR-045): the ground as a block of earth (Revit's
// toposolid), contour lines on it, their labels, and the contour interval.

/** Contour intervals offered, mm. */
export const INTERVALS: [number, string][] = [
  [152.4, `6"`],
  [304.8, `1'`],
  [609.6, `2'`],
  [1524, `5'`],
  [3048, `10'`],
  [6096, `20'`],
];

const ink = "#1b1d1f";

const solidIcon = (
  <svg width="18" height="18" viewBox="0 0 24 24" aria-hidden>
    <path d="M3 9c3-3 6 1 9-1s6-3 9 0v4H3z" fill="#8fae6b" stroke={ink} strokeWidth="1.2" />
    <path d="M3 12h18v8H3z" fill="#9a7650" stroke={ink} strokeWidth="1.2" />
    <path d="M3 15.5h18M3 18h18" stroke="#6e5236" strokeWidth=".9" />
  </svg>
);

const contourIcon = (
  <svg width="18" height="18" viewBox="0 0 24 24" aria-hidden fill="none" stroke={ink}>
    <path d="M2 7c4-3 7 2 11 0s6-3 9-1" strokeWidth="1.2" />
    <path d="M2 12c4-3 7 2 11 0s6-3 9-1" strokeWidth="2" />
    <path d="M2 17c4-3 7 2 11 0s6-3 9-1" strokeWidth="1.2" />
  </svg>
);

const labelIcon = (
  <svg width="18" height="18" viewBox="0 0 24 24" aria-hidden fill="none" stroke={ink}>
    <path d="M2 14c4-3 7 2 11 0s6-3 9-1" strokeWidth="1.2" />
    <rect x="7" y="4" width="10" height="7" rx="1" fill="#fff" strokeWidth="1.2" />
    <path d="M9.5 7.5h5" strokeWidth="1.4" />
  </svg>
);

export function TerrainBar({
  solid,
  contours,
  labels,
  interval,
  onSolid,
  onContours,
  onLabels,
  onInterval,
}: {
  solid: boolean;
  contours: boolean;
  labels: boolean;
  /** Current contour interval, mm. */
  interval: number | null;
  onSolid: (on: boolean) => void;
  onContours: (on: boolean) => void;
  onLabels: (on: boolean) => void;
  onInterval: (mm: number) => void;
}) {
  const known = INTERVALS.find(([mm]) => interval !== null && Math.abs(mm - interval) < 0.5)?.[0];
  return (
    <div className="terrain-bar" role="toolbar" aria-label="Terrain">
      <button
        className={solid ? "on" : undefined}
        aria-pressed={solid}
        aria-label="Earth depth"
        title="Show the ground as a block cut out of the earth (Revit's toposolid)"
        onClick={() => onSolid(!solid)}
      >
        {solidIcon}
      </button>
      <button
        className={contours ? "on" : undefined}
        aria-pressed={contours}
        aria-label="Contours"
        title="Contour lines on the ground (every fifth heavier)"
        onClick={() => onContours(!contours)}
      >
        {contourIcon}
      </button>
      <button
        className={labels ? "on" : undefined}
        aria-pressed={labels}
        aria-label="Contour labels"
        title="Label the contours with their elevations"
        disabled={!contours}
        onClick={() => onLabels(!labels)}
      >
        {labelIcon}
      </button>
      <select
        aria-label="Contour interval"
        title="Contour interval"
        value={interval === null ? "" : known !== undefined ? String(known) : "custom"}
        disabled={!contours || interval === null}
        onChange={(e) => {
          const mm = Number(e.target.value);
          if (mm > 0) onInterval(mm);
        }}
      >
        {known === undefined && interval !== null && (
          <option value="custom">{(interval / 25.4).toFixed(0)}"</option>
        )}
        {INTERVALS.map(([mm, label]) => (
          <option key={mm} value={String(mm)}>
            {label}
          </option>
        ))}
      </select>
    </div>
  );
}
