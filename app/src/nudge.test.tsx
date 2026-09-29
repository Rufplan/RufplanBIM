import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { App } from "./App";
import { useAppStore } from "./store";
import { installFakeBackend, type FakeBackend } from "./test/fakeBackend";
import { keyForControl, nudgeDirection, nudgeStep } from "./nudge";

let fake: FakeBackend;
class SizedObserver {
  constructor(private cb: ResizeObserverCallback) {}
  observe() {
    this.cb([], this as unknown as ResizeObserver);
  }
  unobserve() {}
  disconnect() {}
}
const RealObserver = globalThis.ResizeObserver;

beforeEach(() => {
  useAppStore.setState({
    app: null,
    error: null,
    openViews: [],
    activeView: null,
    selection: [],
    tool: "select",
    picker: null,
    viewDialog: null,
  });
  fake = installFakeBackend();
});
afterEach(() => {
  globalThis.ResizeObserver = RealObserver;
  vi.restoreAllMocks();
});

const argsOf = (cmd: string) =>
  fake.calls.filter((c) => c.cmd === cmd).map((c) => c.args as Record<string, unknown>);

async function openPlan() {
  globalThis.ResizeObserver = SizedObserver as unknown as typeof ResizeObserver;
  vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockReturnValue({
    x: 0,
    y: 0,
    left: 0,
    top: 0,
    right: 800,
    bottom: 600,
    width: 800,
    height: 600,
    toJSON: () => ({}),
  } as DOMRect);
  const { container } = render(<App />);
  await userEvent.click(screen.getByRole("button", { name: "New Project" }));
  await screen.findByRole("toolbar", { name: "Tools" });
  await waitFor(() => expect(container.querySelector("canvas")).not.toBeNull());
}

describe("nudge steps (ADR-075)", () => {
  it("are round distances of a few pixels, ten times farther with Shift", () => {
    // 1 px per mm: 4 px wants 4 mm, so 1/4" (6.35 mm); Shift wants 40 mm, so 2" (50.8 mm).
    expect(nudgeStep(1, false)).toBeCloseTo(6.35);
    expect(nudgeStep(1, true)).toBeCloseTo(50.8);
    // Zoomed out to 0.01 px per mm: 400 mm, so 2' (609.6 mm); Shift 4000 mm, so 20'.
    expect(nudgeStep(0.01, false)).toBeCloseTo(609.6);
    expect(nudgeStep(0.01, true)).toBeCloseTo(6096);
    // Zoomed right in: never below 1/32".
    expect(nudgeStep(1000, false)).toBeCloseTo(25.4 / 32);
  });

  it("map the arrows to model directions, y up", () => {
    expect(nudgeDirection("ArrowUp")).toEqual({ x: 0, y: 1 });
    expect(nudgeDirection("ArrowLeft")).toEqual({ x: -1, y: 0 });
    expect(nudgeDirection("a")).toBeNull();
  });

  it("leave arrows to fields and keyboard controls", () => {
    expect(keyForControl(document.createElement("input"))).toBe(true);
    const menu = document.createElement("div");
    menu.setAttribute("role", "menu");
    const item = document.createElement("button");
    menu.appendChild(item);
    expect(keyForControl(item)).toBe(true);
    expect(keyForControl(document.body)).toBe(false);
  });
});

describe("arrow keys in a plan (ADR-075)", () => {
  it("move the selection, only when something is selected and Modify is active", async () => {
    await openPlan();
    fireEvent.keyDown(window, { key: "ArrowRight" });
    expect(argsOf("move_elements")).toHaveLength(0);

    useAppStore.setState({ selection: ["w1"] });
    fireEvent.keyDown(window, { key: "ArrowRight" });
    await waitFor(() => expect(argsOf("move_elements")).toHaveLength(1));
    const near = argsOf("move_elements")[0]!.delta as { x: number; y: number };
    expect(near.x).toBeGreaterThan(0);
    expect(near.y).toBe(0);

    fireEvent.keyDown(window, { key: "ArrowDown", shiftKey: true });
    await waitFor(() => expect(argsOf("move_elements")).toHaveLength(2));
    const far = argsOf("move_elements")[1]!.delta as { x: number; y: number };
    expect(far.x).toBe(0);
    expect(-far.y).toBeGreaterThan(near.x * 5);

    useAppStore.setState({ tool: "wall" });
    fireEvent.keyDown(window, { key: "ArrowRight" });
    expect(argsOf("move_elements")).toHaveLength(2);
  });
});
