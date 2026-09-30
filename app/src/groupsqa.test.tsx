import { beforeEach, describe, expect, it } from "vitest";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { App } from "./App";
import { useQa } from "./components/Qa";
import { toGroups, withMembers } from "./groups";
import { litOf, useAppStore } from "./store";
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
    ribbonTab: "Architecture",
  });
  useQa.setState({
    report: null,
    open: false,
    resolved: [],
    category: null,
    severity: null,
    query: "",
    hideResolved: false,
  });
  fake = installFakeBackend();
});

async function openProject() {
  render(<App />);
  await userEvent.click(screen.getByRole("button", { name: "New Project" }));
  await screen.findByRole("toolbar", { name: "Tools" });
}

const lastCall = (cmd: string) =>
  [...fake.calls].reverse().find((c) => c.cmd === cmd)?.args as Record<string, unknown> | undefined;

describe("Collaborate holds the Rufplan tools", () => {
  it("has Sign In, Link Project and Publish, and no Rufplan tab", async () => {
    await openProject();
    const tabs = within(screen.getByRole("tablist", { name: "Ribbon tabs" }))
      .getAllByRole("tab")
      .map((t) => t.textContent);
    expect(tabs).not.toContain("Rufplan");
    await userEvent.click(screen.getByRole("tab", { name: "Collaborate" }));
    for (const name of [/Sign In|Account/, /Link Project/, /Publish/]) {
      expect(screen.getByRole("button", { name })).toBeInTheDocument();
    }
  });
});

describe("Model and detail groups (ADR-087)", () => {
  it("Create Group names the selection, and clicking a member selects the group", async () => {
    await openProject();
    useAppStore.getState().select(["wall-a", "wall-b"]);
    useAppStore.getState().setUi({ viewDialog: "createGroup" });
    const dialog = await screen.findByRole("dialog", { name: "Create Group" });
    await userEvent.type(within(dialog).getByLabelText("Group name"), "Unit A");
    await userEvent.click(within(dialog).getByRole("button", { name: "OK" }));
    await waitFor(() =>
      expect(lastCall("group_create")).toEqual({ ids: ["wall-a", "wall-b"], name: "Unit A" }),
    );
    expect(useAppStore.getState().selection).toEqual(["group-1"]);
    // A member picked later selects the whole group; the group lights its members.
    useAppStore.getState().select(["wall-b"]);
    expect(useAppStore.getState().selection).toEqual(["group-1"]);
    expect(litOf(useAppStore.getState())).toEqual(["group-1", "wall-a", "wall-b"]);
    // The contextual tab reads as Revit's, with Edit Group and Ungroup.
    expect(await screen.findByRole("tab", { name: "Modify | Model Groups" })).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Edit Group" }));
    await waitFor(() => expect(lastCall("group_edit")).toEqual({ id: "group-1" }));
    // Edit Group mode: its own tab, and members select on their own.
    expect(
      await screen.findByRole("tab", { name: "Edit Group | Model : Unit A" }),
    ).toBeInTheDocument();
    useAppStore.getState().select(["wall-b"]);
    expect(useAppStore.getState().selection).toEqual(["wall-b"]);
    await userEvent.click(screen.getByRole("button", { name: "Finish" }));
    await waitFor(() => expect(fake.calls.some((c) => c.cmd === "group_finish")).toBe(true));
  });

  it("maps members to their group unless it's being edited", () => {
    const app = { groups: [{ id: "g", members: ["a", "b"] }], editingGroup: null } as never;
    expect(toGroups(app, ["a", "x", "b"])).toEqual(["g", "x"]);
    expect(withMembers(app, ["g"])).toEqual(["g", "a", "b"]);
    const editing = { groups: [{ id: "g", members: ["a", "b"] }], editingGroup: "g" } as never;
    expect(toGroups(editing, ["a"])).toEqual(["a"]);
  });
});

