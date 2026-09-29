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
    mepOverlay: [],
    mepDiscipline: "Mechanical",
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
  await userEvent.click(screen.getByRole("tab", { name: "MEPT" }));
  return r.container.querySelector("canvas");
}

describe("MEPT tab (ADR-082)", () => {
  it("sits after Structure with Mechanical, Electrical, Plumbing and Technology apart", async () => {
    await start();
    const tabs = within(screen.getByRole("tablist", { name: "Ribbon tabs" }))
      .getAllByRole("tab")
      .map((t) => t.textContent);
    expect(tabs.indexOf("MEPT")).toBe(tabs.indexOf("Structure") + 1);
    const toolbar = screen.getByRole("toolbar", { name: "Tools" });
    expect(within(toolbar).getAllByRole("button", { name: /Suggest/ })).toHaveLength(4);
    for (const d of ["Mechanical", "Electrical", "Plumbing", "Technology"])
      expect(within(toolbar).getByRole("button", { name: `${d} overlay` })).toBeDisabled();
  });

  it("suggests systems per discipline, the climate only for mechanical", async () => {
    await start();
    const toolbar = screen.getByRole("toolbar", { name: "Tools" });
    await userEvent.click(within(toolbar).getAllByRole("button", { name: /Suggest/ })[0]!);
    const dialog = await screen.findByRole("dialog", { name: "Suggest Mechanical" });
    await within(dialog).findByRole("region", { name: "Mechanical System A" });
    expect(within(dialog).getByRole("note").textContent).toContain("licensed mechanical engineer");
    expect(within(dialog).getAllByRole("region")).toHaveLength(3);
    await userEvent.selectOptions(within(dialog).getByLabelText("Climate"), "Cold");
    await waitFor(() =>
      expect(argsOf("mep_suggest").at(-1)).toEqual({ discipline: "Mechanical", climate: "Cold" }),
    );
    await userEvent.click(within(dialog).getByRole("tab", { name: "Plumbing" }));
    const plumbing = await screen.findByRole("dialog", { name: "Suggest Plumbing" });
    await within(plumbing).findByRole("region", { name: "Plumbing System A" });
    expect(within(plumbing).queryByLabelText("Climate")).toBeNull();
    expect(within(plumbing).getByText("A red flag.")).toBeInTheDocument();
  });

  it("generates a discipline's overlay, shows it over the plan and explains an item", async () => {
    const canvas = (await start(true))!;
    const toolbar = screen.getByRole("toolbar", { name: "Tools" });
    await userEvent.click(within(toolbar).getAllByRole("button", { name: /Suggest/ })[1]!);
    const dialog = await screen.findByRole("dialog", { name: "Suggest Electrical" });
    const card = await within(dialog).findByRole("region", { name: "Electrical System A" });
    await userEvent.click(within(card).getByRole("button", { name: "Generate overlay" }));
    await waitFor(() =>
      expect(argsOf("mep_generate")[0]).toEqual({
        settings: { discipline: "Electrical", system: "a", climate: "Mixed" },
      }),
    );
    await waitFor(() => expect(useAppStore.getState().mepOverlay).toEqual(["Electrical"]));
    expect(within(toolbar).getByRole("button", { name: "Electrical overlay" })).toBeEnabled();
    await screen.findByLabelText("MEPT overlay legend");
    expect(argsOf("mep_overlay_2d").at(-1)).toMatchObject({ disciplines: ["Electrical"] });
    fireEvent.mouseDown(canvas, { button: 0, clientX: 400, clientY: 300 });
    fireEvent.mouseUp(canvas, { button: 0, clientX: 400, clientY: 300 });
    await screen.findByRole("dialog", { name: "Supply Diffuser — 150 cfm diffuser (prelim.)" });
    fake.savePath = "C:/out/Electrical.json";
    await userEvent.click(within(toolbar).getByRole("button", { name: "Export Electrical JSON" }));
    await waitFor(() =>
      expect(argsOf("mep_export_json")[0]).toEqual({
        discipline: "Electrical",
        path: "C:/out/Electrical.json",
      }),
    );
    // Turning it off hides the legend.
    await userEvent.click(within(toolbar).getByRole("button", { name: "Electrical overlay" }));
    await waitFor(() => expect(screen.queryByLabelText("MEPT overlay legend")).toBeNull());
  });
});
