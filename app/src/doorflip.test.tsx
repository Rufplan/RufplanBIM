import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { App } from "./App";
import { useAppStore } from "./store";
import { installFakeBackend, type FakeBackend } from "./test/fakeBackend";
import { drawFlipControls } from "./canvas/render";

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
    ribbonTab: "Architecture",
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
  return container.querySelector("canvas")!;
}

describe("Door flips (ADR-091)", () => {
  it("the spacebar turns the door being placed, and it goes in turned", async () => {
    const canvas = await openPlan();
    const s = useAppStore.getState();
    s.setToolType("door", s.app!.doorTypes[0]!.id);
    s.setTool("door");
    fireEvent.mouseMove(canvas, { clientX: 300, clientY: 200 });
    await waitFor(() => expect(argsOf("opening_preview").at(-1)).toMatchObject({ turns: 0 }));
    fireEvent.keyDown(window, { key: " " });
    await waitFor(() => expect(argsOf("opening_preview").at(-1)).toMatchObject({ turns: 1 }));
    fireEvent.keyDown(window, { key: " " });
    await waitFor(() => expect(argsOf("opening_preview").at(-1)).toMatchObject({ turns: 2 }));
    fireEvent.mouseDown(canvas, { button: 0, clientX: 300, clientY: 200 });
    fireEvent.mouseUp(canvas, { button: 0, clientX: 300, clientY: 200 });
    await waitFor(() => expect(argsOf("create_opening")).toHaveLength(1));
    expect(argsOf("create_opening")[0]).toMatchObject({ flipHand: true, flipFacing: true });
    // A new tool starts from the cursor's own swing again.
    useAppStore.getState().setTool("window");
    expect(useAppStore.getState().openingTurns).toBe(0);
  });

  it("the spacebar flips the selection when nothing is being placed", async () => {
    await openPlan();
    useAppStore.getState().select(["d1"]);
    fireEvent.keyDown(window, { key: " " });
    await waitFor(() => expect(argsOf("flip_selection").at(-1)).toMatchObject({ ids: ["d1"] }));
  });

  it("flip controls draw as double arrows along their direction", () => {
    const calls: [number, number][] = [];
    const ctx = {
      save() {},
      restore() {},
      setLineDash() {},
      beginPath() {},
      closePath() {},
      fill() {},
      stroke() {},
      arc() {},
      moveTo: (x: number, y: number) => calls.push([x, y]),
      lineTo: (x: number, y: number) => calls.push([x, y]),
    } as unknown as CanvasRenderingContext2D;
    drawFlipControls(
      ctx,
      { cx: 0, cy: 0, zoom: 1 },
      200,
      200,
      [{ at: { x: 0, y: 0 }, dir: { x: 1, y: 0 } }],
      null,
    );
    const xs = calls.map((c) => c[0]);
    const ys = calls.map((c) => c[1]);
    // Left/right: 20 px long across the screen, 10 px tall, centered on the control.
    expect(Math.max(...xs) - Math.min(...xs)).toBeCloseTo(20);
    expect(Math.max(...ys) - Math.min(...ys)).toBeCloseTo(10);
    expect((Math.max(...xs) + Math.min(...xs)) / 2).toBeCloseTo(100);
  });
});
