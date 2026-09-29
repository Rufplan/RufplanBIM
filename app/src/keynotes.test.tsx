import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { App } from "./App";
import { useAppStore } from "./store";
import { toolAllowed } from "./tools";
import { keynoteTree, visibleKeys } from "./keynotes";
import { FAKE_KEYNOTES, installFakeBackend, type FakeBackend } from "./test/fakeBackend";

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
  const o = useAppStore.getState().options;
  useAppStore.setState({
    options: { ...o, keynoteStyle: "Key", keynoteLeader: true, keynoteUserKey: "" },
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
  await userEvent.click(screen.getByRole("tab", { name: "Annotate" }));
  return r.container.querySelector("canvas");
}

const clickAt = async (c: Element, x: number, y: number) => {
  fireEvent.mouseDown(c, { button: 0, clientX: x, clientY: y });
  fireEvent.mouseUp(c, { button: 0, clientX: x, clientY: y });
  await new Promise((r) => setTimeout(r, 20));
};

describe("keynote tree (ADR-081)", () => {
  it("nests by parent and searches every word, keeping ancestors", () => {
    const tree = keynoteTree(FAKE_KEYNOTES);
    expect(tree.map((n) => n.k.key)).toEqual(["03", "09"]);
    expect(tree[1]!.children[0]!.children.map((n) => n.k.key)).toEqual([
      "09 29 00.A1",
      "09 29 00.A2",
    ]);
    const hits = visibleKeys(FAKE_KEYNOTES, "type x gyp")!;
    expect([...hits].sort()).toEqual(["09", "09 29 00", "09 29 00.A2"]);
    expect(toolAllowed("keynoteUser", "Drafting")).toBe(true);
    expect(toolAllowed("keynoteElement", "Drafting")).toBe(false);
  });
});

describe("Keynote Manager (ADR-081)", () => {
  it("edits, renumbers and deletes keynotes", async () => {
    await start();
    await userEvent.click(screen.getByRole("button", { name: /Manager/ }));
    const dialog = await screen.findByRole("dialog", { name: "Keynote Manager" });
    const tree = await within(dialog).findByRole("tree", { name: "Keynotes" });
    await userEvent.click(within(tree).getByRole("treeitem", { name: "09 Finishes" }));
    await userEvent.click(within(tree).getByRole("treeitem", { name: "09 29 00 Gypsum Board" }));
    await userEvent.click(
      within(tree).getByRole("treeitem", { name: '09 29 00.A1 1/2" gypsum board' }),
    );
    const text = within(dialog).getByLabelText("Text");
    await userEvent.clear(text);
    await userEvent.type(text, "1/2 in. gypsum board, level 4 finish");
    await userEvent.click(within(dialog).getByRole("button", { name: "Save Changes" }));
    expect(argsOf("keynote_save")[0]).toEqual({
      oldKey: "09 29 00.A1",
      entry: {
        key: "09 29 00.A1",
        text: "1/2 in. gypsum board, level 4 finish",
        parent: "09 29 00",
      },
    });
    // Search narrows the tree.
    await userEvent.type(within(dialog).getByLabelText("Search keynotes"), "slab");
    expect(within(tree).queryByRole("treeitem", { name: /09 Finishes/ })).toBeNull();
    expect(within(tree).getByRole("treeitem", { name: /03 30 00.A1/ })).toBeInTheDocument();
    // Numbering.
    await userEvent.click(within(dialog).getByRole("radio", { name: "By sheet" }));
    expect(argsOf("keynote_set_numbering")[0]).toEqual({ numbering: "BySheet" });
    // Delete asks first.
    await userEvent.click(within(tree).getByRole("treeitem", { name: /03 30 00.A1/ }));
    await userEvent.click(within(dialog).getByRole("button", { name: "Delete…" }));
    await userEvent.click(
      within(within(dialog).getByRole("alert")).getByRole("button", { name: "Delete" }),
    );
    expect(argsOf("keynote_delete")[0]).toEqual({ key: "03 30 00.A1" });
  });

  it("assigns keynotes to types and materials from one table", async () => {
    await start();
    await userEvent.click(screen.getByRole("button", { name: /Manager/ }));
    const dialog = await screen.findByRole("dialog", { name: "Keynote Manager" });
    await userEvent.click(within(dialog).getByRole("tab", { name: /Assign to Types/ }));
    await within(dialog).findByText(/0 of 2 keynoted/);
    await userEvent.click(
      within(dialog).getByRole("button", { name: /Keynote for Gypsum Wall Board/ }),
    );
    const picker = await screen.findByRole("dialog", { name: "Keynote for Gypsum Wall Board" });
    await userEvent.type(within(picker).getByLabelText("Search keynotes"), "type x");
    await userEvent.click(within(picker).getByRole("button", { name: "Use Keynote" }));
    expect(argsOf("keynote_assign")[0]).toEqual({ ids: ["mat-gyp"], key: "09 29 00.A2" });
    await within(dialog).findByText(/1 of 2 keynoted/);
  });
});

describe("placing keynotes (ADR-081)", () => {
  it("an Element keynote asks for the type's keynote once, then places with a leader", async () => {
    const canvas = (await start(true))!;
    fake.underCursor = [{ ids: ["w1"], label: "Wall" }];
    await userEvent.click(screen.getByRole("button", { name: /^Element/ }));
    await clickAt(canvas, 300, 300);
    const picker = await screen.findByRole("dialog", { name: /Keynote for Exterior/ });
    // Search, then double-click the keynote.
    await userEvent.type(within(picker).getByLabelText("Search keynotes"), "1/2");
    await userEvent.dblClick(within(picker).getByRole("treeitem", { name: /09 29 00.A1/ }));
    await waitFor(() => expect(argsOf("keynote_assign")).toHaveLength(1));
    expect(argsOf("keynote_assign")[0]!.key).toBe("09 29 00.A1");
    await waitFor(() => expect(screen.queryByRole("dialog", { name: /Keynote for/ })).toBeNull());
    await clickAt(canvas, 420, 200);
    await waitFor(() => expect(argsOf("keynote_place")).toHaveLength(1));
    const p = argsOf("keynote_place")[0]!;
    expect(p.source).toEqual({ Element: { target: "w1" } });
    expect(p.arrow).not.toBeNull();
    expect(p.style).toBe("Key");
  });

  it("a Material keynote lets you choose among the element's materials", async () => {
    const canvas = (await start(true))!;
    fake.underCursor = [{ ids: ["w1"], label: "Wall" }];
    fake.keynoteAssigned["mat-gyp"] = "09 29 00.A1";
    await userEvent.click(screen.getByRole("button", { name: /^Material/ }));
    await clickAt(canvas, 300, 300);
    const menu = await screen.findByRole("menu", { name: "Which material?" });
    await userEvent.click(within(menu).getByRole("menuitem", { name: /Gypsum Wall Board/ }));
    await clickAt(canvas, 420, 200);
    await waitFor(() => expect(argsOf("keynote_place")).toHaveLength(1));
    expect(argsOf("keynote_place")[0]!.source).toEqual({
      Material: { target: "w1", material: "mat-gyp" },
    });
  });

  it("a User keynote uses the options bar's keynote, without a leader when off", async () => {
    const canvas = (await start(true))!;
    await userEvent.click(screen.getByRole("button", { name: /^User/ }));
    await userEvent.click(screen.getByRole("button", { name: "Keynote to place" }));
    const picker = await screen.findByRole("dialog", { name: "User Keynote" });
    await userEvent.type(within(picker).getByLabelText("Search keynotes"), "slab");
    await userEvent.click(within(picker).getByRole("button", { name: "Use Keynote" }));
    await userEvent.selectOptions(screen.getByLabelText("Keynote tag type"), "KeyAndText");
    await userEvent.click(screen.getByLabelText("Leader"));
    await clickAt(canvas, 300, 300);
    await waitFor(() => expect(argsOf("keynote_place")).toHaveLength(1));
    expect(argsOf("keynote_place")[0]).toMatchObject({
      source: { User: { key: "03 30 00.A1" } },
      arrow: null,
      style: "KeyAndText",
    });
  });

  it("the Legend opens the project's keynote legend", async () => {
    await start();
    await userEvent.click(screen.getByRole("button", { name: /Legend/ }));
    await waitFor(() => expect(useAppStore.getState().activeView).toBe("legend-1"));
  });
});
