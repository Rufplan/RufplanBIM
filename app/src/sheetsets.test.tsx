import { beforeEach, describe, expect, it } from "vitest";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { App } from "./App";
import { useAppStore } from "./store";
import { installFakeBackend, type FakeBackend } from "./test/fakeBackend";

let fake: FakeBackend;

beforeEach(() => {
  useAppStore.setState({
    app: null,
    error: null,
    confirm: null,
    openViews: [],
    activeView: null,
    selection: [],
    tool: "select",
    viewDialog: null,
  });
  fake = installFakeBackend();
});

const calls = (cmd: string) =>
  fake.calls.filter((c) => c.cmd === cmd).map((c) => c.args as Record<string, unknown>);

describe("Sheet Sets (ADR-032)", () => {
  it("previews each phase's deliverables, creates the sheets and exports the PDFs", async () => {
    render(<App />);
    await userEvent.click(screen.getByRole("button", { name: "New Project" }));
    await screen.findByRole("toolbar", { name: "Tools" });
    await userEvent.click(screen.getByRole("tab", { name: "View" }));
    await userEvent.click(screen.getByRole("button", { name: "Sheet Sets" }));
    const dialog = await screen.findByRole("dialog", { name: "Sheet Sets" });
    // Every phase by default: the sheet index and the CD deliverables.
    const table = await within(dialog).findByRole("table", { name: "Sheets" });
    expect(await within(table).findByText("Level 1 Floor Plan")).toBeTruthy();
    const list = within(dialog).getByRole("list", { name: "Deliverables" });
    expect(within(list).getByText("Permit Set")).toBeTruthy();
    // Only SD: the foundation plan (DD on) drops out.
    for (const p of ["PD", "DD", "CD", "BN", "CA"]) {
      await userEvent.click(within(dialog).getByRole("checkbox", { name: new RegExp(`^${p} `) }));
    }
    expect(await within(list).findByText("100% Schematic Design")).toBeTruthy();
    expect(within(list).queryByText("Permit Set")).toBeNull();
    expect(within(table).queryByText("Foundation Plan")).toBeNull();
    expect(calls("sheet_set_plan").at(-1)).toMatchObject({
      options: { buildingType: "SingleFamily", phases: ["SD"], size: "ArchD" },
    });
    // Hotels get their own sheets.
    await userEvent.click(within(dialog).getByRole("checkbox", { name: /^CD / }));
    await userEvent.selectOptions(within(dialog).getByLabelText("Building type"), "Hotel");
    expect(await within(table).findByText("Hotel Foundation Plan")).toBeTruthy();
    // Create, then export into a folder, recorded as issued.
    await userEvent.click(within(dialog).getByRole("button", { name: "Create Sheets" }));
    expect(await within(dialog).findByText(/3 sheets created · 1 views placed/)).toBeTruthy();
    expect(calls("create_sheet_sets")[0]).toMatchObject({
      options: { buildingType: "Hotel", phases: ["SD", "CD"] },
    });
    fake.openPath = "C:/Sets";
    await userEvent.click(within(dialog).getByLabelText("Record as issued"));
    await userEvent.click(within(dialog).getByRole("button", { name: "Export PDFs…" }));
    expect(await within(dialog).findByText(/Exported 1 deliverable to C:\/Sets/)).toBeTruthy();
    expect(calls("export_sheet_sets")[0]).toEqual({
      phases: ["SD", "CD"],
      folder: "C:/Sets",
      record: true,
    });
  });
});
