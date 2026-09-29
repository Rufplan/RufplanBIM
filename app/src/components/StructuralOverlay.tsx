import type { OverlayInfo } from "../bindings/OverlayInfo";
import { STRUCT_COLORS, STRUCT_LEGEND } from "../render/structural";
import { DISCLAIMER } from "./StructuralDialog";

// The structural overlay's legend and its hover/click card (ADR-080).

export interface InfoAt {
  info: OverlayInfo;
  x: number;
  y: number;
  pinned: boolean;
}

export function StructuralInfoCard({ at, onClose }: { at: InfoAt; onClose: () => void }) {
  return (
    <div
      className={`st-info${at.pinned ? " pinned" : ""}`}
      role={at.pinned ? "dialog" : "tooltip"}
      aria-label={at.info.title}
      style={{ left: at.x + 14, top: at.y + 14 }}
    >
      <h5 style={{ color: STRUCT_COLORS[at.info.kind] }}>{at.info.title}</h5>
      {at.info.lines.map((l) => (
        <p key={l}>{l}</p>
      ))}
      {at.pinned && (
        <button className="btn-ghost" onClick={onClose}>
          Close
        </button>
      )}
    </div>
  );
}

export function StructuralLegend() {
  return (
    <div className="st-legend" aria-label="Structural overlay legend">
      <b>Structural layer</b>
      {STRUCT_LEGEND.map(([k, label]) => (
        <span key={k}>
          <i style={{ background: STRUCT_COLORS[k] }} />
          {label}
        </span>
      ))}
      <small>{DISCLAIMER}</small>
    </div>
  );
}
