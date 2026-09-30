import { beforeEach, describe, expect, it } from "vitest";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { App } from "./App";
import { articleLabel, paraLabel, titleCase, useSpecs } from "./components/Specs";
import { useAppStore } from "./store";
import { fakeSection, installFakeBackend, type FakeBackend } from "./test/fakeBackend";

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
  useSpecs.setState({
    state: null,
    selected: null,
    draft: null,
    dirty: false,
    saving: false,
    query: "",
    collapsed: {},
    dialog: null,
    editOpen: false,
    log: [],
    note: null,
  });
  fake = installFakeBackend();
});

async function openSpecs() {
  render(<App />);
  await userEvent.click(screen.getByRole("button", { name: "New Project" }));
  await screen.findByRole("toolbar", { name: "Tools" });
  await userEvent.click(screen.getByRole("tab", { name: "Specifications" }));
  return screen.findByRole("complementary", { name: "Specifications Browser" });
}

async function generate() {
  await userEvent.click(await screen.findByRole("button", { name: "Generate Project Manual" }));
  const dialog = await screen.findByRole("dialog", { name: "Generate Project Manual" });
  await userEvent.click(within(dialog).getByRole("button", { name: "GENERATE" }));
  await waitFor(() => expect(useSpecs.getState().state?.book).not.toBeNull());
}

const lastCall = (cmd: string) =>
  [...fake.calls].reverse().find((c) => c.cmd === cmd)?.args as Record<string, unknown> | undefined;

describe("Specifications tab (ADR-085)", () => {
  it("comes after Sheets and starts with Generate", async () => {
    render(<App />);
    await userEvent.click(screen.getByRole("button", { name: "New Project" }));
    const tabs = within(await screen.findByRole("tablist", { name: "Ribbon tabs" }))
      .getAllByRole("tab")
      .map((t) => t.textContent);
    expect(tabs[tabs.indexOf("Sheets") + 1]).toBe("Specifications");
    await userEvent.click(screen.getByRole("tab", { name: "Specifications" }));
    expect(await screen.findByText("called for by this project")).toBeInTheDocument();
    expect(screen.queryByRole("complementary", { name: "Project browser" })).toBeNull();
  });

  it("generates the manual in the chosen style and opens a section", async () => {
    await openSpecs();
    await userEvent.click(await screen.findByRole("button", { name: "Generate Project Manual" }));
    const dialog = await screen.findByRole("dialog", { name: "Generate Project Manual" });
    await userEvent.click(within(dialog).getByRole("radio", { name: /Decimal Outline/ }));
    await userEvent.click(within(dialog).getByRole("button", { name: "GENERATE" }));
    await waitFor(() =>
      expect(lastCall("spec_generate")).toMatchObject({
        styleId: "decimal",
        issue: "Issued for Permit",
        date: "2026-10-01",
      }),
    );
    // The browser lists the sections by division; the first opens.
    const browser = screen.getByRole("complementary", { name: "Specifications Browser" });
    expect(await within(browser).findByText("Gypsum Board")).toBeInTheDocument();
    expect(await screen.findByDisplayValue("Section Includes:")).toBeInTheDocument();
  });

  it("edits a paragraph and saves the section after a pause", async () => {
    await openSpecs();
    await generate();
    const p = await screen.findByDisplayValue("Type X.");
    await userEvent.clear(p);
    await userEvent.type(p, "Type X, 5/8 inch.");
    await waitFor(() => expect(lastCall("spec_set_section")).toBeDefined(), { timeout: 2000 });
    const s = lastCall("spec_set_section")!.section as ReturnType<typeof fakeSection>;
    expect(s.parts[1]!.articles[0]!.paragraphs[0]!.text).toBe("Type X, 5/8 inch.");
  });

  it("Enter splits a paragraph, Tab indents it", async () => {
    await openSpecs();
    await generate();
    const p = (await screen.findByDisplayValue("Install.")) as HTMLTextAreaElement;
    p.focus();
    p.setSelectionRange(p.value.length, p.value.length);
    fireEvent.keyDown(p, { key: "Enter" });
    await waitFor(() =>
      expect(useSpecs.getState().draft!.parts[2]!.articles[0]!.paragraphs).toHaveLength(2),
    );
    const next = document.querySelector('[data-key="p2a0i1"]') as HTMLTextAreaElement;
    fireEvent.keyDown(next, { key: "Tab" });
    await waitFor(() =>
      expect(useSpecs.getState().draft!.parts[2]!.articles[0]!.paragraphs[1]!.level).toBe(1),
    );
  });

  it("adds library sections, recommended first", async () => {
    await openSpecs();
    await generate();
    await userEvent.click(screen.getByRole("button", { name: "Add" }));
    const dialog = await screen.findByRole("dialog", { name: "Add Sections" });
    await userEvent.click(within(dialog).getByText("Toilet Compartments"));
    await userEvent.click(within(dialog).getByRole("button", { name: /ADD 1/ }));
    await waitFor(() => expect(lastCall("spec_add_library")).toEqual({ numbers: ["10 21 13"] }));
  });

  it("previews Claude's edit as a diff and applies it", async () => {
    await openSpecs();
    await generate();
    fake.specPlan = {
      edit: {
        operations: [{ op: "replace_section", number: "09 29 00" }],
        summary: "Makes all board Type X",
        message: "",
      },
      changes: [
        {
          number: "09 29 00",
          title: "GYPSUM BOARD",
          kind: "changed",
          added: 1,
          removed: 1,
          lines: [
            { sign: "-", text: "Type X." },
            { sign: "+", text: "Type X, 5/8 inch, throughout." },
          ],
        },
      ],
      error: null,
    };
    await userEvent.click(screen.getByRole("button", { name: "Edit specs with Claude" }));
    const dialog = await screen.findByRole("dialog", { name: "Edit specs with Claude" });
    await userEvent.type(
      within(dialog).getByLabelText("Describe a change to the specifications"),
      "Make it all Type X{Enter}",
    );
    expect(await within(dialog).findByText("Type X, 5/8 inch, throughout.")).toBeInTheDocument();
    expect(lastCall("spec_edit_preview")).toMatchObject({ prompt: "Make it all Type X" });
    await userEvent.click(within(dialog).getByRole("button", { name: "APPLY" }));
    await waitFor(() => expect(lastCall("spec_edit_apply")).toBeDefined());
    expect(await within(dialog).findByText("Makes all board Type X")).toBeInTheDocument();
  });

  it("numbers like the styles", () => {
    expect(paraLabel("Csi", 0, 2, [])).toBe("B.");
    expect(paraLabel("Csi", 2, 1, [])).toBe("a.");
    expect(paraLabel("Decimal", 1, 2, [1, 3, 1, 2])).toBe("1.3.1.2");
    expect(articleLabel("CsiZero", 2, 4)).toBe("2.04");
    expect(titleCase("CAST-IN-PLACE CONCRETE")).toBe("Cast-in-Place Concrete");
    expect(titleCase("COMMON WORK RESULTS FOR HVAC")).toBe("Common Work Results for HVAC");
  });
});
