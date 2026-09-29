import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { App } from "./App";
import { useAppStore } from "./store";
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
    ribbonTab: "Architecture",
    structuralOverlay: false,
    structuralAlpha: 0.85,
  });
  fake = installFakeBackend();
});
afterEach(() => {
  globalThis.ResizeObserver = RealObserver;
  vi.restoreAllMocks();
});

const argsOf = (cmd: string) =>
  fake.calls.filter((c) => c.cmd === cmd).map((c) => c.args as Record<string, unknown>);

async function start(sized = false) {
  if (sized) {
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
  }
  const r = render(<App />);
  await userEvent.click(screen.getByRole("button", { name: "New Project" }));
  await screen.findByRole("toolbar", { name: "Tools" });
  await userEvent.click(screen.getByRole("tab", { name: "Structure" }));
  return r;
}

describe("Suggest Structure (ADR-080)", () => {
  it("proposes the top three schemes, labelled preliminary, with questions and red flags", async () => {
    await start();
    expect(screen.getByRole("button", { name: /^Overlay/ })).toBeDisabled();
    await userEvent.click(screen.getByRole("button", { name: /Suggest Structure/ }));
    const dialog = await screen.findByRole("dialog", { name: "Suggest Structure" });
    expect(within(dialog).getByRole("note").textContent).toContain(
      "Preliminary — not engineered. Requires review by a licensed structural engineer.",
    );
    await within(dialog).findByRole("region", { name: "Light Wood Frame + Wood Shear Walls" });
    expect(within(dialog).getAllByRole("region")).toHaveLength(3);
    expect(within(dialog).getByText(/Transfer beams likely at Level 2/)).toBeInTheDocument();
    expect(within(dialog).getByText(/seismic region\?/)).toBeInTheDocument();
    await userEvent.click(within(dialog).getByRole("button", { name: /Show all 4/ }));
    expect(within(dialog).getAllByRole("region")).toHaveLength(4);
    await userEvent.selectOptions(within(dialog).getByLabelText("Seismic region"), "High");
    await waitFor(() => expect(argsOf("structural_suggest").at(-1)).toEqual({ seismic: "High" }));
  });

  it("generates the overlay for the chosen scheme with adjusted settings", async () => {
    await start();
    await userEvent.click(screen.getByRole("button", { name: /Suggest Structure/ }));
    const dialog = await screen.findByRole("dialog", { name: "Suggest Structure" });
    const card = await within(dialog).findByRole("region", {
      name: "Light Wood Frame + Wood Shear Walls",
    });
    const grid = within(card).getByLabelText("Grid east–west");
    await userEvent.clear(grid);
    await userEvent.type(grid, "20");
    await userEvent.selectOptions(within(card).getByLabelText("Joist direction"), "X");
    await userEvent.click(within(card).getByRole("button", { name: "Generate overlay" }));
    await waitFor(() => expect(argsOf("structural_generate")).toHaveLength(1));
    const settings = argsOf("structural_generate")[0]!.settings as Record<string, unknown>;
    expect(settings).toMatchObject({ kind: "LightWood", spanDir: "X" });
    expect(settings.gridX as number).toBeCloseTo(20 * 304.8);
    await waitFor(() => expect(useAppStore.getState().structuralOverlay).toBe(true));
    expect(screen.queryByRole("dialog", { name: "Suggest Structure" })).toBeNull();
    expect(screen.getByRole("button", { name: /^Overlay/ })).toBeEnabled();
  });

  it("shows the overlay over the greyed plan, explains a member, and exports", async () => {
    const { container } = await start(true);
    fake.state = { ...fake.state!, structuralLayer: "layer-1" };
    useAppStore.getState().setApp(fake.state);
    await userEvent.click(screen.getByRole("button", { name: /^Overlay/ }));
    await screen.findByLabelText("Structural overlay legend");
    await waitFor(() => expect(argsOf("structural_overlay_2d").length).toBeGreaterThan(0));
    const canvas = container.querySelector("canvas")!;
    fireEvent.mouseDown(canvas, { button: 0, clientX: 400, clientY: 300 });
    fireEvent.mouseUp(canvas, { button: 0, clientX: 400, clientY: 300 });
    const card = await screen.findByRole("dialog", { name: "Column — W10x49 (prelim.)" });
    expect(within(card).getByText(/At a grid intersection/)).toBeInTheDocument();
    // Selection isn't changed by explaining a member.
    expect(useAppStore.getState().selection).toEqual([]);
    fake.savePath = "C:/out/Structure.json";
    await userEvent.click(screen.getByRole("button", { name: "Export JSON" }));
    await waitFor(() =>
      expect(argsOf("structural_export_json")[0]).toEqual({ path: "C:/out/Structure.json" }),
    );
    fake.savePath = "C:/out/Structure.ifc";
    await userEvent.click(screen.getByRole("button", { name: "Export IFC" }));
    await waitFor(() =>
      expect(argsOf("structural_export_ifc")[0]).toEqual({ path: "C:/out/Structure.ifc" }),
    );
  });
});
