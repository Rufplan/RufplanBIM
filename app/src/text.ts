// Revit's Text tool (ADR-070): its options, the leader clicks it needs, and the in-place
// editor's placement. The note itself (layout, leaders) is Rust's.
import type { Leader } from "./bindings/Leader";
import type { TextAlign } from "./bindings/TextAlign";
import type { Pt } from "./ipc";

/** Revit's leader choices on Modify | Place Text. */
export type LeaderMode = "None" | "One" | "Two" | "Curved";

export const LEADER_MODES: [LeaderMode, string][] = [
  ["None", "No Leader"],
  ["One", "One Segment"],
  ["Two", "Two Segments"],
  ["Curved", "Curved"],
];

/** Revit's text types: Arial at these printed heights (paper mm). */
export const TEXT_TYPES: [number, string][] = [
  [2.4, `3/32" Arial`],
  [3.0, `1/8" Arial`],
  [4.8, `3/16" Arial`],
  [6.4, `1/4" Arial`],
  [12.7, `1/2" Arial`],
];

export const TEXT_ALIGNS: [TextAlign, string][] = [
  ["Left", "Align Left"],
  ["Center", "Align Center"],
  ["Right", "Align Right"],
];

/** Clicks before the text's own: the arrowhead, then (two segments) the elbow. */
export function leaderClicks(mode: LeaderMode): number {
  return mode === "Two" ? 2 : mode === "None" ? 0 : 1;
}

/** The leader from the clicks made so far. */
export function leadersFrom(mode: LeaderMode, pts: Pt[]): Leader[] {
  if (mode === "None" || pts.length === 0) return [];
  const end = pts[0]!;
  if (mode === "Two") return [{ end, elbow: pts[1] ?? null, arc: false }];
  return [{ end, elbow: null, arc: mode === "Curved" }];
}

/** The status bar's prompt for the Text tool after `n` clicks. */
export function textPrompt(mode: LeaderMode, n: number): string {
  const need = leaderClicks(mode);
  if (n < need)
    return n === 0
      ? "Click where the leader's arrowhead points"
      : "Click the leader's elbow, then where the text goes";
  return "Click to place the text (drag to set its width), then type. Click outside or Esc to finish";
}
