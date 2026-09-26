import { beforeEach, describe, expect, it } from "vitest";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { App } from "./App";
import { BUILDING_TYPES } from "./components/GenerateDialog";
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

async function openProject() {
  render(<App />);
  await userEvent.click(screen.getByRole("button", { name: "New Project" }));
  await screen.findByRole("toolbar", { name: "Tools" });
}

describe("Generate with Claude (ADR-030)", () => {
  it("asks for the Claude key, sends the brief, and reports what was built", async () => {
    await openProject();
    await userEvent.click(screen.getByRole("tab", { name: "Architecture" }));
    await userEvent.click(screen.getByRole("button", { name: "Generate" }));
    const dialog = await screen.findByRole("dialog", { name: "Generate with Claude" });
    // No key yet: Generate waits for one.
    const key = await within(dialog).findByLabelText("Claude API key");
    const go = within(dialog).getByRole("button", { name: "Generate" }) as HTMLButtonElement;
    expect(go.disabled).toBe(true);
    await userEvent.type(key, "sk-ant-test");
    await userEvent.click(within(dialog).getByRole("button", { name: "Save Key" }));
    expect(fake.claudeKey).toBe(true);
    expect(within(dialog).queryByLabelText("Claude API key")).toBeNull();
    // The brief: type (which sets stories and the count field), style, roof, references,
    // prompt, model.
    await userEvent.selectOptions(within(dialog).getByLabelText("Building type"), "6");
    expect((within(dialog).getByLabelText("Stories") as HTMLInputElement).value).toBe("4");
    expect(within(dialog).getByLabelText("Keys")).toBeTruthy();
    await userEvent.selectOptions(within(dialog).getByLabelText("Building type"), "0");
    await userEvent.clear(within(dialog).getByLabelText("Bedrooms"));
    await userEvent.type(within(dialog).getByLabelText("Bedrooms"), "4");
    await userEvent.type(within(dialog).getByLabelText("Area"), "3,200");
    await userEvent.selectOptions(within(dialog).getByLabelText("Style"), "Craftsman");
    await userEvent.selectOptions(within(dialog).getByLabelText("Roof"), "gable");
    await userEvent.type(within(dialog).getByLabelText("References"), "deep front porch");
    await userEvent.type(
      within(dialog).getByLabelText("Prompt"),
      "primary suite on the main floor",
    );
    await userEvent.selectOptions(within(dialog).getByLabelText("Model"), "claude-sonnet-5");
    // No lot set: fitting to one is off.
    expect((within(dialog).getByRole("checkbox") as HTMLInputElement).disabled).toBe(true);
    await userEvent.click(within(dialog).getByRole("button", { name: "Generate" }));
    expect(fake.generated).toMatchObject({
      buildingType: "Single-family house",
      stories: 2,
      area: 3200,
      count: 4,
      bathrooms: 2.5,
      style: "Craftsman",
      roof: "gable",
      fitLot: false,
      references: "deep front porch",
      prompt: "primary suite on the main floor",
      images: [],
      model: "claude-sonnet-5",
    });
    // The result: summary, counts, warnings, and a way to see it.
    const status = await within(dialog).findByText("Oak Hollow");
    expect(status).toBeTruthy();
    expect(within(dialog).getByText(/three-bedroom modern farmhouse/)).toBeTruthy();
    expect(within(dialog).getByText(/32 walls · 12 doors · 18 windows/)).toBeTruthy();
    expect(within(dialog).getByText(/Loft has no route in/)).toBeTruthy();
    expect(within(dialog).getByRole("button", { name: "Open 3D View" })).toBeTruthy();
    await userEvent.click(within(dialog).getByRole("button", { name: "Revise & Generate Again" }));
    expect(within(dialog).getByLabelText("Prompt")).toHaveProperty(
      "value",
      "primary suite on the main floor",
    );
  });

  it("offers the US building types with the right counts", () => {
    expect(BUILDING_TYPES.map((t) => t.label)).toContain("Garden-style apartments");
    expect(BUILDING_TYPES.find((t) => t.label === "Boutique hotel")?.count).toBe("Keys");
    expect(BUILDING_TYPES.find((t) => t.label === "Single-family house")?.count).toBe("Bedrooms");
  });
});
