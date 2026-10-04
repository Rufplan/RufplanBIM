import { describe, expect, it } from "vitest";
import { fireEvent, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ScheduleDialog } from "./components/ScheduleDialog";
import { installFakeBackend } from "./test/fakeBackend";
import { dropGap, dropTo, indexProblems, insertRow, moveRow, removable } from "./sheetIndex";

// Double-clicking the sheet index on a sheet opens the Sheet Index dialog (ADR-110, 113,
// 114): its type and a 1:1 preview, and its rows: sheets renamed, added, ordered, and
// placeholders.
describe("sheet index dialog", () => {
  it("edits the sheet index's type and its rows", async () => {
    const fake = installFakeBackend();
    const calls = (cmd: string) =>
      fake.calls.filter((c) => c.cmd === cmd).map((c) => c.args as Record<string, unknown>);
    render(<ScheduleDialog view="index" onClose={() => {}} />);
    const dialog = await screen.findByRole("dialog", { name: "Sheet Index" });
    expect(await within(dialog).findByText("1 SHEETS")).toBeTruthy();
    await userEvent.selectOptions(await within(dialog).findByLabelText("Font"), "Sans");
    await userEvent.click(
      within(within(dialog).getByRole("group", { name: "TITLE height" })).getByRole("button", {
        name: `1/4"`,
      }),
    );
    await userEvent.click(within(dialog).getByRole("button", { name: "Row larger" }));
    // Rename the sheet, add a placeholder, then move the placeholder up.
    const name = await within(dialog).findByLabelText("Row 1 name");
    await userEvent.clear(name);
    await userEvent.type(name, "First Floor Plan");
    await userEvent.click(within(dialog).getByRole("button", { name: "+ ADD PLACEHOLDER" }));
    const number = await within(dialog).findByLabelText("Row 2 number");
    expect((number as HTMLInputElement).value).toBe("A-102");
    expect(within(dialog).getByText("No new sheets · 1 placeholder")).toBeTruthy();
    expect(within(dialog).getByText("PLACEHOLDER")).toBeTruthy();
    await userEvent.clear(number);
    // An empty number holds OK back.
    expect(within(dialog).getByText("Row 2 needs a sheet number")).toBeTruthy();
    expect((within(dialog).getByRole("button", { name: "OK" }) as HTMLButtonElement).disabled).toBe(
      true,
    );
    await userEvent.type(number, "S-101");
    await userEvent.type(within(dialog).getByLabelText("Row 2 name"), "Foundation Plan");
    fireEvent.contextMenu(within(dialog).getByRole("listitem", { name: "S-101 Foundation Plan" }));
    await userEvent.click(await screen.findByRole("menuitem", { name: "Move Up" }));
    // A sheet in the project can't be removed here.
    await userEvent.click(within(dialog).getByRole("listitem", { name: "A-101 First Floor Plan" }));
    expect(
      (within(dialog).getByRole("button", { name: "REMOVE" }) as HTMLButtonElement).disabled,
    ).toBe(true);
    await userEvent.click(within(dialog).getByRole("button", { name: "OK" }));
    expect(calls("set_schedule_style")[0]).toMatchObject({
      view: "index",
      style: { font: "Sans", title: 6.35, row: 6.5 },
    });
    expect(fake.indexRows).toEqual([
      { sheet: null, number: "S-101", name: "Foundation Plan", placeholder: true },
      { sheet: "s1", number: "A-101", name: "First Floor Plan", placeholder: false },
    ]);
  }, 20000);

  it("drags a row by its handle to a new place", async () => {
    const fake = installFakeBackend();
    fake.indexRows = ["A-101", "A-102", "A-103", "A-104"].map((n, i) => ({
      sheet: `s${i}`,
      number: n,
      name: "",
      placeholder: false,
    }));
    // jsdom has no PointerEvent: a MouseEvent carries clientY just as well.
    if (!("PointerEvent" in window))
      Object.assign(window, { PointerEvent: class extends MouseEvent {} });
    render(<ScheduleDialog view="index" onClose={() => {}} />);
    const dialog = await screen.findByRole("dialog", { name: "Sheet Index" });
    await within(dialog).findByText("4 SHEETS");
    // Rows 40 px tall, top down.
    within(dialog)
      .getAllByRole("listitem")
      .forEach((row, i) =>
        Object.defineProperty(row, "getBoundingClientRect", {
          value: () => ({ top: i * 40, bottom: i * 40 + 40, left: 0, right: 600 }),
        }),
      );
    const handle = within(dialog).getByLabelText("Drag A-101 to reorder");
    fireEvent.pointerDown(handle, { button: 0, clientY: 20 });
    fireEvent.pointerMove(handle, { clientY: 130 });
    fireEvent.pointerUp(handle, { clientY: 130 });
    const numbers = within(dialog)
      .getAllByRole("listitem")
      .map((r) => r.getAttribute("aria-label")!.trim());
    expect(numbers).toEqual(["A-102", "A-103", "A-101", "A-104"]);
  });

  it("inserts, moves, checks and keeps sheets that only the project browser deletes", () => {
    const a = { sheet: "a", number: "A-101", name: "", placeholder: false };
    const b = { sheet: null, number: "A-102", name: "", placeholder: false };
    const c = { sheet: null, number: "S-101", name: "", placeholder: true };
    expect(insertRow([a, c], 1, b)).toEqual([a, b, c]);
    expect(moveRow([a, b, c], 2, 0)).toEqual([c, a, b]);
    expect(moveRow([a, b, c], 0, 9)).toEqual([b, c, a]);
    expect([a, b, c].map(removable)).toEqual([false, true, true]);
    // Rows centred at 20, 60, 100: dropping at 70 goes in the gap before the third.
    expect(dropGap([20, 60, 100], 70)).toBe(2);
    expect(dropGap([20, 60, 100], 500)).toBe(3);
    expect(dropTo(0, 2)).toBe(1);
    expect(dropTo(2, 0)).toBe(0);
    expect(dropTo(1, 2)).toBe(1);
    expect(indexProblems([a, { ...b, number: "A-101" }, { ...c, number: " " }])).toEqual([
      { row: 1, message: "Sheet A-101 is listed twice" },
      { row: 2, message: "Row 3 needs a sheet number" },
    ]);
  });
});
