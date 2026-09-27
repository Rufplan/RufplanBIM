import { beforeEach, describe, expect, it } from "vitest";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { App } from "./App";
import { useAppStore } from "./store";
import { installFakeBackend } from "./test/fakeBackend";
import { lineOptions } from "./lines";

beforeEach(() => {
  useAppStore.setState({
    app: null,
    error: null,
    openViews: [],
    activeView: null,
    selection: [],
    tool: "select",
    lineUi: { mode: "Line", style: "Thin", chain: true, sides: 6 },
  });
  installFakeBackend();
});

async function openProject() {
  render(<App />);
  await userEvent.click(screen.getByRole("button", { name: "New Project" }));
  await screen.findByRole("toolbar", { name: "Tools" });
}

describe("Detail and model lines (ADR-054)", () => {
  it("Detail Line on the Annotate tab, with line styles and draw modes", async () => {
    await openProject();
    await userEvent.click(screen.getByRole("tab", { name: "Annotate" }));
    await userEvent.click(screen.getByRole("button", { name: /Detail Line/ }));
    expect(useAppStore.getState().tool).toBe("detailLine");
    const style = screen.getByRole("combobox", { name: "Line Style" });
    expect(
      within(style)
        .getAllByRole("option")
        .map((o) => o.textContent),
    ).toEqual([
      "Thin Lines",
      "Medium Lines",
      "Wide Lines",
      "<Hidden>",
      "<Centerline>",
      "<Overhead>",
      "<Demolished>",
      "<Beyond>",
    ]);
    await userEvent.selectOptions(style, "Hidden");
    expect(useAppStore.getState().lineUi.style).toBe("Hidden");
    const draw = screen.getByRole("radiogroup", { name: "Draw" });
    expect(within(draw).getAllByRole("radio")).toHaveLength(7);
    await userEvent.click(within(draw).getByRole("radio", { name: "Inscribed Polygon" }));
    expect(useAppStore.getState().lineUi.mode).toBe("InscribedPolygon");
    expect(screen.getByLabelText("Sides")).toBeInTheDocument();
    expect(screen.queryByLabelText("Chain")).toBeNull();
  });

  it("Model Line on the Architecture tab and the LI shortcut", async () => {
    await openProject();
    expect(screen.getByRole("button", { name: /Model Line/ })).toBeEnabled();
    await userEvent.keyboard("li");
    expect(useAppStore.getState().tool).toBe("modelLine");
    await userEvent.keyboard("{Escape}dl");
    expect(useAppStore.getState().tool).toBe("detailLine");
    expect(lineOptions({ mode: "Line", sides: 0 })).toEqual({ offset: 0, radius: null, sides: 6 });
  });
});
