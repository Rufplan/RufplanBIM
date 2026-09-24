import { beforeEach, describe, expect, it } from "vitest";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { App } from "./App";
import { useAppStore } from "./store";
import { pointAtLength, shortcut, startsTypedValue, sweep, toolAllowed } from "./tools";
import { installFakeBackend, type FakeBackend } from "./test/fakeBackend";

let fake: FakeBackend;

beforeEach(() => {
  useAppStore.setState({
    app: null,
    error: null,
    confirm: null,
    rufplan: null,
    paramsOpen: false,
    openViews: [],
    activeView: null,
    selection: [],
    tool: "select",
  });
  fake = installFakeBackend();
});

async function openProject() {
  render(<App />);
  await userEvent.click(screen.getByRole("button", { name: "New Project" }));
  await screen.findByRole("toolbar", { name: "Tools" });
}

describe("modify tools", () => {
  it("Revit shortcuts reach every new tool", () => {
    const pairs: [string, string][] = [
      ["CO", "copy"],
      ["RO", "rotate"],
      ["MM", "mirror"],
      ["AR", "array"],
      ["AL", "align"],
      ["TR", "trim"],
      ["OF", "offset"],
      ["SL", "split"],
      ["RF", "roof"],
      ["ST", "stair"],
    ];
    for (const [keys, tool] of pairs) {
      expect(shortcut(keys[0]!, keys[1]!).tool).toBe(tool);
    }
    expect(toolAllowed("roof", "Plan")).toBe(true);
    expect(toolAllowed("stair", "CeilingPlan")).toBe(false);
    expect(toolAllowed("copy", "CeilingPlan")).toBe(true);
    expect(toolAllowed("mirror", "Elevation")).toBe(false);
  });

  it("typed lengths and angles", () => {
    expect(startsTypedValue("1")).toBe(true);
    expect(startsTypedValue("'")).toBe(true);
    expect(startsTypedValue("a")).toBe(false);
    // 12' toward a cursor to the north-east.
    const p = pointAtLength({ x: 0, y: 0 }, { x: 3, y: 4 }, 3657.6)!;
    expect(p.x).toBeCloseTo(3657.6 * 0.6);
    expect(p.y).toBeCloseTo(3657.6 * 0.8);
    expect(pointAtLength({ x: 0, y: 0 }, { x: 0, y: 0 }, 10)).toBeNull();
    expect(sweep({ x: 0, y: 0 }, { x: 1, y: 0 }, { x: 0, y: 1 })).toBeCloseTo(Math.PI / 2);
    expect(sweep({ x: 0, y: 0 }, { x: 1, y: 0 }, { x: 0, y: -1 })).toBeCloseTo(-Math.PI / 2);
  });

  it("the Modify tab holds the Revit modify tools and the options bar shows their settings", async () => {
    await openProject();
    await userEvent.click(screen.getByRole("tab", { name: "Modify" }));
    for (const name of [
      "Copy",
      "Rotate",
      "Mirror",
      "Array",
      "Align",
      "Trim/Extend",
      "Offset",
      "Split",
    ]) {
      expect(screen.getByRole("button", { name })).toBeInTheDocument();
    }
    expect(screen.getByRole("button", { name: "Flip" })).toBeDisabled();
    await userEvent.click(screen.getByRole("button", { name: "Array" }));
    const bar = screen.getByRole("group", { name: "Tool options" });
    const n = within(bar).getByLabelText("Number of items");
    await userEvent.clear(n);
    await userEvent.type(n, "5");
    expect(useAppStore.getState().options.arrayCount).toBe(5);
    await userEvent.click(screen.getByRole("button", { name: "Mirror" }));
    expect(
      within(screen.getByRole("group", { name: "Tool options" })).getByLabelText("Copy"),
    ).toBeChecked();
  });

  it("the Architecture tab places roofs and stairs", async () => {
    await openProject();
    await userEvent.click(screen.getByRole("button", { name: "Roof" }));
    expect(useAppStore.getState().tool).toBe("roof");
    await userEvent.click(screen.getByRole("button", { name: "Stair" }));
    expect(useAppStore.getState().tool).toBe("stair");
  });

  it("project parameters are added and removed from the Manage tab", async () => {
    await openProject();
    await userEvent.click(screen.getByRole("tab", { name: "Manage" }));
    await userEvent.click(screen.getByRole("button", { name: "Project Parameters" }));
    const dialog = await screen.findByRole("dialog", { name: "Project Parameters" });
    await userEvent.type(within(dialog).getByLabelText("Name"), "Fire Rating");
    await userEvent.click(within(dialog).getByLabelText("Walls"));
    await userEvent.click(within(dialog).getByRole("button", { name: "Add Parameter" }));
    const call = fake.calls.find((c) => c.cmd === "add_project_parameter");
    expect(call?.args).toEqual({
      label: "Fire Rating",
      kind: "Text",
      typeScope: false,
      categories: ["Door", "Wall"],
    });
    expect(await within(dialog).findByText("Fire Rating")).toBeInTheDocument();
    await userEvent.click(within(dialog).getByRole("button", { name: "Remove Fire Rating" }));
    expect(fake.calls.some((c) => c.cmd === "remove_project_parameter")).toBe(true);
  });
});
