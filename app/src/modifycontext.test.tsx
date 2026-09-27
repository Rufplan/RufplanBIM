import { beforeEach, describe, expect, it } from "vitest";
import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { App } from "./App";
import { useAppStore } from "./store";
import { installFakeBackend, type FakeBackend } from "./test/fakeBackend";
import { contextLabel } from "./components/ModifyContext";

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
  fake = installFakeBackend();
});

async function openProject() {
  render(<App />);
  await userEvent.click(screen.getByRole("button", { name: "New Project" }));
  await screen.findByRole("toolbar", { name: "Tools" });
}

const select = (ids: string[]) => act(() => useAppStore.getState().select(ids));
const argsOf = (cmd: string) =>
  fake.calls.filter((c) => c.cmd === cmd).map((c) => c.args as Record<string, unknown>);

describe("contextual Modify tab (ADR-055)", () => {
  it("names the tab after the selection, as Revit does", () => {
    expect(contextLabel(["Floor"])).toBe("Modify | Floors");
    expect(contextLabel(["DetailLine", "ModelLine"])).toBe("Modify | Lines");
    expect(contextLabel(["Wall", "Door"])).toBe("Modify | Multi-Select");
    expect(contextLabel(["Material"])).toBeNull();
    expect(contextLabel([])).toBeNull();
  });

  it("selecting a floor opens Modify | Floors, and clearing it goes back", async () => {
    await openProject();
    fake.properties = { f1: { category: "Floor" } };
    select(["f1"]);
    const tab = await screen.findByRole("tab", { name: "Modify | Floors" });
    expect(tab).toHaveAttribute("aria-selected", "true");
    expect(tab.className).toContain("contextual");
    expect(screen.getByRole("button", { name: "Edit Boundary" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "Type Properties" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "Paint" })).toBeInTheDocument();
    // Tools for other categories stay out.
    expect(screen.queryByRole("button", { name: "Attach Top" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Tag" })).toBeNull();
    select([]);
    await waitFor(() =>
      expect(screen.getByRole("tab", { name: "Architecture" })).toHaveAttribute(
        "aria-selected",
        "true",
      ),
    );
    expect(screen.getByRole("tab", { name: "Modify" }).className).not.toContain("contextual");
  });

  it("a wall and a door together: Multi-Select, with both panels", async () => {
    await openProject();
    select(["w1", "d1"]);
    await screen.findByRole("tab", { name: "Modify | Multi-Select" });
    expect(screen.getByRole("button", { name: "Attach Top" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Tag" })).toBeInTheDocument();
    // Browse Types is for doors alone or windows alone.
    expect(screen.queryByRole("button", { name: "Browse Types" })).toBeNull();
  });

  it("Tag tags only the selection's taggable elements, in one step", async () => {
    await openProject();
    select(["w1", "d1"]);
    await userEvent.click(await screen.findByRole("button", { name: "Tag" }));
    await waitFor(() => expect(argsOf("tag_elements")).toHaveLength(1));
    expect(argsOf("tag_elements")[0]).toMatchObject({ targets: ["d1"] });
  });

  it("doors get Flip and Browse Types", async () => {
    await openProject();
    select(["d1"]);
    await screen.findByRole("tab", { name: "Modify | Doors" });
    await userEvent.click(screen.getByRole("button", { name: "Flip" }));
    expect(argsOf("flip_selection").at(-1)).toMatchObject({ ids: ["d1"] });
    await userEvent.click(screen.getByRole("button", { name: "Browse Types" }));
    expect(await screen.findByRole("dialog", { name: "Door Types" })).toBeInTheDocument();
  });

  it("lines get a Line Style that sets every selected line", async () => {
    await openProject();
    fake.properties = { l1: { category: "DetailLine" }, l2: { category: "ModelLine" } };
    select(["l1", "l2"]);
    await screen.findByRole("tab", { name: "Modify | Lines" });
    await userEvent.selectOptions(
      screen.getByRole("combobox", { name: "Selected Line Style" }),
      "Hidden",
    );
    await waitFor(() => expect(argsOf("set_property")).toHaveLength(2));
    expect(argsOf("set_property").map((a) => [a.id, a.key, a.value])).toEqual([
      ["l1", "style", "Hidden"],
      ["l2", "style", "Hidden"],
    ]);
  });

  it("a material or a type picked in the browser keeps the plain ribbon", async () => {
    await openProject();
    fake.properties = { m1: { category: "Material" } };
    select(["m1"]);
    await waitFor(() => expect(argsOf("selection_categories").length).toBeGreaterThan(0));
    expect(screen.getByRole("tab", { name: "Architecture" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
    expect(screen.queryByRole("tab", { name: /Modify \|/ })).toBeNull();
  });
});
