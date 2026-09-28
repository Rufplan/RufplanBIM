// Enscape-style navigation in the 3D view (ADR-063): Fly (free movement) and Walk (at eye
// height, following floors and stairs, stopped by walls). Pure: the view supplies the
// ground and the walls; mm, z up.

export type NavMode = "orbit" | "fly" | "walk";

export interface NavState {
  pos: [number, number, number];
  /** Heading, radians counter-clockwise from +x (east); pitch up from level. */
  yaw: number;
  pitch: number;
}

export interface NavInput {
  forward: number; // -1 back … 1 forward
  right: number; // -1 left … 1 right
  up: number; // -1 down … 1 up (fly only)
  fast: boolean;
}

/** Walk eye height (5'-5"), how high a step it climbs (a stair riser and a bit), and the
 * clearance it keeps from walls. */
export const EYE = 1650;
export const STEP = 450;
export const CLEARANCE = 350;

/** Base speeds, mm a second: walking pace and a brisk fly. */
export const WALK_SPEED = 1500;
export const FLY_SPEED = 5000;

/** Where the camera looks. */
export function lookDir(s: Pick<NavState, "yaw" | "pitch">): [number, number, number] {
  const c = Math.cos(s.pitch);
  return [Math.cos(s.yaw) * c, Math.sin(s.yaw) * c, Math.sin(s.pitch)];
}

/** The state for a camera at `pos` looking at `target`. */
export function fromLook(
  pos: [number, number, number],
  target: [number, number, number],
): NavState {
  const d = [target[0] - pos[0], target[1] - pos[1], target[2] - pos[2]];
  const flat = Math.hypot(d[0]!, d[1]!);
  return {
    pos: [...pos],
    yaw: Math.atan2(d[1]!, d[0]!),
    pitch: Math.atan2(d[2]!, flat || 1e-9),
  };
}

/** Turns the view by a mouse drag of (dx, dy) px. */
export function look(s: NavState, dx: number, dy: number, sensitivity = 0.0035): NavState {
  const lim = (85 * Math.PI) / 180;
  return {
    ...s,
    yaw: s.yaw - dx * sensitivity,
    pitch: Math.max(-lim, Math.min(lim, s.pitch - dy * sensitivity)),
  };
}

export interface World {
  /** The top of whatever is under (x, y) at or below `z`, or null. */
  ground: (x: number, y: number, z: number) => number | null;
  /** Whether moving from `a` to `b` at walking height runs into something. */
  blocked: (a: [number, number, number], b: [number, number, number]) => boolean;
}

/** One step of `dt` seconds. Fly moves where it looks (and up and down); Walk moves on the
 * level, keeps its eyes EYE above the ground (climbing steps up to STEP, easing down to
 * lower floors) and slides along walls instead of passing through them. */
export function navStep(
  s: NavState,
  input: NavInput,
  dt: number,
  mode: NavMode,
  speedScale = 1,
  world?: World,
): NavState {
  if (mode === "orbit") return s;
  const speed = (mode === "walk" ? WALK_SPEED : FLY_SPEED) * speedScale * (input.fast ? 3 : 1);
  const d = speed * dt;
  const fwd = mode === "fly" ? lookDir(s) : [Math.cos(s.yaw), Math.sin(s.yaw), 0];
  const right = [Math.sin(s.yaw), -Math.cos(s.yaw), 0];
  const move = [0, 1, 2].map((k) => (fwd[k]! * input.forward + right[k]! * input.right) * d) as [
    number,
    number,
    number,
  ];
  if (mode === "fly") {
    move[2] += input.up * d;
    return { ...s, pos: [s.pos[0] + move[0], s.pos[1] + move[1], s.pos[2] + move[2]] };
  }
  // Walk: slide along walls (try the move, then each axis alone).
  let [x, y] = [s.pos[0], s.pos[1]];
  const z = s.pos[2];
  const feet = z - EYE;
  const ok = (nx: number, ny: number) => {
    if (!world) return true;
    const knee = feet + STEP + 50;
    // Look a little further than the step, so it stops short of the wall.
    const len = Math.hypot(nx - x, ny - y) || 1;
    const k = (len + CLEARANCE) / len;
    const reach: [number, number, number] = [x + (nx - x) * k, y + (ny - y) * k, knee];
    return !world.blocked([x, y, knee], reach);
  };
  if (move[0] || move[1]) {
    if (ok(x + move[0], y + move[1])) {
      x += move[0];
      y += move[1];
    } else if (move[0] && ok(x + move[0], y)) {
      x += move[0];
    } else if (move[1] && ok(x, y + move[1])) {
      y += move[1];
    }
  }
  // Onto the ground: up a step at most, down gently.
  const g = world?.ground(x, y, feet + STEP);
  let nz = z;
  if (g !== null && g !== undefined) {
    const want = g + EYE;
    nz = want > z ? want : z + (want - z) * Math.min(1, dt * 8);
  }
  return { ...s, pos: [x, y, nz] };
}
