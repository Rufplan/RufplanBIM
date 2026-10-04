import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { App } from "./App";
import { useAppStore } from "./store";
import { installFakeBackend, type FakeBackend } from "./test/fakeBackend";
import { leaderClicks, leadersFrom, textPrompt } from "./text";

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
  useAppStore.setState({ options: { ...o, textLeader: "None", textAlign: "Left", textSize: 2.4 } });
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

const clickAt = (c: HTMLCanvasElement, x: number, y: number) => {
  fireEvent.mouseDown(c, { button: 0, clientX: x, clientY: y });
  fireEvent.mouseUp(c, { button: 0, clientX: x, clientY: y });
};

describe("Revit's Text (ADR-070)", () => {
  it("places a note by typing in place, not in a pop-up", async () => {
    const prompt = vi.spyOn(window, "prompt");
    const canvas = await openPlan();
    useAppStore.getState().setTool("text");
    expect(await screen.findByRole("tab", { name: "Modify | Place Text" })).toBeTruthy();
    // The tab bar stays, as in Revit (ADR-107): the Modify tab takes the contextual name.
    expect(screen.getByRole("tab", { name: "Architecture" })).toBeTruthy();
    expect(screen.getByLabelText("Text Type")).toHaveValue("2.4");
    clickAt(canvas, 300, 200);
    const editor = await screen.findByRole("textbox", { name: "Text" });
    expect(await screen.findByRole("tab", { name: "Modify | Edit Text" })).toBeTruthy();
    await userEvent.type(editor, "TYPICAL{Enter}AT ALL WINDOWS");
    fireEvent.blur(editor);
    await waitFor(() => expect(argsOf("create_text_note")).toHaveLength(1));
    expect(argsOf("create_text_note")[0]).toMatchObject({
      text: "TYPICAL\nAT ALL WINDOWS",
      size: 2.4,
      leaders: [],
      align: "Left",
      width: null,
    });
    expect(prompt).not.toHaveBeenCalled();
    // An empty note is dropped, as in Revit.
    clickAt(canvas, 300, 260);
    const again = await screen.findByRole("textbox", { name: "Text" });
    fireEvent.keyDown(again, { key: "Escape" });
    await waitFor(() => expect(screen.queryByRole("textbox", { name: "Text" })).toBeNull());
    expect(argsOf("create_text_note")).toHaveLength(1);
  });

  it("takes the leader's clicks first: one segment, two segments, curved", async () => {
    const canvas = await openPlan();
    useAppStore.getState().setTool("text");
    await screen.findByRole("tab", { name: "Modify | Place Text" });
    const leader = screen.getByRole("radiogroup", { name: "Leader" });
    await userEvent.click(within(leader).getByRole("radio", { name: "Two Segments" }));
    await userEvent.click(screen.getByRole("radio", { name: "Align Right" }));
    await userEvent.selectOptions(screen.getByLabelText("Text Type"), "3");
    clickAt(canvas, 100, 100);
    clickAt(canvas, 150, 150);
    expect(screen.queryByRole("textbox", { name: "Text" })).toBeNull();
    clickAt(canvas, 250, 150);
    const editor = await screen.findByRole("textbox", { name: "Text" });
    await userEvent.type(editor, "SEALANT");
    fireEvent.keyDown(editor, { key: "Enter", ctrlKey: true });
    await waitFor(() => expect(argsOf("create_text_note")).toHaveLength(1));
    const args = argsOf("create_text_note")[0]!;
    expect(args).toMatchObject({ text: "SEALANT", size: 3, align: "Right" });
    const leaders = args.leaders as { end: unknown; elbow: unknown; arc: boolean }[];
    expect(leaders).toHaveLength(1);
    expect(leaders[0]!.elbow).not.toBeNull();
    expect(leaders[0]!.arc).toBe(false);
    // The helpers: clicks each leader needs, and the leaders from them.
    expect([leaderClicks("None"), leaderClicks("One"), leaderClicks("Two")]).toEqual([0, 1, 2]);
    const p = { x: 1, y: 2 };
    expect(leadersFrom("Curved", [p])).toEqual([{ end: p, elbow: null, arc: true }]);
    expect(textPrompt("One", 0)).toContain("arrowhead");
  });

  it("dragging sets the text box's width", async () => {
    const canvas = await openPlan();
    useAppStore.getState().setTool("text");
    await screen.findByRole("tab", { name: "Modify | Place Text" });
    fireEvent.mouseDown(canvas, { button: 0, clientX: 100, clientY: 100 });
    fireEvent.mouseMove(canvas, { button: 0, clientX: 200, clientY: 120 });
    fireEvent.mouseMove(canvas, { button: 0, clientX: 300, clientY: 140 });
    fireEvent.mouseUp(canvas, { button: 0, clientX: 300, clientY: 140 });
    const editor = await screen.findByRole("textbox", { name: "Text" });
    await userEvent.type(editor, "A LONG NOTE THAT WRAPS");
    fireEvent.blur(editor);
    await waitFor(() => expect(argsOf("create_text_note")).toHaveLength(1));
    expect(argsOf("create_text_note")[0]!.width).toBeGreaterThan(0);
  });

  it("text notes get leader buttons, and Edit Text edits in place", async () => {
    await openPlan();
    fake.properties = { t1: { category: "TextNote" } } as typeof fake.properties;
    useAppStore.getState().select(["t1"]);
    await userEvent.click(await screen.findByRole("button", { name: /Add Left Leader/ }));
    expect(argsOf("add_text_leader")[0]).toEqual({ ids: ["t1"], left: true });
    await userEvent.click(screen.getByRole("button", { name: /Remove Last Leader/ }));
    expect(argsOf("remove_text_leader")).toHaveLength(1);
    await userEvent.click(screen.getByRole("button", { name: /Edit Text/ }));
    const editor = await screen.findByRole("textbox", { name: "Text" });
    expect(editor).toHaveValue("EXISTING NOTE");
    await userEvent.type(editor, " TYP.");
    fireEvent.blur(editor);
    await waitFor(() =>
      expect(argsOf("set_property").at(-1)).toEqual({
        id: "t1",
        key: "text",
        value: "EXISTING NOTE TYP.",
      }),
    );
  });
});
