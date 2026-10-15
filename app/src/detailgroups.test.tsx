import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { App } from "./App";
import { useAppStore } from "./store";
import { categoryLabel, groupOf, reloadLibrary } from "./components/DetailLibrary";
import { FAKE_DETAILS, installFakeBackend } from "./test/fakeBackend";

// Door and window details (ADR-122, ADR-123) as the library lists them.
const MORE = [
  ["door-ext-wood-2x6-head", "Exterior Door - Wood Frame & Siding - Head", "Doors - Typical"],
  ["marvin-uifd-head", "Marvin Ultimate Inswing French Door G2 - Head", "Doors - Hinged Patio"],
  ["marvin-uifd-sill", "Marvin Ultimate Inswing French Door G2 - Sill", "Doors - Hinged Patio"],
  ["window-stucco-head", "Window - Stucco - Head", "Windows - Typical"],
].map(([id, name, category]) => ({
  id: id!,
  name: name!,
  category: category!,
  scale: 4,
  scaleLabel: `3" = 1'-0"`,
  description: "",
  user: false,
}));

beforeEach(() => {
  FAKE_DETAILS.push(...MORE);
  reloadLibrary();
  useAppStore.setState({ app: null, openViews: [], activeView: null, viewDialog: null });
  installFakeBackend();
});
afterEach(() => {
  FAKE_DETAILS.splice(FAKE_DETAILS.length - MORE.length, MORE.length);
  reloadLibrary();
});

describe("Detail Library groups (ADR-122, ADR-123)", () => {
  it("reads a door or window category as its group and type", () => {
    expect(groupOf("Doors - Hinged Patio")).toEqual({ group: "Doors", sub: "Hinged Patio" });
    expect(groupOf("Roofs")).toEqual({ group: "Roofs", sub: null });
    expect(categoryLabel("Windows - Typical")).toBe("Windows › Typical");
    expect(categoryLabel("Roofs")).toBe("Roofs");
  });

  it("lists Doors and Windows once each in the sidebar, their types as tabs", async () => {
    render(<App />);
    await userEvent.click(screen.getByRole("button", { name: "New Project" }));
    await screen.findByRole("toolbar", { name: "Tools" });
    await userEvent.click(screen.getByRole("tab", { name: "Details" }));
    await userEvent.click(screen.getByRole("button", { name: /Detail Library/ }));
    const dialog = await screen.findByRole("dialog", { name: "Detail Library" });
    const nav = within(dialog).getByRole("navigation", { name: "Categories" });
    await within(nav).findByRole("button", { name: /^Doors/ });
    const cats = within(nav)
      .getAllByRole("button")
      .map((b) => b.textContent);
    expect(cats).toEqual(["All7", "Foundations1", "Openings1", "Roofs1", "Doors3", "Windows1"]);
    // No tabs until a grouped category is chosen.
    expect(within(dialog).queryByRole("tablist", { name: "Doors types" })).toBeNull();
    await userEvent.click(within(nav).getByRole("button", { name: /^Doors/ }));
    const tabs = within(dialog).getByRole("tablist", { name: "Doors types" });
    expect(
      within(tabs)
        .getAllByRole("tab")
        .map((t) => t.textContent),
    ).toEqual(["All doors3", "Typical1", "Hinged Patio2"]);
    const grid = within(dialog).getByRole("list", { name: "Details" });
    expect(within(grid).getAllByRole("listitem")).toHaveLength(3);
    await userEvent.click(within(tabs).getByRole("tab", { name: /Hinged Patio/ }));
    expect(within(grid).getAllByRole("listitem")).toHaveLength(2);
    // The chosen detail's category reads as group and type.
    await userEvent.click(within(grid).getAllByRole("listitem")[0]!);
    expect(within(dialog).getByText("Doors › Hinged Patio")).toBeInTheDocument();
    // Choosing another category resets the tab, and an empty search says so.
    await userEvent.click(within(nav).getByRole("button", { name: /^Windows/ }));
    expect(within(dialog).getByRole("tab", { name: /All windows/ })).toHaveAttribute(
      "aria-selected",
      "true",
    );
    await userEvent.type(within(dialog).getByLabelText("Search details"), "zzz");
    expect(within(dialog).getByText("No details match “zzz”.")).toBeInTheDocument();
  });
});
