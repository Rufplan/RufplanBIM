import { beforeEach, describe, expect, it } from "vitest";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { App } from "./App";
import { activeViewInfo, useAppStore } from "./store";
import { installFakeBackend, type FakeBackend } from "./test/fakeBackend";

let fake: FakeBackend;
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

async function start() {
  render(<App />);
  await userEvent.click(screen.getByRole("button", { name: "New Project" }));
  await screen.findByRole("toolbar", { name: "Tools" });
}

const argsOf = (cmd: string) =>
  fake.calls.filter((c) => c.cmd === cmd).map((c) => c.args as Record<string, unknown>);

const browser = () => screen.getByRole("complementary", { name: "Project browser" });

describe("Ribbon tabs (ADR-074)", () => {
  it("shows Modify only on the Modify tab; View starts with New View, Sheets with New Sheet", async () => {
    await start();
    expect(screen.queryByRole("button", { name: /^Modify/ })).toBeNull();
    await userEvent.click(screen.getByRole("tab", { name: "Modify" }));
    expect(screen.getByRole("button", { name: /^Modify/ })).toBeInTheDocument();
    for (const [tab, first] of [
      ["View", /New View/],
      ["Sheets", /New Sheet/],
    ] as const) {
      await userEvent.click(screen.getByRole("tab", { name: tab }));
      const toolbar = screen.getByRole("toolbar", { name: "Tools" });
      expect(within(toolbar).getAllByRole("button")[0]!.textContent).toMatch(first);
    }
  });

  it("New View makes a floor plan of a level and opens it", async () => {
    await start();
    await userEvent.click(screen.getByRole("tab", { name: "View" }));
    await userEvent.click(screen.getByRole("button", { name: /New View/ }));
    const menu = screen.getByRole("menu", { name: "New View" });
    await userEvent.click(within(menu).getByRole("menuitem", { name: /^Floor Plan/ }));
    await userEvent.click(screen.getByRole("menuitem", { name: "Level 1" }));
    expect(argsOf("create_plan_view")[0]).toMatchObject({ ceiling: false });
    await waitFor(() => expect(activeViewInfo(useAppStore.getState())?.name).toBe("Level 1 (1)"));
  });
});

describe("Project browser right-click (ADR-074)", () => {
  it("duplicates a view with detailing", async () => {
    await start();
    const item = within(browser()).getAllByRole("button", { name: "North" })[0]!;
    fireEvent.contextMenu(item);
    const menu = screen.getByRole("menu", { name: "North menu" });
    await userEvent.click(within(menu).getByRole("menuitem", { name: /Duplicate View/ }));
    await userEvent.click(screen.getByRole("menuitem", { name: "Duplicate with Detailing" }));
    expect(argsOf("duplicate_view")[0]).toMatchObject({ detailing: true });
    await waitFor(() => expect(activeViewInfo(useAppStore.getState())?.name).toBe("North Copy 1"));
  });

  it("offers Revit's three sheet duplicates", async () => {
    await start();
    const s = fake.state!;
    fake.state = {
      ...s,
      views: [
        ...s.views,
        { ...s.views[0]!, id: "sheet-1", name: "A101 - Plans", viewType: "Sheet" },
      ],
    };
    useAppStore.getState().setApp(fake.state);
    const item = await within(browser()).findByRole("button", { name: /A101 - Plans/ });
    fireEvent.contextMenu(item);
    await userEvent.click(screen.getByRole("menuitem", { name: /Duplicate Sheet/ }));
    for (const n of ["Duplicate Empty Sheet", "Duplicate with Detailing", "Duplicate with Views"])
      expect(screen.getByRole("menuitem", { name: n })).toBeInTheDocument();
    await userEvent.click(screen.getByRole("menuitem", { name: "Duplicate with Views" }));
    expect(argsOf("duplicate_sheet")[0]).toEqual({ sheet: "sheet-1", how: "WithViews" });
  });

  it("renames a view", async () => {
    await start();
    fireEvent.contextMenu(within(browser()).getAllByRole("button", { name: "North" })[0]!);
    await userEvent.click(screen.getByRole("menuitem", { name: "Rename…" }));
    const dialog = screen.getByRole("dialog", { name: "Rename View" });
    const input = within(dialog).getByLabelText("Name");
    await userEvent.clear(input);
    await userEvent.type(input, "North Elevation{Enter}");
    expect(argsOf("set_property").at(-1)).toMatchObject({ key: "name", value: "North Elevation" });
  });
});
