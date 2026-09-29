import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { App } from "./App";
import { activeViewInfo, useAppStore } from "./store";
import { toolAllowed } from "./tools";
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
  });
  const o = useAppStore.getState().options;
  useAppStore.setState({ options: { ...o, refOther: false, refTarget: "" } });
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

/** A click, then a moment for the tool to take the point (snapping is async). */
const clickAt = async (c: HTMLCanvasElement, x: number, y: number) => {
  fireEvent.mouseDown(c, { button: 0, clientX: x, clientY: y });
  fireEvent.mouseUp(c, { button: 0, clientX: x, clientY: y });
  await new Promise((r) => setTimeout(r, 20));
};

/** Adds a drafting view (a typical detail) to the fake project. */
function addDetail(name: string) {
  const s = fake.state!;
  fake.state = {
    ...s,
    views: [...s.views, { ...s.views[0]!, id: "draft-1", name, viewType: "Drafting", level: null }],
  };
  useAppStore.getState().setApp(fake.state);
}

describe("Reference Other View (ADR-076)", () => {
  it("a section references a typical detail picked on the options bar", async () => {
    const canvas = await openPlan();
    addDetail("Typ. Eave");
    useAppStore.getState().setTool("section");
    const box = await screen.findByRole("checkbox", { name: "Reference Other View" });
    const list = screen.getByRole("combobox", { name: "Referenced View" });
    expect(list).toBeDisabled();
    await userEvent.click(box);
    await screen.findByRole("option", { name: "Drafting View: Typ. Eave" });
    await userEvent.selectOptions(list, "draft-1");
    await clickAt(canvas, 100, 300);
    await clickAt(canvas, 500, 300);
    await waitFor(() => expect(argsOf("create_reference")).toHaveLength(1));
    const call = argsOf("create_reference")[0]!;
    expect(call.target).toBe("draft-1");
    expect(call.shape).toHaveProperty("Section");
    expect(argsOf("create_section")).toHaveLength(0);
  });

  it("unchecked, Section makes a new section view as before; <New drafting view> sends none", async () => {
    const canvas = await openPlan();
    useAppStore.getState().setTool("section");
    await screen.findByRole("checkbox", { name: "Reference Other View" });
    await clickAt(canvas, 100, 300);
    await clickAt(canvas, 500, 300);
    await waitFor(() => expect(argsOf("create_section")).toHaveLength(1));

    useAppStore.getState().setTool("callout");
    await userEvent.click(await screen.findByRole("checkbox", { name: "Reference Other View" }));
    await clickAt(canvas, 100, 100);
    await clickAt(canvas, 300, 250);
    await waitFor(() => expect(argsOf("create_reference")).toHaveLength(1));
    expect(argsOf("create_reference")[0]).toMatchObject({ target: null });
    expect(argsOf("create_reference")[0]!.shape).toHaveProperty("Callout");
  });

  it("drafting views allow sections and callouts, always as references", () => {
    expect(toolAllowed("section", "Drafting")).toBe(true);
    expect(toolAllowed("callout", "Drafting")).toBe(true);
  });

  it("double-clicking a reference's head opens the view it points at", async () => {
    const canvas = await openPlan();
    addDetail("Typ. Eave");
    fake.underCursor = [{ ids: ["ref-1"], label: "Reference Section" }];
    fireEvent.doubleClick(canvas, { clientX: 200, clientY: 200 });
    await waitFor(() => expect(activeViewInfo(useAppStore.getState())?.name).toBe("Typ. Eave"));
  });
});
