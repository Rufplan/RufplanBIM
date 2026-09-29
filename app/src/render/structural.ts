// The structural overlay (ADR-080), drawn over the greyed-out architecture: a colour per
// element type, with adjustable transparency. The geometry is Rust's (studio-views
// `structural`); this only paints it.
import * as THREE from "three";
import type { MepKind } from "../bindings/MepKind";
import type { OverlayKind } from "../bindings/OverlayKind";
import type { OverlayMesh } from "../bindings/OverlayMesh";
import type { OverlayPrim } from "../bindings/OverlayPrim";

/** Colours by element type, as a structural drawing set's layers often read. */
export const STRUCT_COLORS: Record<OverlayKind, string> = {
  Grid: "#7a8691",
  Column: "#d32f2f",
  Girder: "#1565c0",
  Beam: "#42a5f5",
  BearingWall: "#8d6e63",
  ShearWall: "#ef6c00",
  BracedFrame: "#8e24aa",
  MomentFrame: "#c2185b",
  Span: "#2e7d32",
  Transfer: "#ff1744",
  Flag: "#ffb300",
  Mep: "#607d8b",
  MepZone: "#90caf9",
};

/** MEPT colours (ADR-082): a hue family per discipline — mechanical purples and blues,
 * electrical oranges and yellow, plumbing teals, greens and red (hot), technology indigo. */
export const MEP_COLORS: Record<MepKind, string> = {
  Equipment: "#6a1b9a",
  OutdoorUnit: "#8e24aa",
  IndoorUnit: "#ab47bc",
  Diffuser: "#1e88e5",
  ReturnGrille: "#546e7a",
  SupplyDuct: "#42a5f5",
  Refrigerant: "#7e57c2",
  Piping: "#5e35b1",
  Shaft: "#455a64",
  Service: "#e65100",
  Panel: "#ef6c00",
  Transformer: "#bf360c",
  Generator: "#bf360c",
  Feeder: "#fb8c00",
  Light: "#f9a825",
  Receptacle: "#f57c00",
  WaterService: "#00838f",
  WaterHeater: "#c62828",
  Fixture: "#0097a7",
  Stack: "#2e7d32",
  BuildingDrain: "#388e3c",
  ColdWater: "#0277bd",
  HotWater: "#e53935",
  Vent: "#66bb6a",
  Mdf: "#283593",
  Idf: "#3949ab",
  Pathway: "#5c6bc0",
  Backbone: "#1a237e",
  DataOutlet: "#3f51b5",
  AccessPoint: "#00acc1",
  Camera: "#37474f",
  AccessControl: "#455a64",
  AvDisplay: "#6d4c41",
};

/** A prim's colour: structural by kind, MEPT by its item. */
export const primColor = (p: { kind: OverlayKind; mep?: MepKind }) =>
  p.kind === "Mep" && p.mep ? MEP_COLORS[p.mep] : STRUCT_COLORS[p.kind];

/** The overlay's legend order and labels. */
export const STRUCT_LEGEND: [OverlayKind, string][] = [
  ["Column", "Columns"],
  ["Girder", "Girders"],
  ["Beam", "Beams"],
  ["Span", "Joists/deck"],
  ["BearingWall", "Bearing walls"],
  ["ShearWall", "Shear walls"],
  ["BracedFrame", "Braced frames"],
  ["MomentFrame", "Moment frames"],
  ["Transfer", "Transfers"],
  ["Flag", "Flags"],
];

const LINE_PX: Partial<Record<OverlayKind, number>> = {
  Grid: 1,
  Girder: 3,
  Beam: 1.5,
  Transfer: 3.5,
  BracedFrame: 2,
  MomentFrame: 3,
  Span: 1.25,
};

