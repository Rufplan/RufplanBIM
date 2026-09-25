import { beforeEach, describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { App } from "./App";
import { useAppStore } from "./store";
import { shortcut, sketchPrompt, toolAllowed } from "./tools";
import { FAKE_IDS, installFakeBackend, type FakeBackend } from "./test/fakeBackend";

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

describe("floor boundary sketch mode (ADR-021)", () => {
  it("Floor enters sketch mode with Pick Walls and Revit's contextual tab", async () => {
    await openProject();
    await userEvent.click(screen.getByRole("button", { name: "Floor" }));
    expect(fake.calls.find((c) => c.cmd === "sketch_begin")?.args).toMatchObject({
      view: FAKE_IDS.plan1,
      kind: "Floor",
      target: null,
    });
    expect(await screen.findByRole("tab", { name: "Modify | Create Floor Boundary" })).toBeTruthy();
    expect(useAppStore.getState().tool).toBe("sketch");
    expect(screen.getByRole("button", { name: "Pick Walls" }).getAttribute("aria-pressed")).toBe(
      "true",
    );
    // The options bar shows Pick Walls' settings.
    expect(screen.getByRole("checkbox", { name: "Extend into wall (to core)" })).toBeTruthy();
    await userEvent.click(screen.getByRole("button", { name: "Rectangle" }));
    expect(useAppStore.getState().sketchUi.mode).toBe("Rectangle");
    expect(screen.getByRole("checkbox", { name: "Radius" })).toBeTruthy();
  });

  it("Finish reports open loops; Cancel leaves sketch mode", async () => {
    await openProject();
    await userEvent.keyboard("SB");
    await screen.findByRole("tab", { name: "Modify | Create Floor Boundary" });
    await userEvent.click(screen.getByRole("button", { name: "Finish" }));
    expect((await screen.findByRole("alert")).textContent).toContain(
      "Lines must be in closed loops",
    );
    expect(useAppStore.getState().tool).toBe("sketch");
    await userEvent.click(screen.getByRole("button", { name: "Cancel" }));
    expect(await screen.findByRole("tab", { name: "Architecture" })).toBeTruthy();
    expect(useAppStore.getState().tool).toBe("select");
  });

  it("Esc steps out of a draw tool but never out of the sketch", async () => {
    await openProject();
    await userEvent.keyboard("SB");
    await screen.findByRole("tab", { name: "Modify | Create Floor Boundary" });
    await userEvent.keyboard("{Escape}");
    expect(useAppStore.getState().sketchUi.mode).toBe("Modify");
    await userEvent.keyboard("{Escape}");
    expect(useAppStore.getState().tool).toBe("sketch");
  });

  it("prompts, shortcuts and the Elevation tool", async () => {
    expect(sketchPrompt("Line", 0)).toBe("Click to enter line start point.");
    expect(sketchPrompt("StartEndRadiusArc", 2)).toContain("radius");
    expect(sketchPrompt("PickWalls", 0)).toContain("Tab");
    expect(shortcut("E", "L").tool).toBe("elevation");
    expect(toolAllowed("elevation", "Plan")).toBe(true);
    expect(toolAllowed("elevation", "Section")).toBe(false);
    await openProject();
    await userEvent.keyboard("EL");
    expect(useAppStore.getState().tool).toBe("elevation");
    const type = await screen.findByRole("combobox", { name: "Elevation type" });
    await userEvent.selectOptions(type, "building");
    expect(useAppStore.getState().elevationInterior).toBe(false);
  });
});
