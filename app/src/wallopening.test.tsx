import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { App } from "./App";
import { useAppStore } from "./store";
import { FAKE_IDS, installFakeBackend, type FakeBackend } from "./test/fakeBackend";
import { toolAllowed } from "./tools";

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

beforeEach(() => {
  useAppStore.setState({
    app: null,
    error: null,
    openViews: [],
    activeView: null,
    selection: [],
    tool: "select",
    ribbonTab: "Architecture",
  });
  fake = installFakeBackend();
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
});
afterEach(() => {
  globalThis.ResizeObserver = RealObserver;
  vi.restoreAllMocks();
});

async function openElevation() {
  const { container } = render(<App />);
  await userEvent.click(screen.getByRole("button", { name: "New Project" }));
  await screen.findByRole("toolbar", { name: "Tools" });
  act(() => useAppStore.getState().openView(FAKE_IDS.north));
  await waitFor(() => expect(container.querySelector("canvas")).not.toBeNull());
  return container.querySelector("canvas")!;
}

const argsOf = (cmd: string) =>
  fake.calls.filter((c) => c.cmd === cmd).map((c) => c.args as Record<string, unknown>);

describe("Wall Opening (ADR-058)", () => {
  it("is on the Openings tab, for elevations, sections and 3D", () => {
    expect(toolAllowed("wallOpening", "Elevation")).toBe(true);
    expect(toolAllowed("wallOpening", "Section")).toBe(true);
    expect(toolAllowed("wallOpening", "ThreeD")).toBe(true);
    expect(toolAllowed("wallOpening", "Plan")).toBe(false);
  });

  it("clicking a wall in an elevation starts the sketch on its face", async () => {
    fake.underCursor = [{ ids: ["w1"], label: "Wall : Generic" }];
    const canvas = await openElevation();
    await userEvent.click(screen.getByRole("tab", { name: "Openings" }));
    await userEvent.click(screen.getByRole("button", { name: "Wall Opening" }));
    expect(useAppStore.getState().tool).toBe("wallOpening");
    fireEvent.mouseDown(canvas, { button: 0, clientX: 400, clientY: 300 });
    fireEvent.mouseUp(canvas, { button: 0, clientX: 400, clientY: 300 });
    await waitFor(() => expect(argsOf("sketch_begin")).toHaveLength(1));
    expect(argsOf("sketch_begin")[0]).toMatchObject({
      view: FAKE_IDS.north,
      kind: "WallOpening",
      target: null,
      host: "w1",
    });
    // Revit's sketch tab for it: the draw tools, starting with Rectangle, but no Pick
    // Walls, Pick Lines or Flip.
    expect(
      await screen.findByRole("tab", { name: "Modify | Create Wall Opening Sketch" }),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Rectangle" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    for (const name of ["Circle", "Inscribed Polygon", "Start-End-Radius Arc", "Line"])
      expect(screen.getByRole("button", { name })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Pick Walls" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Pick Lines" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Flip" })).toBeNull();
    // The sketch works in the elevation, where floor sketches don't.
    expect(toolAllowed("sketch", "Elevation")).toBe(true);
  });

  it("clicking something other than a wall explains", async () => {
    fake.underCursor = [{ ids: ["d1"], label: "Door" }];
    const canvas = await openElevation();
    act(() => useAppStore.getState().setTool("wallOpening"));
    fireEvent.mouseDown(canvas, { button: 0, clientX: 400, clientY: 300 });
    fireEvent.mouseUp(canvas, { button: 0, clientX: 400, clientY: 300 });
    await waitFor(() =>
      expect(useAppStore.getState().error).toBe("Click a wall to cut the opening in."),
    );
    expect(argsOf("sketch_begin")).toHaveLength(0);
  });

  it("a selected opening's contextual tab has Edit Sketch", async () => {
    await openElevation();
    fake.properties = { o1: { category: "WallOpening" } };
    act(() => useAppStore.getState().select(["o1"]));
    await screen.findByRole("tab", { name: "Modify | Wall Openings" });
    await userEvent.click(screen.getByRole("button", { name: "Edit Sketch" }));
    await waitFor(() => expect(argsOf("sketch_begin")).toHaveLength(1));
    expect(argsOf("sketch_begin")[0]).toMatchObject({ kind: "WallOpening", target: "o1" });
  });

  it("lighting fixtures go in from an elevation too (ADR-059)", async () => {
    const canvas = await openElevation();
    act(() => {
      useAppStore.getState().setToolType("light", "00000000-0000-7000-8000-000000000043");
      useAppStore.getState().setTool("light");
    });
    fireEvent.mouseDown(canvas, { button: 0, clientX: 400, clientY: 300 });
    fireEvent.mouseUp(canvas, { button: 0, clientX: 400, clientY: 300 });
    await waitFor(() => expect(argsOf("create_lighting_fixture")).toHaveLength(1));
    // The elevation works out the level and height from the click.
    expect(argsOf("create_lighting_fixture")[0]).toMatchObject({
      view: FAKE_IDS.north,
      level: null,
      elevation: null,
    });
  });
});
