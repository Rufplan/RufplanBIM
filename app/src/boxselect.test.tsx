import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { App } from "./App";
import { useAppStore } from "./store";
import { installFakeBackend, type FakeBackend } from "./test/fakeBackend";
import { categoryLabel } from "./components/ModifyContext";

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
  return container.querySelector("canvas")!;
}

const dragBox = (canvas: HTMLCanvasElement, from: [number, number], to: [number, number]) => {
  fireEvent.mouseDown(canvas, { button: 0, clientX: from[0], clientY: from[1] });
  fireEvent.mouseMove(canvas, { button: 0, clientX: (from[0] + to[0]) / 2, clientY: to[1] });
  fireEvent.mouseMove(canvas, { button: 0, clientX: to[0], clientY: to[1] });
  fireEvent.mouseUp(canvas, { button: 0, clientX: to[0], clientY: to[1] });
};

describe("Revit's box selection and Filter", () => {
  it("left to right is a window, right to left a crossing; Ctrl adds, Shift takes away", async () => {
    const canvas = await openPlan();
    dragBox(canvas, [100, 100], [400, 300]);
    await waitFor(() => expect(argsOf("pick_in_rect")).toHaveLength(1));
    expect(argsOf("pick_in_rect")[0]).toMatchObject({ crossing: false });
    await waitFor(() => expect(useAppStore.getState().selection).toEqual(["w1", "w2", "d1"]));
    dragBox(canvas, [400, 300], [100, 100]);
    await waitFor(() => expect(argsOf("pick_in_rect")).toHaveLength(2));
    expect(argsOf("pick_in_rect")[1]).toMatchObject({ crossing: true });
    useAppStore.getState().select(["w3"]);
    fireEvent.mouseDown(canvas, { button: 0, clientX: 100, clientY: 100 });
    fireEvent.mouseMove(canvas, { button: 0, clientX: 300, clientY: 300 });
    fireEvent.mouseUp(canvas, { button: 0, clientX: 300, clientY: 300, ctrlKey: true });
    await waitFor(() => expect(useAppStore.getState().selection).toEqual(["w3", "w1", "w2", "d1"]));
    fireEvent.mouseDown(canvas, { button: 0, clientX: 100, clientY: 100 });
    fireEvent.mouseMove(canvas, { button: 0, clientX: 300, clientY: 300 });
    fireEvent.mouseUp(canvas, { button: 0, clientX: 300, clientY: 300, shiftKey: true });
    await waitFor(() => expect(useAppStore.getState().selection).toEqual(["w3"]));
  });

  it("the Filter dialog keeps only the categories left checked", async () => {
    const canvas = await openPlan();
    dragBox(canvas, [100, 100], [400, 300]);
    await waitFor(() => expect(useAppStore.getState().selection).toHaveLength(3));
    // The status bar's funnel shows the count and opens the Filter.
    await userEvent.click(
      screen.getByRole("button", { name: "Filter the selection (3 selected)" }),
    );
    const dialog = await screen.findByRole("dialog", { name: "Filter" });
    const rows = await within(dialog).findAllByRole("listitem");
    expect(rows.map((r) => r.textContent)).toEqual(["Doors1", "Walls2"]);
    expect(within(dialog).getByLabelText("Total selected items")).toHaveTextContent("3");
    await userEvent.click(within(rows[0]!).getByRole("checkbox"));
    expect(within(dialog).getByLabelText("Total selected items")).toHaveTextContent("2");
    await userEvent.click(within(dialog).getByRole("button", { name: "Check None" }));
    expect(within(dialog).getByLabelText("Total selected items")).toHaveTextContent("0");
    await userEvent.click(within(dialog).getByRole("button", { name: "Check All" }));
    await userEvent.click(within(rows[0]!).getByRole("checkbox"));
    await userEvent.click(within(dialog).getByRole("button", { name: "OK" }));
    expect(useAppStore.getState().selection).toEqual(["w1", "w2"]);
    expect(screen.queryByRole("dialog", { name: "Filter" })).toBeNull();
    expect(categoryLabel("GroundRegion")).toBe("Ground Regions");
    expect(categoryLabel("SpotElevation")).toBe("Spot Elevations");
  });
});
