import { describe, expect, it } from "vitest";
import {
  EYE,
  FLY_SPEED,
  fromLook,
  look,
  lookDir,
  navStep,
  STEP,
  WALK_SPEED,
  type World,
} from "./render/navigate";

const still = { forward: 0, right: 0, up: 0, fast: false };
const ahead = { ...still, forward: 1 };

describe("Enscape-style Walk and Fly (ADR-063)", () => {
  it("looks where the camera points, and a drag turns it", () => {
    const s = fromLook([0, 0, 1000], [1000, 1000, 1000]);
    expect(s.yaw).toBeCloseTo(Math.PI / 4, 9);
    expect(s.pitch).toBeCloseTo(0, 9);
    const d = lookDir(s);
    expect(d[0]).toBeCloseTo(Math.SQRT1_2, 9);
    // Dragging right turns right (clockwise); up/down is clamped short of straight.
    expect(look(s, 100, 0).yaw).toBeLessThan(s.yaw);
    expect(look(s, 0, -1e6).pitch).toBeCloseTo((85 * Math.PI) / 180, 9);
  });

  it("flies where it looks, up and down with Q/E, faster with Shift", () => {
    const s = fromLook([0, 0, 0], [0, 1000, 1000]);
    const n = navStep(s, ahead, 1, "fly");
    expect(n.pos[1]).toBeCloseTo(FLY_SPEED * Math.SQRT1_2, 6);
    expect(n.pos[2]).toBeCloseTo(FLY_SPEED * Math.SQRT1_2, 6);
    const up = navStep(s, { ...still, up: 1 }, 0.5, "fly");
    expect(up.pos[2]).toBeCloseTo(FLY_SPEED / 2, 9);
    const fast = navStep(s, { ...ahead, fast: true }, 1, "fly");
    expect(Math.hypot(...fast.pos)).toBeCloseTo(FLY_SPEED * 3, 6);
    // Orbit doesn't move.
    expect(navStep(s, ahead, 1, "orbit")).toBe(s);
  });

  it("walks on the level at eye height, climbs a step, and stops at walls", () => {
    // A floor at 0, a 7" step up beyond x = 1000, a wall at x = 3000.
    const world: World = {
      ground: (x) => (x > 1000 ? 180 : 0),
      blocked: (a, b) => a[0] < 3000 !== b[0] < 3000,
    };
    // Looking down at the floor, it still walks level.
    let s = { ...fromLook([0, 0, EYE], [1000, 0, 0]) };
    s = navStep(s, ahead, 0.5, "walk", 1, world);
    expect(s.pos[0]).toBeCloseTo(WALK_SPEED / 2, 9);
    expect(s.pos[2]).toBe(EYE);
    // Up the step: at once, eyes above the new floor.
    s = navStep(s, ahead, 0.5, "walk", 1, world);
    expect(s.pos[2]).toBe(180 + EYE);
    expect(180).toBeLessThan(STEP);
    // Walk on: it stops short of the wall, whatever the time.
    for (let i = 0; i < 20; i++) s = navStep(s, ahead, 0.5, "walk", 1, world);
    expect(s.pos[0]).toBeLessThan(3000);
    expect(s.pos[0]).toBeGreaterThan(2000);
    // Sliding: along the wall (y) is still free.
    const slid = navStep({ ...s, yaw: Math.PI / 4 }, ahead, 1, "walk", 1, world);
    expect(slid.pos[1]).toBeGreaterThan(s.pos[1] + 500);
  });
});
