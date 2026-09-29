import { beforeEach, describe, expect, it } from "vitest";
import { useState } from "react";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { App } from "./App";
import { runAction } from "./actions";
import { DetailLevelToggle, DETAIL_LEVELS } from "./components/DetailLevelToggle";
import { conflicts, DEFAULT_SHORTCUTS } from "./shortcuts";
import { shortcut } from "./tools";
import { activeViewInfo, useAppStore } from "./store";
import { appState, installFakeBackend, type FakeBackend } from "./test/fakeBackend";
import type { DetailLevel } from "./bindings/DetailLevel";

let fake: FakeBackend;
beforeEach(() => {
  useAppStore.setState({
    app: null,
    error: null,
    openViews: [],
    activeView: null,
    selection: [],
    tool: "select",
    picker: null,
    viewDialog: null,
  });
  fake = installFakeBackend();
});

function Harness() {
  const [v, setV] = useState<DetailLevel>("Fine");
  return <DetailLevelToggle value={v} onChange={setV} />;
}

describe("Detail Level (ADR-067)", () => {
  it("the pill shows the current level and opens to Coarse, Medium and Fine", async () => {
    render(<Harness />);
    const group = screen.getByRole("radiogroup", { name: "Detail Level" });
    expect(screen.getAllByRole("radio").map((b) => b.getAttribute("aria-label"))).toEqual(["Fine"]);
    expect(screen.getByRole("radio", { name: "Fine" }).getAttribute("title")).toBe("Fine (DF)");
    await userEvent.hover(group);
    expect(screen.getAllByRole("radio").map((b) => b.getAttribute("aria-label"))).toEqual(
      DETAIL_LEVELS.map((d) => d.label),
    );
    await userEvent.click(screen.getByRole("radio", { name: "Coarse" }));
    expect(screen.getByRole("radio", { name: "Coarse" }).getAttribute("aria-checked")).toBe("true");
    // Arrow keys step through the levels.
    screen.getByRole("radio", { name: "Coarse" }).focus();
    await userEvent.keyboard("{ArrowRight}");
    expect(screen.getByRole("radio", { name: "Medium" }).getAttribute("aria-checked")).toBe("true");
    await userEvent.unhover(group);
    screen.getByRole("radio", { name: "Medium" }).blur();
    await waitFor(() =>
      expect(screen.getAllByRole("radio").map((b) => b.getAttribute("aria-label"))).toEqual([
        "Medium",
      ]),
    );
  });

  it("sits in plans and sets the view's Detail Level; DC, DD and DF too", async () => {
    render(<App />);
    await userEvent.click(screen.getByRole("button", { name: "New Project" }));
    await screen.findByRole("toolbar", { name: "Tools" });
    const group = await screen.findByRole("radiogroup", { name: "Detail Level" });
    await userEvent.hover(group);
    await userEvent.click(within(group).getByRole("radio", { name: "Medium" }));
    const sets = () =>
      fake.calls
        .filter((c) => c.cmd === "set_property")
        .map((c) => (c.args as { key: string; value: string }).value);
    await waitFor(() => expect(sets()).toEqual(["Medium"]));
    await waitFor(() => expect(activeViewInfo(useAppStore.getState())?.detailLevel).toBe("Medium"));
    // The Revit-style two-letter shortcuts (ours: Revit has none).
    expect(shortcut("D", "C").action).toBe("detailCoarse");
    expect(shortcut("D", "F").action).toBe("detailFine");
    expect(conflicts(DEFAULT_SHORTCUTS)).toEqual([]);
    await runAction("detailCoarse");
    expect(activeViewInfo(useAppStore.getState())?.detailLevel).toBe("Coarse");
    // Choosing the level a view already has changes nothing.
    await runAction("detailCoarse");
    expect(sets()).toEqual(["Medium", "Coarse"]);
  });

  it("3D views have none (they use Visual Style)", async () => {
    const app = appState(null);
    const v3d = app.views.find((v) => v.viewType === "ThreeD")!;
    expect(v3d.detailLevel).toBeNull();
    useAppStore.setState({ app, activeView: v3d.id, openViews: [v3d.id] });
    await runAction("detailFine");
    expect(useAppStore.getState().error).toContain("Detail Level applies to");
  });
});
