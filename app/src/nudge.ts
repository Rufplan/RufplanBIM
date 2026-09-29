// Nudge (ADR-075): the arrow keys move the selection a small step, as in Revit. The step
// follows the zoom: the smallest round distance that shows as a few pixels on screen, and
// Shift moves about ten times as far.
import type { Pt } from "./bindings/Pt";

/** Round distances in inches, from 1/32" to 100'. */
const STEPS_IN = [1 / 32, 1 / 16, 1 / 8, 1 / 4, 1 / 2, 1, 2, 3, 6, 12, 24, 60, 120, 240, 600, 1200];

/** The nudge in mm at `zoom` (pixels per mm); `far` is Shift. */
export function nudgeStep(zoom: number, far: boolean): number {
  const px = far ? 40 : 4;
  const want = px / Math.max(zoom, 1e-9);
  const step = STEPS_IN.map((i) => i * 25.4).find((mm) => mm >= want);
  return step ?? STEPS_IN[STEPS_IN.length - 1]! * 25.4;
}

/** The arrow key's direction in model coordinates (y up), or null for any other key. */
export function nudgeDirection(key: string): Pt | null {
  switch (key) {
    case "ArrowLeft":
      return { x: -1, y: 0 };
    case "ArrowRight":
      return { x: 1, y: 0 };
    case "ArrowUp":
      return { x: 0, y: 1 };
    case "ArrowDown":
      return { x: 0, y: -1 };
    default:
      return null;
  }
}

/** Whether a key press belongs to a field or a keyboard-driven control, not the view. */
export function keyForControl(target: EventTarget | null): boolean {
  const t = target as HTMLElement | null;
  if (!t || typeof t.closest !== "function") return false;
  if (/^(input|textarea|select)$/i.test(t.tagName) || t.isContentEditable) return true;
  return !!t.closest(
    '[role="menu"],[role="tablist"],[role="radiogroup"],[role="listbox"],[role="dialog"]',
  );
}
