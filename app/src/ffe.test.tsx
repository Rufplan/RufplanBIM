import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { App } from "./App";
import { useAppStore } from "./store";
import { installFakeBackend, type FakeBackend } from "./test/fakeBackend";
import { ffeLength, ffeSpecs } from "./components/FfePicker";

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

async function openFfe() {
  render(<App />);
  await userEvent.click(screen.getByRole("button", { name: "New Project" }));
  await screen.findByRole("toolbar", { name: "Tools" });
  await userEvent.click(screen.getByRole("tab", { name: "FFE" }));
}

describe("FFE tab (ADR-090)", () => {
  it("has Furniture, then the lighting tools, then Equipment", async () => {
    await openFfe();
    const names = screen
      .getAllByRole("button")
      .map((b) => b.textContent ?? "")
      .filter((t) =>
        ["Furniture", "Lighting Fixture", "Equipment", "Sun Settings", "Render"].includes(t),
      );
    expect(names).toEqual(["Furniture", "Lighting Fixture", "Equipment", "Sun Settings", "Render"]);
    for (const name of [/Load Furniture/, /Load Fixtures/, /Load Equipment/, /Artificial Lights/])
      expect(screen.getByRole("button", { name })).toBeInTheDocument();
  });

  it("Load Furniture filters the library by group and project type, and loads to place", async () => {
    await openFfe();
    await userEvent.click(screen.getByRole("button", { name: /Load Furniture/ }));
    const dialog = await screen.findByRole("dialog", { name: "Furniture Types" });
    const lib = await within(dialog).findByRole("list", { name: "Library furniture" });
    await waitFor(() => expect(within(lib).getAllByRole("listitem")).toHaveLength(3));
    await userEvent.click(within(dialog).getByRole("button", { name: "Bedroom" }));
    expect(
      within(lib)
        .getAllByRole("listitem")
        .map((i) => i.textContent),
    ).toEqual(["King Bed"]);
    await userEvent.click(within(dialog).getByRole("button", { name: "All Furniture" }));
    await userEvent.selectOptions(within(dialog).getByLabelText("Project type"), "Hospitality");
    expect(within(lib).getAllByRole("listitem")).toHaveLength(3);
    await userEvent.selectOptions(within(dialog).getByLabelText("Project type"), "Multifamily");
    expect(within(lib).getAllByRole("listitem")).toHaveLength(1);
    await userEvent.click(within(lib).getByRole("listitem", { name: /Sofa/ }));
    expect(within(dialog).getByRole("table", { name: "Size" }).textContent).toContain("7'-0\"");
    await userEvent.click(within(dialog).getByRole("button", { name: "Load & Place" }));
    await waitFor(() =>
      expect(argsOf("load_ffe_types").at(-1)).toMatchObject({ names: ['Sofa 84"'] }),
    );
    expect(useAppStore.getState().tool).toBe("furniture");
    expect(useAppStore.getState().toolTypes.furniture).toBe("00000000-0000-7000-8000-000000000092");
  });

  it("Equipment opens its own library when the project has no equipment", async () => {
    await openFfe();
    await userEvent.click(screen.getByRole("button", { name: "Equipment" }));
    const dialog = await screen.findByRole("dialog", { name: "Equipment Types" });
    expect(useAppStore.getState().tool).toBe("equipment");
    const lib = await within(dialog).findByRole("list", { name: "Library equipment" });
    await waitFor(() => expect(within(lib).getAllByRole("listitem")).toHaveLength(2));
    expect(argsOf("ffe_library").at(-1)).toMatchObject({ class: "Equipment" });
  });

  it("clicking a plan places the piece with the options bar's rotation", async () => {
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
    const canvas = container.querySelector("canvas")!;
    useAppStore.getState().setToolType("furniture", "00000000-0000-7000-8000-000000000091");
    useAppStore.getState().setTool("furniture");
    await userEvent.click(await screen.findByRole("button", { name: "Rotate 90°" }));
    fireEvent.mouseDown(canvas, { button: 0, clientX: 300, clientY: 200 });
    fireEvent.mouseUp(canvas, { button: 0, clientX: 300, clientY: 200 });
    await waitFor(() => expect(argsOf("create_ffe")).toHaveLength(1));
    expect(argsOf("create_ffe")[0]).toMatchObject({
      typeId: "00000000-0000-7000-8000-000000000091",
    });
    expect(argsOf("create_ffe")[0]!.rotation).toBeCloseTo(Math.PI / 2);
  });

  it("sizes read in feet-inches", () => {
    expect(ffeLength(84 * 25.4)).toBe("7'-0\"");
    expect(ffeLength(30 * 25.4)).toBe('30"');
    expect(ffeLength(60.5 * 25.4)).toBe("5'-0.5\"");
    const rows = ffeSpecs({
      class: "Equipment",
      kind: "Tv",
      width: 57 * 25.4,
      depth: 2 * 25.4,
      height: 33 * 25.4,
      mount: "Wall",
      mount_height: 40 * 25.4,
      count: 0,
      color: [0, 0, 0],
      accent: [0, 0, 0],
    });
    expect(rows.at(-1)).toEqual(["Mounting height", '40"']);
  });
});
