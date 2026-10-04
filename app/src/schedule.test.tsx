import { describe, expect, it } from "vitest";
import { fireEvent, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ScheduleDialog } from "./components/ScheduleDialog";
import { installFakeBackend } from "./test/fakeBackend";
import { insertRow, moveRow, removable } from "./sheetIndex";

// Double-clicking the sheet index on a sheet opens it to edit (ADR-110, ADR-113): its font
// and text, and its rows: sheets renamed, added, ordered, and placeholders.
describe("schedule dialog", () => {
  it("edits the sheet index's appearance and its rows", async () => {
    const fake = installFakeBackend();
    const calls = (cmd: string) =>
      fake.calls.filter((c) => c.cmd === cmd).map((c) => c.args as Record<string, unknown>);
    render(<ScheduleDialog view="index" onClose={() => {}} />);
    const dialog = await screen.findByRole("dialog", { name: "Schedule" });
    expect(await within(dialog).findByRole("heading", { name: "SHEET INDEX" })).toBeTruthy();
    await userEvent.selectOptions(await within(dialog).findByLabelText("Font"), "Sans");
    // Rename the sheet, add a placeholder after it, then move the placeholder up.
    const name = await within(dialog).findByLabelText("Row 1 name");
    await userEvent.clear(name);
    await userEvent.type(name, "First Floor Plan");
    await userEvent.click(within(dialog).getByRole("button", { name: "Add Placeholder" }));
    const number = await within(dialog).findByLabelText("Row 2 number");
    expect((number as HTMLInputElement).value).toBe("A-102");
    await userEvent.clear(number);
    await userEvent.type(number, "S-101");
    await userEvent.type(within(dialog).getByLabelText("Row 2 name"), "Foundation Plan");
    fireEvent.contextMenu(within(dialog).getByRole("listitem", { name: "S-101 Foundation Plan" }));
    await userEvent.click(await screen.findByRole("menuitem", { name: "Move Up" }));
    await userEvent.click(within(dialog).getByRole("button", { name: "OK" }));
    expect(calls("set_schedule_style")[0]).toMatchObject({
      view: "index",
      style: { font: "Sans" },
    });
    expect(fake.indexRows).toEqual([
      { sheet: null, number: "S-101", name: "Foundation Plan", placeholder: true },
      { sheet: "s1", number: "A-101", name: "First Floor Plan", placeholder: false },
    ]);
  });

  it("inserts, moves and keeps sheets that only the project browser deletes", () => {
    const a = { sheet: "a", number: "A-101", name: "", placeholder: false };
    const b = { sheet: null, number: "A-102", name: "", placeholder: false };
    const c = { sheet: null, number: "S-101", name: "", placeholder: true };
    expect(insertRow([a, c], 1, b)).toEqual([a, b, c]);
    expect(moveRow([a, b, c], 2, 0)).toEqual([c, a, b]);
    expect(moveRow([a, b, c], 0, 9)).toEqual([b, c, a]);
    expect([a, b, c].map(removable)).toEqual([false, true, true]);
  });
});
