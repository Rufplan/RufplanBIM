import { beforeEach, describe, expect, it } from "vitest";
import { cleanup, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { App } from "./App";
import { runAction } from "./actions";
import { clock } from "./components/RenderDialog";
import { samePose } from "./components/View3D";
import * as THREE from "three";
import { BACKGROUNDS } from "./render/backgrounds";
import {
  equirectUv,
  horizontalIrradiance,
  projectedGround,
  surfaceFor,
  toYUp,
} from "./render/pathtrace";
import { physicalSky, sunColor } from "./render/sky";
import { conflicts, DEFAULT_SHORTCUTS } from "./shortcuts";
import { useAppStore } from "./store";
import { POINT_TOOLS, promptFor, shortcut, toolAllowed } from "./tools";
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
    expect(tabs.slice(0, 4)).toEqual(["Site", "Architecture", "Materials", "Rendering"]);
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
    // Preetham sky with the sun: blue overhead, a ground below, the sun far brighter.
    const sunDir = toYUp([0, -0.5, 0.8]).normalize();
    const sky = physicalSky({ sunDir, altitude: 58, width: 256, height: 128 });
    const d = sky.image.data as Float32Array;
    const at = (row: number, col: number): [number, number, number] => {
      const k = (row * 256 + col) * 4;
      return [d[k]!, d[k + 1]!, d[k + 2]!];
    };
    const [zr, , zb] = at(127, 0);
    expect(zb).toBeGreaterThan(zr * 1.5);
    const [gr, gg, gb] = at(0, 0);
    expect(gr + gg + gb).toBeLessThan(zr + zb + 1);
    let peak = 0;
    for (let k = 0; k < d.length; k += 4) peak = Math.max(peak, d[k + 1]!);
    expect(peak).toBeGreaterThan(100 * (zb + 0.01));
    // Low sun is orange; high sun white.
    const low = sunColor(3);
    expect(low[2]).toBeLessThan(0.6);
    expect(sunColor(80)[2]).toBeGreaterThan(0.85);
    // Irradiance counts only the sky half, whichever way the rows run.
    // A uniform sky of radiance 1 over a black ground gives π on a horizontal surface.
    const rows = 64;
    const px = new Float32Array(4 * rows * 4);
    for (let j = rows / 2; j < rows; j++)
      for (let i = 0; i < 4; i++) px.fill(1, (j * 4 + i) * 4, (j * 4 + i) * 4 + 4);
    const up = new THREE.DataTexture(px, 4, rows);
    expect(horizontalIrradiance(up)).toBeCloseTo(Math.PI, 2);
    up.flipY = true;
    expect(horizontalIrradiance(up)).toBe(0);
  });

  it("offers photo backgrounds and projects their ground like V-Ray (ADR-028)", async () => {
    expect(BACKGROUNDS.map((b) => b.label)).toEqual([
      "Sky",
      "Mountains",
      "Grass Plain",
      "City",
      "Physical Sky (matches the sun)",
      "White",
    ]);
    expect(BACKGROUNDS.filter((b) => b.project).map((b) => b.id)).toEqual([
      "mountains",
      "grass",
      "city",
    ]);
    // Equirect mapping matches three: +x is u 0.5, straight down is v 0; rotation turns it.
    expect(equirectUv(new THREE.Vector3(1, 0, 0), 0)).toEqual([0.5, 0.5]);
    expect(equirectUv(new THREE.Vector3(0, -1, 0), 0)[1]).toBeCloseTo(0);
    // Turned 90° (three samples a rotated background with the inverse turn): +z reads
    // the panorama's seam (u 0 or 1).
    const [u] = equirectUv(new THREE.Vector3(0, 0, 1), Math.PI / 2);
    expect(Math.min(u, 1 - u)).toBeCloseTo(0);
    // The projected ground faces up, stays below the photo's horizon, and no triangle is
    // smeared across the panorama's seam.
    const geo = projectedGround(0, {
      photo: new THREE.Texture(),
      rotation: 0,
      center: new THREE.Vector3(0, 1700, 0),
      height: 1700,
      albedoScale: 0.2,
    });
    const uv = geo.getAttribute("uv");
    const n = geo.getAttribute("normal");
    for (let i = 0; i < uv.count; i += 3) {
      const us = [uv.getX(i), uv.getX(i + 1), uv.getX(i + 2)];
      expect(Math.max(...us) - Math.min(...us)).toBeLessThan(0.5);
      expect(uv.getY(i)).toBeLessThanOrEqual(0.5);
      expect(Math.abs(n.getY(i))).toBeGreaterThan(0.99);
    }
    // The dialog: backgrounds, lighting, tone, and saving without the background.
    cleanup();
    const app = appState(null);
    const v3d = app.views.find((v) => v.viewType === "ThreeD")!;
    useAppStore.setState({ app, activeView: v3d.id, openViews: [v3d.id] });
    render(<ViewDialogs />);
    await runAction("render");
    const dialog = await screen.findByRole("dialog", { name: "Render" });
    const bg = within(dialog).getByLabelText("Background") as HTMLSelectElement;
    expect([...bg.options].map((o) => o.textContent)).toContain("Grass Plain");
    expect(within(dialog).getByText(/Kloofendal/)).toBeTruthy();
    await userEvent.selectOptions(bg, "white");
    expect((within(dialog).getByLabelText("Lighting") as HTMLSelectElement).disabled).toBe(true);
    await userEvent.selectOptions(bg, "mountains");
    await userEvent.selectOptions(within(dialog).getByLabelText("Lighting"), "dome");
    expect(within(dialog).getByText(/Lit by the photo/)).toBeTruthy();
    expect(within(dialog).getByLabelText("Background rotation")).toBeTruthy();
    expect((within(dialog).getByLabelText("Tone") as HTMLSelectElement).value).toBe("contrast");
    const keep = within(dialog).getByLabelText("Include background") as HTMLInputElement;
    expect(keep.checked).toBe(true);
    await userEvent.click(keep);
    expect(keep.checked).toBe(false);
  });

  it("places cameras by clicking (the Camera tool was not routed to placement)", () => {
    expect(POINT_TOOLS).toContain("camera");
  });
});
