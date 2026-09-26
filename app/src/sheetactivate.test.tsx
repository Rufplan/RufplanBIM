import { beforeEach, describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { modelToSheetCam, sheetToModelCam } from "./components/ViewCanvas";
import { Workspace } from "./components/Workspace";
import { toScreen } from "./canvas/render";
import { activeViewInfo, useAppStore } from "./store";
import { appState, FAKE_IDS, installFakeBackend } from "./test/fakeBackend";

const SHEET = "00000000-0000-7000-8000-0000000000a1";
const VP = "00000000-0000-7000-8000-0000000000a2";

function withSheet() {
  const app = appState(null);
  const plan = app.views.find((v) => v.id === FAKE_IDS.plan1)!;
  app.views.push({ ...plan, id: SHEET, name: "A-101 - Floor Plan", viewType: "Sheet" });
  return app;
}

beforeEach(() => {
  installFakeBackend();
  useAppStore.setState({
    app: withSheet(),
    openViews: [SHEET, FAKE_IDS.north],
    activeView: SHEET,
    activeViewport: null,
    selection: [],
    tool: "select",
  });
});

describe("activating a view on a sheet (ADR-039)", () => {
  it("keeps the drawing where the sheet showed it", () => {
    // A 1/4" plan (48) placed at (400, 300) on the sheet; its drawing spans 0..9600 mm.
    const bounds: [number, number, number, number] = [0, 0, 9600, 7200];
    const center = { x: 400, y: 300 };
    const sheetCam = { cx: 420, cy: 310, zoom: 2 };
    const cam = sheetToModelCam(sheetCam, center, bounds, 48);
    // A model point lands on screen where its paper point did.
    const model = { x: 1200, y: 6000 };
    const paper = { x: 400 + (1200 - 4800) / 48, y: 300 + (6000 - 3600) / 48 };
    const a = toScreen(cam, 1000, 700, model.x, model.y);
    const b = toScreen(sheetCam, 1000, 700, paper.x, paper.y);
    expect(a[0]).toBeCloseTo(b[0], 9);
    expect(a[1]).toBeCloseTo(b[1], 9);
    // And back: panning the view pans the sheet.
    const back = modelToSheetCam(cam, center, bounds, 48);
    expect(back.cx).toBeCloseTo(420, 9);
    expect(back.cy).toBeCloseTo(310, 9);
    expect(back.zoom).toBeCloseTo(2, 9);
  });

  it("works in the view until deactivated or another view opens", async () => {
    const s = useAppStore.getState();
    s.activateViewport({
      sheet: SHEET,
      viewport: VP,
      view: FAKE_IDS.plan1,
      center: { x: 1, y: 2 },
    });
    // The ribbon, properties and tools see the activated view.
    expect(activeViewInfo(useAppStore.getState())?.id).toBe(FAKE_IDS.plan1);
    render(<Workspace />);
    expect(screen.getByRole("status").textContent).toContain("Working in Level 1 on the sheet");
    // The sheet's tab stays the active one.
    expect(screen.getByRole("tab", { selected: true }).textContent).toContain("A-101");
    await userEvent.click(screen.getByRole("button", { name: "Deactivate View" }));
    expect(useAppStore.getState().activeViewport).toBeNull();
    expect(activeViewInfo(useAppStore.getState())?.id).toBe(SHEET);
    expect(screen.queryByRole("button", { name: "Deactivate View" })).toBeNull();
    // Opening another view deactivates it too.
    s.activateViewport({
      sheet: SHEET,
      viewport: VP,
      view: FAKE_IDS.plan1,
      center: { x: 1, y: 2 },
    });
    useAppStore.getState().openView(FAKE_IDS.north);
    expect(useAppStore.getState().activeViewport).toBeNull();
  });
});