/** Paints the plan overlay. `S` maps model mm to screen pixels. */
export function drawStructural(
  ctx: CanvasRenderingContext2D,
  S: (x: number, y: number) => [number, number],
  prims: OverlayPrim[],
  alpha: number,
) {
  ctx.save();
  for (const p of prims) {
    const color = primColor(p);
    const a =
      p.kind === "Flag"
        ? 1
        : p.kind === "MepZone"
          ? alpha * 0.2
          : p.kind === "Grid"
            ? Math.min(alpha, 0.8)
            : alpha;
    ctx.globalAlpha = a;
    for (const ring of p.fill) {
      ctx.beginPath();
      ring.forEach(([x, y], i) => {
        const [sx, sy] = S(x, y);
        if (i === 0) ctx.moveTo(sx, sy);
        else ctx.lineTo(sx, sy);
      });
      ctx.closePath();
      ctx.fillStyle = color;
      ctx.fill();
      ctx.strokeStyle = color;
      ctx.lineWidth = 1;
      ctx.setLineDash([]);
      ctx.stroke();
    }
    ctx.strokeStyle = color;
    ctx.lineWidth = LINE_PX[p.kind] ?? 1.5;
    ctx.setLineDash(p.kind === "Grid" ? [10, 4, 2, 4] : []);
    for (const line of p.lines) {
      ctx.beginPath();
      line.forEach(([x, y], i) => {
        const [sx, sy] = S(x, y);
        if (i === 0) ctx.moveTo(sx, sy);
        else ctx.lineTo(sx, sy);
      });
      ctx.stroke();
    }
    if (p.label) {
      const [text, [x, y]] = p.label;
      const [sx, sy] = S(x, y);
      ctx.setLineDash([]);
      ctx.globalAlpha = Math.max(a, 0.9);
      ctx.font = `${p.kind === "Flag" ? "bold 12" : p.kind === "Grid" ? "600 12" : "10"}px Inter, Arial, sans-serif`;
      ctx.textAlign = "center";
      ctx.textBaseline = "middle";
      ctx.fillStyle = p.kind === "Flag" ? "#1b1b1b" : p.kind === "MepZone" ? "#0d47a1" : color;
      ctx.fillText(text, sx, sy);
    }
  }
  ctx.restore();
}

/** Builds the 3D overlay: coloured, see-through boxes. */
export function buildStructural3d(meshes: OverlayMesh[], alpha: number): THREE.Group {
  const g = new THREE.Group();
  g.name = "structural-overlay";
  for (const m of meshes) {
    const geo = new THREE.BufferGeometry();
    geo.setAttribute("position", new THREE.Float32BufferAttribute(m.positions, 3));
    geo.computeVertexNormals();
    const mat = new THREE.MeshLambertMaterial({
      color: primColor(m),
      transparent: alpha < 1 || m.kind === "Flag",
      opacity: m.kind === "Flag" ? 0.95 : alpha,
      side: THREE.DoubleSide,
      depthWrite: alpha >= 0.95,
    });
    const mesh = new THREE.Mesh(geo, mat);
    mesh.userData = {
      structural: m.kind !== "Mep" && !m.mep,
      mep: !!m.mep || m.kind === "Mep",
      member: m.member ?? null,
      flag: m.flag ?? null,
    };
    mesh.renderOrder = 5;
    g.add(mesh);
  }
  return g;
}

export function disposeGroup(g: THREE.Group) {
  for (const c of [...g.children]) {
    g.remove(c);
    if (c instanceof THREE.Mesh) {
      c.geometry.dispose();
      (c.material as THREE.Material).dispose();
    }
  }
}

/** Greys the architecture out under the overlay (or restores it). */
export function ghostArchitecture(group: THREE.Group, on: boolean) {
  group.traverse((o) => {
    const obj = o as THREE.Mesh | THREE.LineSegments;
    const mat = (obj as THREE.Mesh).material as THREE.Material | THREE.Material[] | undefined;
    if (!mat) return;
    for (const m of Array.isArray(mat) ? mat : [mat]) {
      const u = m.userData as {
        ghost?: { opacity: number; transparent: boolean; depthWrite: boolean };
      };
      if (on) {
        u.ghost ??= { opacity: m.opacity, transparent: m.transparent, depthWrite: m.depthWrite };
        m.transparent = true;
        m.opacity = u.ghost.opacity * 0.18;
        m.depthWrite = false;
      } else if (u.ghost) {
        m.opacity = u.ghost.opacity;
        m.transparent = u.ghost.transparent;
        m.depthWrite = u.ghost.depthWrite;
        delete u.ghost;
      }
      m.needsUpdate = true;
    }
  });
}
