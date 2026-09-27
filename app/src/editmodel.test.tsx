import { beforeEach, describe, expect, it } from "vitest";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { App } from "./App";
import { useAppStore } from "./store";
import { FAKE_IDS, installFakeBackend, type FakeBackend } from "./test/fakeBackend";

let fake: FakeBackend;

beforeEach(() => {
  useAppStore.setState({
    app: null,
    error: null,
    openViews: [],
    activeView: null,
    selection: [],
    tool: "select",
    highlight: [],
    editModel: { open: false, log: [] },
  });
  fake = installFakeBackend();
});

async function openProject() {
  render(<App />);
  await userEvent.click(screen.getByRole("button", { name: "New Project" }));
  await screen.findByRole("toolbar", { name: "Tools" });
}

const dialog = () => screen.queryByRole("dialog", { name: "Edit model with Claude" });

describe("Edit Model with Claude (ADR-050)", () => {
  it("is offered in plans, elevations, sections and 3D, and Ctrl+K opens it", async () => {
    await openProject();
    expect(screen.getByRole("button", { name: "Edit model with Claude" })).toBeVisible();
    await userEvent.keyboard("{Control>}k{/Control}");
    expect(dialog()).not.toBeNull();
    await userEvent.keyboard("{Escape}");
    await waitFor(() => expect(dialog()).toBeNull());
    // Elevations too (ADR-050); the view is Claude's context.
    useAppStore.getState().openView(FAKE_IDS.north);
    await screen.findByRole("button", { name: "Edit model with Claude" });
    await userEvent.keyboard("{Control>}k{/Control}");
    expect(dialog()).not.toBeNull();
    await userEvent.keyboard("{Escape}");
    await waitFor(() => expect(dialog()).toBeNull());
    // Not on the Standards tab: no button, and Ctrl+K does nothing.
    useAppStore.getState().setRibbonTab("Standards");
    await waitFor(() =>
      expect(screen.queryByRole("button", { name: "Edit model with Claude" })).toBeNull(),
    );
    await userEvent.keyboard("{Control>}k{/Control}");
    expect(dialog()).toBeNull();
  });

  it("previews Claude's edit, highlights it, and applies it as one undo step", async () => {
    await openProject();
    await userEvent.click(screen.getByRole("button", { name: "Edit model with Claude" }));
    const box = screen.getByRole("textbox", { name: "Describe a change to the model" });
    expect(box).toHaveFocus();
    await userEvent.type(box, "change all doors to 3'-0\"{Enter}");
    const card = await screen.findByLabelText("Preview");
    expect(card.textContent).toContain("PREVIEW · 2 ELEMENTS HIGHLIGHTED");
    expect(card.textContent).toContain("2'-8\" → 3'-0\"");
    expect(useAppStore.getState().highlight).toEqual(["door-1", "door-2"]);
    const call = fake.calls.find((c) => c.cmd === "model_edit_preview");
    expect(call?.args).toMatchObject({
      prompt: "change all doors to 3'-0\"",
      view: FAKE_IDS.plan1,
    });
    // Nothing changed yet; Apply commits it.
    expect(fake.calls.some((c) => c.cmd === "model_edit_apply")).toBe(false);
    await userEvent.click(screen.getByRole("button", { name: /APPLY/ }));
    await waitFor(() => expect(fake.calls.some((c) => c.cmd === "model_edit_apply")).toBe(true));
    expect(useAppStore.getState().highlight).toEqual([]);
    const history = await screen.findByRole("list", { name: "History" });
    expect(history.textContent).toContain("Width → 3'-0\" on 2 doors");
    expect(box).toHaveValue("");
    // UNDO in the history undoes that step; REDO puts it back.
    await userEvent.click(within(history).getByRole("button", { name: "UNDO" }));
    await waitFor(() =>
      expect(within(history).getByRole("button", { name: "REDO" })).toBeEnabled(),
    );
    expect(history.querySelector("li")?.className).toBe("undone");
    await userEvent.click(within(history).getByRole("button", { name: "REDO" }));
    await waitFor(() =>
      expect(within(history).getByRole("button", { name: "UNDO" })).toBeEnabled(),
    );
  });

  it("previews a plan of several steps: what it creates, changes and deletes (ADR-051)", async () => {
    await openProject();
    await userEvent.click(screen.getByRole("button", { name: "Edit model with Claude" }));
    await userEvent.type(
      screen.getByRole("textbox", { name: "Describe a change to the model" }),
      "add an office off the east side{Enter}",
    );
    const card = await screen.findByLabelText("Preview");
    expect(card.textContent).toContain("PREVIEW · 4 STEPS · 1 HIGHLIGHTED");
    expect(card.textContent).toContain("Adds a 12' x 10' office");
    expect(card.textContent).toContain("Creates2 Walls, 1 Doors");
    expect(card.textContent).toContain("Deletes1 Windows");
    expect(within(card).getAllByRole("listitem")).toHaveLength(4);
    await userEvent.click(screen.getByRole("button", { name: /APPLY/ }));
    await waitFor(() => expect(fake.calls.some((c) => c.cmd === "model_edit_apply")).toBe(true));
    const call = fake.calls.find((c) => c.cmd === "model_edit_apply");
    expect(
      (call?.args as { editPlan: { operations: unknown[] } }).editPlan.operations,
    ).toHaveLength(1);
  });

  it("shows why a request can't be done, and editing the text drops a preview", async () => {
    await openProject();
    await userEvent.click(screen.getByRole("button", { name: "Edit model with Claude" }));
    const box = screen.getByRole("textbox", { name: "Describe a change to the model" });
    await userEvent.type(box, "paint it blue{Enter}");
    expect(await screen.findByRole("alert")).toHaveTextContent("no blue material");
    expect(screen.queryByLabelText("Preview")).toBeNull();
    await userEvent.clear(box);
    // A suggestion chip fills the prompt and previews it.
    await userEvent.click(screen.getByRole("button", { name: "Set all doors to 3'-0\" W" }));
    await screen.findByLabelText("Preview");
    await userEvent.type(box, " please");
    expect(screen.queryByLabelText("Preview")).toBeNull();
    expect(useAppStore.getState().highlight).toEqual([]);
    expect(screen.getByRole("button", { name: /PREVIEW CHANGE/ })).toBeEnabled();
  });

  it("closes, dropping the preview, when the view changes to one it isn't offered in", async () => {
    await openProject();
    await userEvent.click(screen.getByRole("button", { name: "Edit model with Claude" }));
    await userEvent.type(
      screen.getByRole("textbox", { name: "Describe a change to the model" }),
      "doors 3'-0\"{Enter}",
    );
    await screen.findByLabelText("Preview");
    useAppStore.getState().setRibbonTab("Standards");
    await waitFor(() => expect(dialog()).toBeNull());
    expect(useAppStore.getState().highlight).toEqual([]);
    expect(useAppStore.getState().editModel.open).toBe(false);
  });
});
