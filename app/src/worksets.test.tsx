import { beforeEach, describe, expect, it } from "vitest";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { App } from "./App";
import { useAppStore } from "./store";
import { FAKE_WORKSETS, installFakeBackend, type FakeBackend } from "./test/fakeBackend";

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
    grayInactive: false,
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

describe("Worksets (ADR-079)", () => {
  it("Collaborate lists Revit's worksets with Architecture active by default", async () => {
    await start();
    await userEvent.click(screen.getByRole("tab", { name: "Collaborate" }));
    const active = within(screen.getByRole("toolbar", { name: "Tools" })).getByRole("combobox", {
      name: "Active Workset",
    }) as HTMLSelectElement;
    expect(active.selectedOptions[0]!.textContent).toBe("Architecture");
    await userEvent.click(screen.getByRole("button", { name: /^Worksets/ }));
    const dialog = await screen.findByRole("dialog", { name: "Worksets" });
    const table = within(dialog).getByRole("table");
    for (const w of FAKE_WORKSETS) expect(await within(table).findByText(w.name)).toBeTruthy();
    // The default can't be deleted.
    await userEvent.click(within(table).getByText("Architecture"));
    expect(within(dialog).getByRole("button", { name: /Delete/ })).toBeDisabled();
    await userEvent.click(within(table).getByText("Interiors"));
    await userEvent.click(within(dialog).getByRole("button", { name: /Delete/ }));
    const del = within(dialog).getByRole("group", { name: "Delete Workset" });
    await userEvent.click(within(del).getByRole("button", { name: "Delete" }));
    expect(argsOf("delete_workset")[0]).toEqual({
      ws: FAKE_WORKSETS[3]!.id,
      moveTo: FAKE_WORKSETS[0]!.id,
    });
    // New.
    await userEvent.click(within(dialog).getByRole("button", { name: /New/ }));
    await userEvent.type(within(dialog).getByLabelText("Workset name"), "Furniture{Enter}");
    expect(argsOf("create_workset")[0]).toEqual({ name: "Furniture", visible: true });
  });

  it("switches the active workset from the status bar and grays inactive ones", async () => {
    await start();
    const status = document.querySelector(".statusbar") as HTMLElement;
    const active = within(status).getByRole("combobox", { name: "Active Workset" });
    await userEvent.selectOptions(active, FAKE_WORKSETS[2]!.id);
    expect(argsOf("set_active_workset")[0]).toEqual({ ws: FAKE_WORKSETS[2]!.id });
    await waitFor(() =>
      expect(useAppStore.getState().app?.activeWorkset).toBe(FAKE_WORKSETS[2]!.id),
    );
    await userEvent.click(screen.getByRole("tab", { name: "Collaborate" }));
    await userEvent.click(screen.getByRole("button", { name: /Gray Inactive/ }));
    expect(useAppStore.getState().grayInactive).toBe(true);
    await waitFor(() => expect(argsOf("element_worksets").length).toBeGreaterThan(0));
  });

  it("Visibility/Graphics hides a workset in the view", async () => {
    await start();
    useAppStore.getState().setUi({ viewDialog: "visibility" });
    const dialog = await screen.findByRole("dialog", { name: "Visibility/Graphics" });
    await userEvent.click(within(dialog).getByRole("tab", { name: "Worksets" }));
    const group = within(dialog).getByRole("group", { name: "Worksets" });
    await userEvent.click(within(group).getByLabelText("Architecture"));
    expect(argsOf("set_workset_visible_in_view")[0]).toMatchObject({
      ws: FAKE_WORKSETS[0]!.id,
      visible: false,
    });
  });
});
