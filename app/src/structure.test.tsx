import { beforeEach, describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { App } from "./App";
import { useAppStore } from "./store";
import { shortcut, toolAllowed } from "./tools";
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

describe("structure and circulation (ADR-019)", () => {
  it("shortcuts and views for columns, beams and railings", () => {
    expect(shortcut("S", "C").tool).toBe("column");
    expect(shortcut("B", "M").tool).toBe("beam");
    expect(shortcut("R", "A").tool).toBe("railing");
    // CL stays Ceiling (Revit's column shortcut would clash).
    expect(shortcut("C", "L").tool).toBe("ceilingAuto");
    for (const t of ["column", "beam", "railing"] as const) {
      expect(toolAllowed(t, "Plan")).toBe(true);
      expect(toolAllowed(t, "Elevation")).toBe(false);
    }
  });

  it("the Structure tab places columns at grids and picks a default steel type", async () => {
    await openProject();
    expect(useAppStore.getState().toolTypes.column).toBe("00000000-0000-7000-8000-000000000025");
    await userEvent.click(screen.getByRole("tab", { name: "Structure" }));
    await userEvent.click(screen.getByRole("button", { name: "At Grids" }));
    const call = fake.calls.find((c) => c.cmd === "columns_at_grids");
    expect(call?.args).toMatchObject({
      view: FAKE_IDS.plan1,
      typeId: "00000000-0000-7000-8000-000000000025",
    });
    await userEvent.click(screen.getByRole("button", { name: "Beam" }));
    expect(useAppStore.getState().tool).toBe("beam");
    // The properties panel shows the beam type the tool will place.
    expect(await screen.findByText("Beam — Type")).toBeTruthy();
  });

  it("the options bar sets the wall location line and the stair shape", async () => {
    await openProject();
    useAppStore.getState().setTool("wall");
    const loc = await screen.findByRole("combobox", { name: "Location Line" });
    await userEvent.selectOptions(loc, "FinishExterior");
    expect(useAppStore.getState().options.wallLocation).toBe("FinishExterior");
    useAppStore.getState().setTool("stair");
    const shape = await screen.findByRole("combobox", { name: "Shape" });
    await userEvent.selectOptions(shape, "l-left");
    expect(useAppStore.getState().options.stairShape).toBe("l-left");
  });

  it("Attach Top sends the selected walls", async () => {
    await openProject();
    useAppStore.getState().select(["00000000-0000-7000-8000-0000000000aa"]);
    await userEvent.click(screen.getByRole("tab", { name: "Modify" }));
    await userEvent.click(screen.getByRole("button", { name: "Attach Top" }));
    expect(fake.calls.find((c) => c.cmd === "attach_wall_tops")?.args).toMatchObject({
      ids: ["00000000-0000-7000-8000-0000000000aa"],
      attach: true,
    });
  });
});
