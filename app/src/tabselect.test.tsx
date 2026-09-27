import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { App } from "./App";
import { useAppStore } from "./store";
import { installFakeBackend, type FakeBackend } from "./test/fakeBackend";

let fake: FakeBackend;

// The canvas needs a size to have a camera: a ResizeObserver that reports 800 x 600.
class SizedObserver {
  constructor(private cb: ResizeObserverCallback) {}
  observe() {
    this.cb([], this as unknown as ResizeObserver);
  }
  unobserve() {}
  disconnect() {}
}
const RealObserver = globalThis.ResizeObserver;
afterEach(() => {
  globalThis.ResizeObserver = RealObserver;
  vi.restoreAllMocks();
});

beforeEach(() => {
  useAppStore.setState({
    app: null,
    error: null,
    openViews: [],
    activeView: null,
    selection: [],
    tool: "select",
    hoverLabel: "",
  });
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
  fake = installFakeBackend();
  fake.underCursor = [
    { ids: ["w1"], label: 'Wall : Generic - 8"' },
    { ids: ["w1", "w2", "w3"], label: "Chain of walls (3)" },
    { ids: ["d1"], label: 'Door : 36" x 84"' },
  ];
});

async function openPlan() {
  const { container } = render(<App />);
  await userEvent.click(screen.getByRole("button", { name: "New Project" }));
  await screen.findByRole("toolbar", { name: "Tools" });
  await waitFor(() => expect(container.querySelector("canvas")).not.toBeNull());
  return container.querySelector("canvas")!;
}

const status = () => document.querySelector(".status-prompt")?.textContent ?? "";
const tab = (shift = false) => fireEvent.keyDown(window, { key: "Tab", shiftKey: shift });
const click = (canvas: HTMLCanvasElement) => {
  fireEvent.mouseDown(canvas, { button: 0, clientX: 100, clientY: 100 });
  fireEvent.mouseUp(canvas, { button: 0, clientX: 100, clientY: 100 });
};

describe("Tab selection (ADR-056)", () => {
  it("names what's under the cursor and steps through it with Tab", async () => {
    const canvas = await openPlan();
    fireEvent.mouseMove(canvas, { clientX: 100, clientY: 100 });
    await waitFor(() => expect(status()).toBe('Wall : Generic - 8"  (Tab for the next)'));
    tab();
    expect(status()).toBe("Chain of walls (3)  (2 of 3, Tab for the next)");
    tab();
    expect(status()).toContain('Door : 36" x 84"  (3 of 3');
    tab();
    expect(status()).toContain("Wall : Generic");
    // Shift+Tab steps back.
    tab(true);
    expect(status()).toContain("Door");
    tab(true);
    expect(status()).toContain("Chain of walls");
    // The click selects the candidate: the whole chain.
    click(canvas);
    await waitFor(() => expect(useAppStore.getState().selection).toEqual(["w1", "w2", "w3"]));
  });

  it("without Tab, a click selects the first; Shift adds Tab's candidate", async () => {
    const canvas = await openPlan();
    fireEvent.mouseMove(canvas, { clientX: 100, clientY: 100 });
    await waitFor(() => expect(status()).toContain("Wall"));
    click(canvas);
    await waitFor(() => expect(useAppStore.getState().selection).toEqual(["w1"]));
    fireEvent.mouseMove(canvas, { clientX: 101, clientY: 100 });
    await waitFor(() => expect(fake.calls.filter((c) => c.cmd === "pick_cycle").length).toBe(2));
    tab();
    tab();
    fireEvent.mouseDown(canvas, { button: 0, clientX: 101, clientY: 100, shiftKey: true });
    fireEvent.mouseUp(canvas, { button: 0, clientX: 101, clientY: 100, shiftKey: true });
    await waitFor(() => expect(useAppStore.getState().selection).toEqual(["w1", "d1"]));
  });

  it("Tab does nothing with one thing under the cursor", async () => {
    fake.underCursor = [{ ids: ["w1"], label: "Wall : Generic" }];
    const canvas = await openPlan();
    fireEvent.mouseMove(canvas, { clientX: 100, clientY: 100 });
    await waitFor(() => expect(status()).toBe("Wall : Generic"));
    tab();
    expect(status()).toBe("Wall : Generic");
  });
});
