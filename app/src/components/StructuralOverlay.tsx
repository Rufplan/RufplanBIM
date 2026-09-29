import type { OverlayInfo } from "../bindings/OverlayInfo";
import { MEP_COLORS, STRUCT_COLORS, STRUCT_LEGEND } from "../render/structural";
import type { MepKind } from "../bindings/MepKind";
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
      <h5
        style={{
          color: at.info.mep ? MEP_COLORS[at.info.mep] : STRUCT_COLORS[at.info.kind],
        }}
      >
        {at.info.title}
      </h5>
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

/** The MEPT overlays' legend: the kinds on show, in a discipline's colours. */
export function MepLegend({ kinds, shift }: { kinds: MepKind[]; shift?: boolean }) {
  return (
    <div
      className="st-legend mep-legend"
      aria-label="MEPT overlay legend"
      style={shift ? { left: 300 } : undefined}
    >
      <b>MEPT layers</b>
      {kinds.map((k) => (
        <span key={k}>
          <i style={{ background: MEP_COLORS[k] }} />
          {MEP_LABELS[k] ?? k}
        </span>
      ))}
      <small>Preliminary — not engineered. Requires review by licensed engineers.</small>
    </div>
  );
}

const MEP_LABELS: Partial<Record<MepKind, string>> = {
  OutdoorUnit: "Outdoor units",
  IndoorUnit: "Indoor units",
  SupplyDuct: "Supply ducts",
  ReturnGrille: "Returns",
  WaterHeater: "Water heater",
  WaterService: "Water service",
  BuildingDrain: "Building drain",
  ColdWater: "Cold water",
  HotWater: "Hot water",
  DataOutlet: "Data outlets",
  AccessPoint: "Wi-Fi APs",
  AccessControl: "Access control",
  AvDisplay: "AV",
};

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