describe("QA/QC tab (ADR-088)", () => {
  it("sits in the Document group and reviews for the chosen milestone", async () => {
    await openProject();
    const tabs = within(screen.getByRole("tablist", { name: "Ribbon tabs" }))
      .getAllByRole("tab")
      .map((t) => t.textContent);
    expect(tabs[tabs.indexOf("Specifications") + 1]).toBe("QA/QC");
    await userEvent.click(screen.getByRole("tab", { name: "QA/QC" }));
    await userEvent.selectOptions(screen.getByLabelText("Milestone"), "Permit");
    await userEvent.click(screen.getByRole("checkbox", { name: /Consultants/ }));
    await userEvent.click(screen.getByRole("button", { name: "Run Review" }));
    await waitFor(() => expect(lastCall("qa_review")).toBeDefined());
    const opts = lastCall("qa_review")!.options as { milestone: string; categories: string[] };
    expect(opts.milestone).toBe("Permit");
    expect(opts.categories).not.toContain("Consultants");
    // The pop-up list.
    const panel = await screen.findByRole("dialog", { name: "QA/QC findings" });
    expect(within(panel).getByText("Bedroom has no emergency escape opening")).toBeInTheDocument();
    expect(within(panel).getByText("IRC R310.1")).toBeInTheDocument();
    expect(within(panel).getByLabelText("Score 84 of 100")).toBeInTheDocument();
    // Filter to Minor; resolve it; Show selects what a finding is about.
    await userEvent.click(within(panel).getByRole("radio", { name: /Minor/ }));
    expect(within(panel).queryByText("Bedroom has no emergency escape opening")).toBeNull();
    await userEvent.click(within(panel).getByRole("button", { name: "Resolve" }));
    expect(useQa.getState().resolved).toEqual(["tags"]);
    await userEvent.click(within(panel).getByRole("radio", { name: /^All/ }));
    const item = within(panel)
      .getByText("Bedroom has no emergency escape opening")
      .closest(".qa-item") as HTMLElement;
    await userEvent.click(within(item).getByRole("button", { name: "Show" }));
    expect(useAppStore.getState().selection).toEqual(["room-1"]);
  });

  it("adds Claude's overall review", async () => {
    await openProject();
    await userEvent.click(screen.getByRole("tab", { name: "QA/QC" }));
    await userEvent.click(screen.getByRole("button", { name: "Overall Review" }));
    const panel = await screen.findByRole("dialog", { name: "QA/QC findings" });
    expect(
      await within(panel).findByText("The set is close; fix egress first."),
    ).toBeInTheDocument();
  });
});

describe("Fix Issues (ADR-089)", () => {
  async function reviewed() {
    await openProject();
    await userEvent.click(screen.getByRole("tab", { name: "QA/QC" }));
    await userEvent.click(screen.getByRole("button", { name: "Run Review" }));
    await screen.findByRole("dialog", { name: "QA/QC findings" });
  }
  const applies = () => fake.calls.filter((c) => c.cmd === "qa_fix_apply");

  it("Auto applies every fix as one step and reviews again", async () => {
    await reviewed();
    await userEvent.click(screen.getByRole("radio", { name: "Auto" }));
    await userEvent.click(screen.getByRole("button", { name: "Fix Issues" }));
    const dialog = await screen.findByRole("dialog", { name: "Fix Issues" });
    expect(within(dialog).getByText("DESIGN CHANGE")).toBeInTheDocument();
    await userEvent.click(within(dialog).getByRole("button", { name: "APPLY 2 FIXES" }));
    await waitFor(() => expect(applies()).toHaveLength(1));
    expect((applies()[0]!.args as { actions: unknown[] }).actions).toHaveLength(2);
    expect(
      await within(dialog).findByText("Score 84 → 100. Undo (Ctrl+Z) takes it all back."),
    ).toBeInTheDocument();
  });

  it("Approve each asks for every change: apply one, skip one", async () => {
    await reviewed();
    await userEvent.click(screen.getByRole("radio", { name: "Approve each" }));
    await userEvent.click(screen.getByRole("button", { name: "Fix Issues" }));
    const dialog = await screen.findByRole("dialog", { name: "Fix Issues" });
    expect(within(dialog).getByText("CHANGE 1 OF 2")).toBeInTheDocument();
    await userEvent.click(within(dialog).getByRole("button", { name: "APPLY" }));
    await waitFor(() => expect(applies()).toHaveLength(1));
    expect((applies()[0]!.args as { actions: unknown[]; label: string }).label).toBe(
      "QA/QC Fix: Tag doors in Level 1",
    );
    expect(await within(dialog).findByText("CHANGE 2 OF 2")).toBeInTheDocument();
    await userEvent.click(within(dialog).getByRole("button", { name: "SKIP" }));
    expect(await within(dialog).findByText("1 fixed · 1 skipped")).toBeInTheDocument();
    expect(applies()).toHaveLength(1);
  });

  it("a finding with a fix has its own Fix button", async () => {
    await reviewed();
    const panel = screen.getByRole("dialog", { name: "QA/QC findings" });
    const buttons = await within(panel).findAllByRole("button", { name: "Fix" });
    await userEvent.click(buttons[0]!);
    await waitFor(() => expect(applies()).toHaveLength(1));
  });
});
