import { beforeEach, describe, expect, it } from "vitest";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { App } from "./App";
import { useAppStore } from "./store";
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
    standardsUi: { category: "sheet", item: 0, filter: "all" },
  });
  fake = installFakeBackend();
});

async function openStandards() {
  render(<App />);
  await userEvent.click(screen.getByRole("button", { name: "New Project" }));
  await screen.findByRole("toolbar", { name: "Tools" });
  await userEvent.click(screen.getByRole("tab", { name: "Standards" }));
  return screen.findByRole("complementary", { name: "Standards Browser" });
}

describe("Standards tab (ADR-047)", () => {
  it("comes after Project Info and replaces the browser, views and properties", async () => {
    render(<App />);
    await userEvent.click(screen.getByRole("button", { name: "New Project" }));
    const tabs = within(await screen.findByRole("tablist", { name: "Ribbon tabs" }))
      .getAllByRole("tab")
      .map((t) => t.textContent);
    expect(tabs.slice(0, 2)).toEqual(["Project Info", "Standards"]);
    await userEvent.click(screen.getByRole("tab", { name: "Standards" }));
    expect(await screen.findByRole("complementary", { name: "Standards Browser" })).toBeVisible();
    expect(screen.queryByRole("complementary", { name: "Project browser" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Modify" })).toBeNull();
    expect(
      await screen.findByText("1 of 3 standards defined", { selector: ".statusbar span" }),
    ).toBeInTheDocument();
  });

  it("lists a category's standards and filters to the undefined ones", async () => {
    await openStandards();
    expect(await screen.findByRole("heading", { name: "Sheet Setup" })).toBeInTheDocument();
    expect(screen.getByText("ARCH D", { selector: ".std-value > span" })).toBeInTheDocument();
    expect(screen.getByText("Not yet set")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("radio", { name: "UNDEFINED" }));
    expect(screen.queryByText("ARCH D", { selector: ".std-value > span" })).toBeNull();
    // The ribbon picks another category.
    const ribbon = screen.getByRole("toolbar", { name: "Tools" });
    await userEvent.click(within(ribbon).getByRole("button", { name: /Dims/ }));
    expect(await screen.findByRole("heading", { name: "Dimensions" })).toBeInTheDocument();
  });

  it("the checkbox marks a standard defined without picking the row", async () => {
    await openStandards();
    await screen.findByRole("heading", { name: "Sheet Setup" });
    await userEvent.click(screen.getByRole("checkbox", { name: "Revision block defined" }));
    const call = fake.calls.find((c) => c.cmd === "standards_set");
    expect(call?.args).toEqual({ category: "sheet", index: 1, value: null, done: true });
    expect(useAppStore.getState().standardsUi.item).toBe(1);
    await waitFor(() =>
      expect(
        screen.getByText("2 of 3 standards defined", { selector: ".statusbar span" }),
      ).toBeInTheDocument(),
    );
  });

  it("a value typed in Properties is saved and defines the standard", async () => {
    await openStandards();
    await userEvent.click(await screen.findByText("Revision block"));
    const props = screen.getByRole("complementary", { name: "Properties" });
    const box = within(props).getByLabelText("Office value");
    await userEvent.type(box, "Delta, lower right");
    box.blur();
    await waitFor(() =>
      expect(fake.calls.find((c) => c.cmd === "standards_set")?.args).toEqual({
        category: "sheet",
        index: 1,
        value: "Delta, lower right",
        done: null,
      }),
    );
    await waitFor(() =>
      expect(
        screen.getByText("Delta, lower right", { selector: ".std-value > span" }),
      ).toBeInTheDocument(),
    );
    expect(fake.standards.categories[0]!.items[1]!.done).toBe(true);
  });

  it("a tool shortcut goes back to the views", async () => {
    await openStandards();
    await userEvent.keyboard("wa");
    expect(useAppStore.getState().ribbonTab).toBe("Architecture");
    expect(await screen.findByRole("complementary", { name: "Project browser" })).toBeVisible();
  });

  it("a standard's value opens its choices, as a grid or a list (ADR-048)", async () => {
    try {
      localStorage.removeItem("rufplan.standards.choicesView");
    } catch {
      // jsdom has storage.
    }
    await openStandards();
    await screen.findByRole("heading", { name: "Sheet Setup" });
    await userEvent.click(screen.getAllByTitle("See the choices")[0]!);
    const dialog = await screen.findByRole("dialog", { name: "Sheet size choices" });
    const options = await within(dialog).findAllByRole("option");
    expect(options.map((o) => o.querySelector(".std-choice-label")?.textContent)).toEqual([
      "ARCH C",
      "ARCH D",
    ]);
    // Grid first; the toggle top right shows a list, remembered.
    const body = within(dialog).getByRole("listbox", { name: "Choices" });
    expect(body.className).toContain("grid");
    await userEvent.click(within(dialog).getByRole("radio", { name: "List" }));
    expect(body.className).toContain("list");
    expect(localStorage.getItem("rufplan.standards.choicesView")).toBe("list");
    // The current one is marked; picking another and Use this saves it.
    expect(options[1]!.textContent).toContain("CURRENT");
    const use = within(dialog).getByRole("button", { name: "Use this" });
    expect(use).toBeDisabled();
    await userEvent.click(options[0]!);
    await userEvent.click(use);
    expect(fake.calls.find((c) => c.cmd === "standards_set")?.args).toEqual({
      category: "sheet",
      index: 0,
      value: "ARCH C",
      done: null,
    });
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    // Properties opens it too, and Escape closes it.
    await userEvent.click(screen.getByRole("button", { name: "Choices…" }));
    expect(await screen.findByRole("dialog", { name: "Sheet size choices" })).toBeVisible();
    await userEvent.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
  });

  it("the Office Standard select loads a library", async () => {
    await openStandards();
    await userEvent.selectOptions(await screen.findByLabelText("Office Standard"), "Residential");
    expect(fake.calls.find((c) => c.cmd === "standards_load_library")?.args).toEqual({
      name: "Residential",
    });
    await waitFor(() =>
      expect(screen.getByText("ARCH C", { selector: ".std-value > span" })).toBeInTheDocument(),
    );
  });
});
