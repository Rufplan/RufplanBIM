import { describe, expect, it } from "vitest";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ScheduleDialog } from "./components/ScheduleDialog";
import { installFakeBackend } from "./test/fakeBackend";

// Double-clicking the sheet index on a sheet opens it to edit (ADR-110): its font and text,
// and the placeholder sheets it lists that aren't in the project.
describe("schedule dialog", () => {
  it("edits the sheet index's appearance and adds a placeholder sheet", async () => {
    const fake = installFakeBackend();
    const calls = (cmd: string) =>
      fake.calls.filter((c) => c.cmd === cmd).map((c) => c.args as Record<string, unknown>);
    render(<ScheduleDialog view="index" onClose={() => {}} />);
    const dialog = await screen.findByRole("dialog", { name: "Schedule" });
    expect(await within(dialog).findByRole("heading", { name: "SHEET INDEX" })).toBeTruthy();
    await userEvent.selectOptions(await within(dialog).findByLabelText("Font"), "Sans");
    await userEvent.click(within(dialog).getByRole("button", { name: "Add Placeholder Sheet" }));
    await userEvent.type(within(dialog).getByLabelText("Placeholder 1 number"), "S-101");
    await userEvent.type(within(dialog).getByLabelText("Placeholder 1 name"), "Foundation Plan");
    await userEvent.click(within(dialog).getByRole("button", { name: "OK" }));
    expect(calls("set_schedule_style")[0]).toMatchObject({
      view: "index",
      style: { font: "Sans" },
    });
    expect(fake.placeholders).toEqual([{ number: "S-101", name: "Foundation Plan" }]);
  });
});
