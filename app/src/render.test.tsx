import { beforeEach, describe, expect, it } from "vitest";
import { cleanup, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { App } from "./App";
import { runAction } from "./actions";
import { clock } from "./components/RenderDialog";
import { samePose } from "./components/View3D";
import { skyTexture, surfaceFor, toYUp } from "./render/pathtrace";
import { conflicts, DEFAULT_SHORTCUTS } from "./shortcuts";
import { useAppStore } from "./store";
import { promptFor, shortcut, toolAllowed } from "./tools";
import { appState, installFakeBackend } from "./test/fakeBackend";
import { ViewDialogs } from "./components/ViewDialogs";
import type { CameraPose } from "./bindings/CameraPose";

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
    viewDialog: null,
  });
  installFakeBackend();
});

async function openProject() {
  render(<App />);
  await userEvent.click(screen.getByRole("button", { name: "New Project" }));
  await screen.findByRole("toolbar", { name: "Tools" });
}

describe("Rendering tab: cameras and renders (ADR-027)", () => {
  it("comes after Architecture, with Camera for plans and Render for 3D", async () => {
    await openProject();
    const tabs = within(screen.getByRole("tablist", { name: "Ribbon tabs" }))
      .getAllByRole("tab")
      .map((t) => t.textContent);
    expect(tabs.slice(0, 3)).toEqual(["Site", "Architecture", "Rendering"]);
    await userEvent.click(screen.getByRole("tab", { name: "Rendering" }));
    // The project opens on a plan: Camera works, Render needs a 3D view.
    const camera = screen.getByRole("button", { name: "Camera" }) as HTMLButtonElement;
    const renderBtn = screen.getByRole("button", { name: "Render" }) as HTMLButtonElement;
    expect(camera.disabled).toBe(false);
    expect(renderBtn.disabled).toBe(true);
    await userEvent.click(camera);
    expect(useAppStore.getState().tool).toBe("camera");
    expect(screen.getByLabelText("Camera eye height")).toHaveProperty("value", `5' 6"`);
    expect(toolAllowed("camera", "Plan")).toBe(true);
    expect(toolAllowed("camera", "ThreeD")).toBe(false);
    expect(promptFor("camera", 0, "Plan")).toContain("eye point");
    expect(promptFor("camera", 1, "Plan")).toContain("target");
  });

  it("RR opens Render in a 3D view, with the sun at the site", async () => {
    expect(shortcut("R", "R").action).toBe("render");
    expect(conflicts(DEFAULT_SHORTCUTS)).toEqual([]);
    await openProject();
    await runAction("render");
    expect(useAppStore.getState().error).toContain("Open a 3D or camera view");
    // Only the dialogs: jsdom has no WebGL for the 3D view itself.
    cleanup();
    const app = appState(null);
    const v3d = app.views.find((v) => v.viewType === "ThreeD")!;
    useAppStore.setState({ app, activeView: v3d.id, openViews: [v3d.id], error: null });
    render(<ViewDialogs />);
    await runAction("render");
    const dialog = await screen.findByRole("dialog", { name: "Render" });
    expect(await within(dialog).findByText(/Sun 53° up, 211° from north/)).toBeTruthy();
    expect(within(dialog).getByRole("button", { name: "Save Image…" })).toHaveProperty(
      "disabled",
      true,
    );
    await userEvent.click(within(dialog).getByRole("button", { name: "Close" }));
    expect(screen.queryByRole("dialog", { name: "Render" })).toBeNull();
  });

  it("formats the sun's time, compares camera poses", () => {
    expect(clock(15.25)).toBe("3:15 PM");
    expect(clock(9)).toBe("9:00 AM");
    expect(clock(12.5)).toBe("12:30 PM");
    const a: CameraPose = { eye: [0, 0, 1676.4], target: [5000, 0, 1676.4], fov: 50 };
    expect(samePose(a, { ...a, eye: [0.4, 0, 1676.4] })).toBe(true);
    expect(samePose(a, { ...a, target: [5000, 30, 1676.4] })).toBe(false);
    expect(samePose(a, { ...a, fov: 45 })).toBe(false);
  });

  it("builds physical surfaces and a y-up sky", () => {
    expect(surfaceFor({ category: "Window", color: null, exterior: true }).transmission).toBe(1);
    expect(surfaceFor({ category: "Beam", color: null, exterior: true }).metalness).toBeGreaterThan(
      0,
    );
    expect(surfaceFor({ category: "Wall", color: [200, 100, 50], exterior: true }).color).toBe(
      0xc86432,
    );
    // z-up model space to three's y-up: north (+y) goes to -z.
    expect(toYUp([1, 2, 3]).toArray()).toEqual([1, 3, -2]);
    const sky = skyTexture({ dir: [0, -0.5, 0.8], altitude: 58, azimuth: 180 }, 8, 8);
    const d = sky.image.data as Float32Array;
    // Row 0 is straight down (the ground), the last row straight up (blue sky).
    const px = (row: number): [number, number, number] => [
      d[row * 8 * 4]!,
      d[row * 8 * 4 + 1]!,
      d[row * 8 * 4 + 2]!,
    ];
    const [gr, , gb] = px(0);
    const [sr, , sb] = px(7);
    expect(gb).toBeLessThan(0.4);
    expect(gr).toBeGreaterThan(gb - 0.05);
    expect(sb).toBeGreaterThan(sr * 2);
  });
});
