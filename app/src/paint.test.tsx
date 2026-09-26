import { beforeEach, describe, expect, it } from "vitest";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { App } from "./App";
import { paintElement } from "./actions";
import { useAppStore } from "./store";
import { toolAllowed } from "./tools";
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
    picker: null,
    paintMaterial: null,
  });
  fake = installFakeBackend();
});

const calls = (cmd: string) =>
  fake.calls.filter((c) => c.cmd === cmd).map((c) => c.args as Record<string, unknown>);

describe("Paint (ADR-034)", () => {
  it("picks a material in the browser, then paints what you click", async () => {
    render(<App />);
    await userEvent.click(screen.getByRole("button", { name: "New Project" }));
    await screen.findByRole("toolbar", { name: "Tools" });
    await userEvent.click(screen.getByRole("tab", { name: "Materials" }));
    await userEvent.click(screen.getAllByRole("button", { name: /Material Browser|Browse/ })[0]!);
    const dialog = await screen.findByRole("dialog", { name: "Material Browser" });
    await userEvent.click(await within(dialog).findByText("White Oak Plank Flooring, Matte"));
    await userEvent.click(within(dialog).getByRole("button", { name: "Paint" }));
    // The browser closes, the brush is armed and the chip says with what.
    expect(screen.queryByRole("dialog", { name: "Material Browser" })).toBeNull();
    const s = useAppStore.getState();
    expect(s.tool).toBe("paint");
    expect(s.paintMaterial).toBe("00000000-0000-7000-8000-0000000000a1");
    const chip = await screen.findByRole("status", { name: "Paint" });
    expect(within(chip).getByText(/Shift-click paints the whole type/)).toBeTruthy();
    // A click paints that element only; Shift-click its whole type.
    await paintElement("00000000-0000-7000-8000-00000000c0c0", false);
    expect(calls("paint_elements").at(-1)).toEqual({
      ids: ["00000000-0000-7000-8000-00000000c0c0"],
      material: "00000000-0000-7000-8000-0000000000a1",
    });
    await paintElement("00000000-0000-7000-8000-00000000c0c0", true);
    expect(calls("apply_material").at(-1)).toEqual({
      ids: ["00000000-0000-7000-8000-00000000c0c0"],
      material: "00000000-0000-7000-8000-0000000000a1",
    });
    await userEvent.click(within(chip).getByRole("button", { name: "Done" }));
    expect(useAppStore.getState().tool).toBe("select");
  });

  it("works in plans, elevations and 3D, not on sheets", () => {
    expect(toolAllowed("paint", "Plan")).toBe(true);
    expect(toolAllowed("paint", "Elevation")).toBe(true);
    expect(toolAllowed("paint", "ThreeD")).toBe(true);
    expect(toolAllowed("paint", "Sheet")).toBe(false);
  });
});
