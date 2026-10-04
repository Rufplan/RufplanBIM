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

describe("General Notes (ADR-103)", () => {
  it("offers the drawing's preset notes, lets you pick and add, and places them", async () => {
    render(<App />);
    await userEvent.click(screen.getByRole("button", { name: "New Project" }));
    await screen.findByRole("toolbar", { name: "Tools" });
    await userEvent.click(screen.getByRole("tab", { name: "Annotate" }));
    await userEvent.click(screen.getByRole("button", { name: "General Notes" }));
    const dialog = await screen.findByRole("dialog", { name: "General Notes" });
    // The open plan defaults to Floor Plans' notes.
    expect(await within(dialog).findByText("Dimensions are to face of stud.")).toBeTruthy();
    expect((within(dialog).getByLabelText("Heading") as HTMLInputElement).value).toBe(
      "FLOOR PLANS GENERAL NOTES",
    );
    // Another drawing brings its own.
    await userEvent.selectOptions(within(dialog).getByLabelText("Drawing"), "SitePlan");
    expect(await within(dialog).findByText("Call 811 before digging.")).toBeTruthy();
    // Leave one out, add one.
    await userEvent.click(within(dialog).getAllByRole("checkbox")[1]!);
    await userEvent.type(within(dialog).getByLabelText("Your own notes"), "Protect the oak.");
    await userEvent.click(within(dialog).getByRole("button", { name: /Place 2 Notes/ }));
    expect(fake.placedNotes).toMatchObject({
      heading: "SITE PLAN GENERAL NOTES",
      notes: ["Call 811 before digging.", "Protect the oak."],
      width: 160,
    });
  });
});
