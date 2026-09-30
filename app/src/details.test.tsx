import { beforeEach, describe, expect, it } from "vitest";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { App } from "./App";
import { activeViewInfo, useAppStore } from "./store";
import { toolAllowed } from "./tools";
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

async function openDetailsTab() {
  render(<App />);
  await userEvent.click(screen.getByRole("button", { name: "New Project" }));
  await screen.findByRole("toolbar", { name: "Tools" });
  await userEvent.click(screen.getByRole("tab", { name: "Details" }));
}

const argsOf = (cmd: string) =>
  fake.calls.filter((c) => c.cmd === cmd).map((c) => c.args as Record<string, unknown>);

describe("Details tab (ADR-069)", () => {
  it("has its own group after Model, with Drafting View, Detail Library, Filled Region", async () => {
    await openDetailsTab();
    const tabs = within(screen.getByRole("tablist", { name: "Ribbon tabs" }))
      .getAllByRole("tab")
      .map((t) => t.textContent);
    expect(tabs.indexOf("Details")).toBe(tabs.indexOf("Modify") + 1);
    for (const name of [/Drafting View/, /Detail Library/, /Filled Region/, /Detail Line/]) {
      expect(screen.getByRole("button", { name })).toBeInTheDocument();
    }
  });

  it("the library shows thumbnails by category and inserts a detail as a drafting view", async () => {
    await openDetailsTab();
    await userEvent.click(screen.getByRole("button", { name: /Detail Library/ }));
    const dialog = await screen.findByRole("dialog", { name: "Detail Library" });
    const grid = within(dialog).getByRole("list", { name: "Details" });
    await within(grid).findByText("Thickened Slab Edge");
    expect(within(grid).getAllByRole("listitem")).toHaveLength(3);
    // Each card shows its typical scale and a drawn thumbnail.
    expect(within(grid).getByText(`3" = 1'-0"`)).toBeInTheDocument();
    await waitFor(() =>
      expect(within(grid).getByRole("img", { name: "Eave with Gutter" }).tagName).toBe("svg"),
    );
    // Categories filter; so does the search.
    await userEvent.click(within(dialog).getByRole("button", { name: /^Roofs/ }));
    expect(within(grid).getAllByRole("listitem")).toHaveLength(1);
    await userEvent.click(within(dialog).getByRole("button", { name: /^All/ }));
    await userEvent.type(within(dialog).getByLabelText("Search details"), "window");
    expect(
      within(grid)
        .getAllByRole("listitem")
        .map((c) => c.textContent),
    ).toEqual([`Window Head - Wood Frame3" = 1'-0"`]);
    await userEvent.click(within(grid).getByRole("listitem"));
    expect(within(dialog).getByRole("heading", { name: "Window Head - Wood Frame" })).toBeTruthy();
    await userEvent.click(within(dialog).getByRole("button", { name: "Insert Detail" }));
    expect(argsOf("detail_insert")[0]).toEqual({ id: "window-head" });
    // It opens, as a drafting view at its scale.
    await waitFor(() => expect(activeViewInfo(useAppStore.getState())?.viewType).toBe("Drafting"));
    expect(activeViewInfo(useAppStore.getState())?.scale).toBe(4);
    expect(screen.queryByRole("dialog", { name: "Detail Library" })).toBeNull();
    // Listed under Drafting Views.
    expect(screen.getAllByText("Drafting Views").length).toBeGreaterThan(0);
  });

  it("makes an empty drafting view at a chosen scale", async () => {
    await openDetailsTab();
    await userEvent.click(screen.getByRole("button", { name: /Drafting View/ }));
    const dialog = await screen.findByRole("dialog", { name: "New Drafting View" });
    await userEvent.type(within(dialog).getByLabelText("Name"), "Typ. Flashing");
    await userEvent.selectOptions(within(dialog).getByLabelText("Scale"), "2");
    await userEvent.click(within(dialog).getByRole("button", { name: "OK" }));
    expect(argsOf("create_drafting_view")[0]).toEqual({ name: "Typ. Flashing", scale: 2 });
    await waitFor(() => expect(activeViewInfo(useAppStore.getState())?.name).toBe("Typ. Flashing"));
  });

  it("sketches filled regions with a pattern; drafting views allow only 2D tools", async () => {
    await openDetailsTab();
    await userEvent.click(screen.getByRole("button", { name: /Filled Region/ }));
    expect(argsOf("sketch_begin")[0]).toMatchObject({ kind: "FilledRegion", target: null });
    expect(
      await screen.findByRole("tab", { name: "Modify | Create Filled Region Boundary" }),
    ).toBeTruthy();
    await userEvent.selectOptions(screen.getByLabelText("Fill Pattern"), "Concrete");
    expect(argsOf("sketch_set_pattern")[0]).toEqual({ pattern: "Concrete" });
    expect(toolAllowed("detailLine", "Drafting")).toBe(true);
    expect(toolAllowed("text", "Drafting")).toBe(true);
    expect(toolAllowed("wall", "Drafting")).toBe(false);
    expect(toolAllowed("door", "Drafting")).toBe(false);
  });
});

describe("Your details (ADR-073)", () => {
  it("saves a drafting view to the library and deletes it again", async () => {
    await openDetailsTab();
    // Save to Library needs a drafting view open: insert one first.
    expect(screen.getByRole("button", { name: /Save to Library/ })).toBeDisabled();
    await userEvent.click(screen.getByRole("button", { name: /Detail Library/ }));
    let dialog = await screen.findByRole("dialog", { name: "Detail Library" });
    await within(dialog).findByText("Eave with Gutter");
    await userEvent.click(within(dialog).getByText("Eave with Gutter"));
    await userEvent.click(within(dialog).getByRole("button", { name: "Insert Detail" }));
    await waitFor(() => expect(activeViewInfo(useAppStore.getState())?.viewType).toBe("Drafting"));
    await userEvent.click(screen.getByRole("button", { name: /Save to Library/ }));
    const save = await screen.findByRole("dialog", { name: "Save to Library" });
    expect(within(save).getByLabelText("Name")).toHaveValue("Eave with Gutter");
    const name = within(save).getByLabelText("Name");
    await userEvent.clear(name);
    await userEvent.type(name, "Our Eave");
    await userEvent.click(within(save).getByRole("button", { name: "Save" }));
    await waitFor(() =>
      expect(argsOf("detail_save")[0]).toMatchObject({ name: "Our Eave", category: "My Details" }),
    );
    // It's in the library under My Details, and can be deleted (two clicks, no pop-up).
    await userEvent.click(screen.getByRole("button", { name: /Detail Library/ }));
    dialog = await screen.findByRole("dialog", { name: "Detail Library" });
    await userEvent.click(await within(dialog).findByRole("button", { name: /^My Details/ }));
    await userEvent.click(within(dialog).getByText("Our Eave"));
    const del = within(dialog).getByRole("button", { name: "Delete from Library" });
    await userEvent.click(del);
    await userEvent.click(within(dialog).getByRole("button", { name: "Click again to delete" }));
    await waitFor(() => expect(argsOf("detail_delete")[0]).toEqual({ id: "user:u1" }));
    await waitFor(() => expect(within(dialog).queryByText("Our Eave")).toBeNull());
  });
});
