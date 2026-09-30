import { beforeEach, describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import * as THREE from "three";
import { App } from "./App";
import { OptionsBar } from "./components/OptionsBar";
import { useAppStore } from "./store";
import { toolAllowed } from "./tools";
import { installFakeBackend } from "./test/fakeBackend";
import { clumpGeometry, kindFor, patchTint } from "./render/grass";
import { physicalSky } from "./render/sky";
import { BACKGROUNDS } from "./render/backgrounds";

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
    ribbonTab: "Architecture",
  });
  installFakeBackend();
});

describe("D5 Render's look (ADR-065)", () => {
  it("each grass kind grows its own clump: lawns dense, meadows with coloured flowers", () => {
    const colors = (g: THREE.BufferGeometry) => g.getAttribute("color");
    const lawn = clumpGeometry("Lawn");
    const meadow = clumpGeometry("Meadow", [104, 132, 62]);
    // RGBA (the path tracer's merge needs it).
    expect(colors(lawn).itemSize).toBe(4);
    // A lawn's clump is all grass: every colour a tint of its green (green the strongest).
    const c = colors(lawn);
    for (let i = 0; i < c.count; i++) expect(c.getY(i)).toBeGreaterThanOrEqual(c.getZ(i));
    // A meadow's flowers keep their own colours: some near white, some yellow or purple.
    const m = colors(meadow);
    let white = 0;
    let purple = 0;
    for (let i = 0; i < m.count; i++) {
      if (m.getX(i) > 0.8 && m.getZ(i) > 0.75) white++;
      if (m.getZ(i) > m.getY(i) * 1.5 && m.getZ(i) > 0.5) purple++;
    }
    expect(white).toBeGreaterThan(0);
    expect(purple).toBeGreaterThan(0);
    // Tall grass is fewer, finer blades than a lush lawn.
    const tris = (g: THREE.BufferGeometry) => g.getIndex()!.count / 3;
    expect(tris(clumpGeometry("LushLawn"))).toBeGreaterThan(tris(clumpGeometry("TallGrass")));
    // A Grass-type material is a lawn, or a meadow when tall.
    expect(kindFor({ positions: [], grass: { height: 60, variation: 0 }, color: [0, 0, 0] })).toBe(
      "Lawn",
    );
    expect(kindFor({ positions: [], grass: { height: 300, variation: 0 }, color: [0, 0, 0] })).toBe(
      "Meadow",
    );
  });

  it("the lawn has gentle patches, not stripes or blotches", () => {
    let lo = Infinity;
    let hi = -Infinity;
    for (let x = 0; x < 40_000; x += 700)
      for (let y = 0; y < 40_000; y += 700) {
        const [r, g] = patchTint(x, y);
        lo = Math.min(lo, g);
        hi = Math.max(hi, g);
        expect(r).toBeGreaterThan(0.7);
      }
    expect(hi - lo).toBeGreaterThan(0.1);
    expect(hi - lo).toBeLessThan(0.45);
  });

  it("the default sky has clouds, bright against the blue", () => {
    const sun = new THREE.Vector3(0.5, 0.6, -0.6).normalize();
    const lum = (t: THREE.DataTexture) => {
      const d = t.image.data as Float32Array;
      const out: number[] = [];
      const { width: w, height: h } = t.image;
      // The upper sky, away from the horizon.
      for (let j = Math.floor(h * 0.7); j < h; j++)
        for (let i = 0; i < w; i++) {
          const k = (j * w + i) * 4;
          out.push(0.2126 * d[k]! + 0.7152 * d[k + 1]! + 0.0722 * d[k + 2]!);
        }
      return out;
    };
    const clear = lum(
      physicalSky({ sunDir: sun, altitude: 35, sunToSky: 0, width: 256, height: 128 }),
    );
    const cloudy = lum(
      physicalSky({
        sunDir: sun,
        altitude: 35,
        sunToSky: 0,
        width: 256,
        height: 128,
        clouds: 0.42,
      }),
    );
    // Clouds brighten a share of the sky (pixel by pixel against the clear one).
    const bright = cloudy.filter((v, i) => v > clear[i]! * 1.3).length;
    expect(bright / cloudy.length).toBeGreaterThan(0.05);
    expect(bright / cloudy.length).toBeLessThan(0.7);
    // It's the renders' default background too.
    expect(BACKGROUNDS[0]!.id).toBe("physical");
    expect(BACKGROUNDS[0]!.label).toContain("D5");
  });
});

describe("D5's Grass Brush (ADR-065)", () => {
  it("paints in 3D, with the grass, brush size, density and Erase on the options bar", async () => {
    expect(toolAllowed("grassBrush", "ThreeD")).toBe(true);
    expect(toolAllowed("grassBrush", "Plan")).toBe(false);
    useAppStore.setState({ tool: "grassBrush" });
    render(<OptionsBar />);
    const kind = screen.getByRole("combobox", { name: "Grass" });
    expect(kind).toHaveValue("Lawn");
    await userEvent.selectOptions(kind, "Meadow");
    expect(useAppStore.getState().options.grassKind).toBe("Meadow");
    expect(screen.getByRole("slider", { name: "Brush size" })).toBeInTheDocument();
    expect(screen.getByRole("slider", { name: "Grass density" })).toBeInTheDocument();
    await userEvent.click(screen.getByRole("checkbox", { name: "Erase" }));
    expect(useAppStore.getState().options.grassErase).toBe(true);
    useAppStore.getState().setOption("grassErase", false);
    useAppStore.getState().setOption("grassKind", "Lawn");
  });

  it("the Vegetation tab has the Grass Brush", async () => {
    render(<App />);
    await userEvent.click(screen.getByRole("button", { name: "New Project" }));
    await screen.findByRole("toolbar", { name: "Tools" });
    await userEvent.click(screen.getByRole("tab", { name: "Landscape" }));
    expect(screen.getByRole("button", { name: /Grass Brush/ })).toBeInTheDocument();
  });
});

describe("zooming into a render", () => {
  it("zooms in and out with the buttons and fits again", async () => {
    const { appState } = await import("./test/fakeBackend");
    const { ViewDialogs } = await import("./components/ViewDialogs");
    const { runAction } = await import("./actions");
    const { within } = await import("@testing-library/react");
    const app = appState(null);
    const v3d = app.views.find((v) => v.viewType === "ThreeD")!;
    useAppStore.setState({ app, activeView: v3d.id, openViews: [v3d.id] });
    render(<ViewDialogs />);
    await runAction("render");
    const dialog = await screen.findByRole("dialog", { name: "Render" });
    const level = within(dialog).getByLabelText("Zoom level");
    expect(level).toHaveTextContent("100%");
    await userEvent.click(within(dialog).getByRole("button", { name: "Zoom in" }));
    expect(level).toHaveTextContent("150%");
    await userEvent.click(within(dialog).getByRole("button", { name: "Zoom in" }));
    expect(level).toHaveTextContent("225%");
    await userEvent.click(within(dialog).getByRole("button", { name: "Zoom out" }));
    expect(level).toHaveTextContent("150%");
    await userEvent.click(within(dialog).getByRole("button", { name: "Fit" }));
    expect(level).toHaveTextContent("100%");
    // Never smaller than fitted.
    await userEvent.click(within(dialog).getByRole("button", { name: "Zoom out" }));
    expect(level).toHaveTextContent("100%");
  });
});
