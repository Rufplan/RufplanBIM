import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { App } from "./App";
import { useAppStore } from "./store";
import { conflicts, DEFAULT_SHORTCUTS } from "./shortcuts";
import { shortcut, toolAllowed } from "./tools";
import { installFakeBackend, type FakeBackend } from "./test/fakeBackend";

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
    ribbonTab: "Details",
  });
  const o = useAppStore.getState().options;
  useAppStore.setState({
    options: { ...o, componentKey: "lum-2x6", componentRotation: 0, componentFlip: false },
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
  await userEvent.click(screen.getByRole("tab", { name: "Details" }));
  await waitFor(() => expect(container.querySelector("canvas")).not.toBeNull());
  return container.querySelector("canvas")!;
}

const clickAt = (c: HTMLCanvasElement, x: number, y: number) => {
  fireEvent.mouseDown(c, { button: 0, clientX: x, clientY: y });
  fireEvent.mouseUp(c, { button: 0, clientX: x, clientY: y });
};

describe("Detail components (ADR-071)", () => {
  it("places a point-based component at a click, turned by Space", async () => {
    const canvas = await openPlan();
    await userEvent.click(await screen.findByRole("button", { name: /Detail Component/ }));
    expect(useAppStore.getState().tool).toBe("component");
    const type = await screen.findByLabelText("Component Type");
    await waitFor(() => expect(type).toHaveValue("lum-2x6"));
    fireEvent.keyDown(window, { key: " " });
    expect(useAppStore.getState().options.componentRotation).toBe(90);
    clickAt(canvas, 300, 300);
    await waitFor(() => expect(argsOf("create_detail_component")).toHaveLength(1));
    const a = argsOf("create_detail_component")[0]!;
    expect(a.key).toBe("lum-2x6");
    const s = a.start as { x: number; y: number };
    const e = a.end as { x: number; y: number };
    // Turned 90°: its direction points up.
    expect(Math.abs(e.x - s.x)).toBeLessThan(1e-6);
    expect(e.y - s.y).toBeCloseTo(100);
  });

  it("Insulation and Repeating Detail are line-based: start, then end", async () => {
    const canvas = await openPlan();
    await userEvent.click(await screen.findByRole("button", { name: /Insulation/ }));
    expect(useAppStore.getState().options.componentKey).toBe("batt-55");
    clickAt(canvas, 100, 300);
    expect(argsOf("create_detail_component")).toHaveLength(0);
    clickAt(canvas, 500, 300);
    await waitFor(() => expect(argsOf("create_detail_component")).toHaveLength(1));
    expect(argsOf("create_detail_component")[0]).toMatchObject({ key: "batt-55", flip: false });
    await userEvent.click(screen.getByRole("button", { name: /Repeating Detail/ }));
    expect(useAppStore.getState().options.componentKey).toBe("brick-mod");
    // CM is Revit's Component shortcut; detail components go in 2D views only.
    expect(shortcut("C", "M").tool).toBe("component");
    expect(conflicts(DEFAULT_SHORTCUTS)).toEqual([]);
    expect(toolAllowed("component", "Drafting")).toBe(true);
    expect(toolAllowed("component", "Section")).toBe(true);
    expect(toolAllowed("component", "ThreeD")).toBe(false);
  });
});
