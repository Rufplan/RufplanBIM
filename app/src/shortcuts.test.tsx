import { beforeEach, describe, expect, it } from "vitest";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { App } from "./App";
import { runAction } from "./actions";
import { activeShortcuts, conflicts, DEFAULT_SHORTCUTS, keyMap, saveOverrides } from "./shortcuts";
import { useAppStore } from "./store";
import { shortcut } from "./tools";
import { installFakeBackend, type FakeBackend } from "./test/fakeBackend";

let fake: FakeBackend;

beforeEach(() => {
  localStorage.clear();
  useAppStore.setState({
    app: null,
    error: null,
    confirm: null,
    openViews: [],
    activeView: null,
    selection: [],
    tool: "select",
    snapOverride: null,
    tempHide: {},
    viewDialog: null,
  });
  fake = installFakeBackend();
});

async function openProject() {
  render(<App />);
  await userEvent.click(screen.getByRole("button", { name: "New Project" }));
  await screen.findByRole("toolbar", { name: "Tools" });
}

describe("Revit keyboard shortcuts (ADR-024)", () => {
  it("ships Revit's defaults without clashes", () => {
    expect(conflicts(DEFAULT_SHORTCUTS)).toEqual([]);
    const m = keyMap(DEFAULT_SHORTCUTS);
    expect(m.get("AL")).toEqual({ tool: "align" });
    expect(m.get("TR")).toEqual({ tool: "trim" });
    expect(m.get("PN")).toEqual({ action: "pin" });
    expect(m.get("HH")).toEqual({ action: "hideElement" });
    expect(m.get("VG")).toEqual({ action: "visibility" });
    expect(m.get("ZA")).toEqual({ action: "zoomFit" });
    expect(m.get("SE")).toEqual({ action: "snapEndpoint" });
    expect(shortcut("S", "B").action).toBe("floor");
  });

  it("applies the owner's overrides", () => {
    saveOverrides({ align: ["AA"] });
    expect(shortcut("A", "A").tool).toBe("align");
    expect(shortcut("A", "L").tool).toBe(null);
    const defs = activeShortcuts({ align: ["WA"] });
    expect(conflicts(defs)).toEqual(["WA"]);
  });

  it("pins, selects all instances and hides temporarily", async () => {
    await openProject();
    useAppStore.setState({ selection: ["w1"] });
    await runAction("pin");
    expect(fake.pinned).toEqual(["w1"]);
    await runAction("selectAll");
    expect(useAppStore.getState().selection).toEqual(["w1", "w2", "w3"]);
    const view = useAppStore.getState().activeView!;
    await runAction("isolateElement");
    expect(useAppStore.getState().tempHide[view]).toEqual({
      isolate: true,
      ids: ["w1", "w2", "w3"],
      categories: [],
    });
    await runAction("resetTemporary");
    expect(useAppStore.getState().tempHide[view]).toBeUndefined();
  });

  it("sets a one-pick snap override", async () => {
    await runAction("snapMidpoint");
    expect(useAppStore.getState().snapOverride).toBe("Midpoint");
    await runAction("snapOverrideOff");
    expect(useAppStore.getState().snapOverride).toBe(null);
  });

  it("KS opens the Keyboard Shortcuts dialog and saves a change", async () => {
    await openProject();
    await userEvent.keyboard("ks");
    const input = await screen.findByLabelText("Keys for Align");
    await userEvent.clear(input);
    await userEvent.type(input, "AA");
    await userEvent.tab();
    const dialog = screen.getByRole("dialog", { name: "Keyboard Shortcuts" });
    await userEvent.click(within(dialog).getByRole("button", { name: "Save" }));
    expect(JSON.parse(localStorage.getItem("rufplan.shortcuts")!)).toEqual({ align: ["AA"] });
    expect(screen.queryByRole("dialog", { name: "Keyboard Shortcuts" })).toBeNull();
  });
});
