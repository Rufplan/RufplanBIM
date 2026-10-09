import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import * as THREE from "three";
import { App } from "./App";
import { useAppStore } from "./store";
import { toolAllowed } from "./tools";
import { plantPlacement } from "./vegetation";
import { installFakeBackend, type FakeBackend } from "./test/fakeBackend";
import { clumpGeometry, coverMask, GrassField, rng } from "./render/grass";
import { instanceMatrix, partGeometry } from "./render/plants";
import { formatFeet } from "./components/assetFormat";
import { buildScene } from "./render/pathtrace";

let fake: FakeBackend;

class SizedObserver {
  constructor(private cb: ResizeObserverCallback) {}
  observe() {
    this.cb([], this as unknown as ResizeObserver);
  }
  unobserve() {}
  disconnect() {}
}
const RealObserver = globalThis.ResizeObserver;

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
    assetFilter: null,
  });
  fake = installFakeBackend();
});
afterEach(() => {
  globalThis.ResizeObserver = RealObserver;
  vi.restoreAllMocks();
});

const argsOf = (cmd: string) =>
  fake.calls.filter((c) => c.cmd === cmd).map((c) => c.args as Record<string, unknown>);

async function openVegetation() {
  render(<App />);
  await userEvent.click(screen.getByRole("button", { name: "New Project" }));
  await screen.findByRole("toolbar", { name: "Tools" });
  await userEvent.click(screen.getByRole("tab", { name: "Landscape" }));
}

describe("the Vegetation tab and Enscape's Asset Library (ADR-064)", () => {
  it("has the Asset Library, placing, seasons and the ground", async () => {
    await openVegetation();
    for (const name of [
      /Asset Library/,
      /Place Plant/,
      /^Trees$/,
      /^Conifers$/,
      /^Palms$/,
      /^Shrubs$/,
      /^Grasses$/,
      /Base Ground/,
      /Ground Region/,
    ])
      expect(screen.getByRole("button", { name })).toBeInTheDocument();
    expect(screen.getByRole("combobox", { name: "Season" })).toBeInTheDocument();
    expect(toolAllowed("plant", "Plan")).toBe(true);
    expect(toolAllowed("plant", "ThreeD")).toBe(true);
    expect(toolAllowed("plant", "Section")).toBe(false);
  });

  it("the library filters by category, season and search; Place loads it and starts placing", async () => {
    await openVegetation();
    await userEvent.click(screen.getByRole("button", { name: /^Shrubs$/ }));
    const dialog = await screen.findByRole("dialog", { name: "Asset Library" });
    const grid = within(dialog).getByRole("list", { name: "Assets" });
    // The Shrubs button opens on Bushes.
    await waitFor(() => expect(within(grid).getAllByRole("listitem")).toHaveLength(1));
    expect(within(grid).getByText("Boxwood, Round")).toBeInTheDocument();
    await userEvent.click(within(dialog).getByRole("button", { name: /All Vegetation/ }));
    // Base assets only until a season is picked; Autumn shows the autumn variant.
    expect(within(grid).queryByText("Red Maple (Autumn)")).toBeNull();
    await userEvent.click(within(dialog).getByRole("radio", { name: "Autumn" }));
    expect(within(grid).getByText("Red Maple (Autumn)")).toBeInTheDocument();
    // Evergreen shrubs (no variants) still show in any season.
    expect(within(grid).getByText("Boxwood, Round")).toBeInTheDocument();
    await userEvent.type(within(dialog).getByRole("textbox", { name: "Search assets" }), "acer");
    expect(within(grid).queryByText("Boxwood, Round")).toBeNull();
    await userEvent.click(within(grid).getByText("Red Maple (Autumn)"));
    expect(within(dialog).getByText("Acer rubrum")).toBeInTheDocument();
    await userEvent.click(within(dialog).getByRole("button", { name: "Place" }));
    await waitFor(() =>
      expect(argsOf("load_planting_types").at(-1)).toMatchObject({
        names: ["Red Maple (Autumn)"],
      }),
    );
    await waitFor(() => expect(useAppStore.getState().tool).toBe("plant"));
    expect(useAppStore.getState().toolTypes.plant).toBe("00000000-0000-7000-8000-000000000061");
    expect(screen.queryByRole("dialog", { name: "Asset Library" })).toBeNull();
    // Enscape's placement options.
    expect(screen.getByRole("checkbox", { name: "Random rotation" })).toBeChecked();
    expect(screen.getByRole("spinbutton", { name: "Random size" })).toHaveValue(15);
  });

  it("placements turn and vary in size per the options bar", () => {
    const seq = [0.25, 1];
    const p = plantPlacement({ x: 1, y: 2 }, () => seq.shift()!);
    expect(p.rotation).toBeCloseTo(Math.PI / 2, 9);
    expect(p.scale).toBeCloseTo(1.15, 9);
    useAppStore.getState().setOption("plantRandomRotation", false);
    useAppStore.getState().setOption("plantSizeVariation", 0);
    const q = plantPlacement({ x: 1, y: 2 }, () => 0.9);
    expect(q).toEqual({ at: { x: 1, y: 2 }, rotation: 0, scale: 1 });
    useAppStore.getState().setOption("plantRandomRotation", true);
    useAppStore.getState().setOption("plantSizeVariation", 15);
  });

  it("clicking a plan with the tool places the picked plant", async () => {
    globalThis.ResizeObserver = SizedObserver as unknown as typeof ResizeObserver;
    vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockReturnValue({
      x: 0,
      y: 0,
      left: 0,
      top: 0,
      right: 800,
      bottom: 600,
      width: 800,
      height: 600,
      toJSON: () => ({}),
    } as DOMRect);
    const { container } = render(<App />);
    await userEvent.click(screen.getByRole("button", { name: "New Project" }));
    await screen.findByRole("toolbar", { name: "Tools" });
    await waitFor(() => expect(container.querySelector("canvas")).not.toBeNull());
    const canvas = container.querySelector("canvas")!;
    useAppStore.getState().setToolType("plant", "00000000-0000-7000-8000-000000000061");
    useAppStore.getState().setTool("plant");
    fireEvent.mouseDown(canvas, { button: 0, clientX: 300, clientY: 200 });
    fireEvent.mouseUp(canvas, { button: 0, clientX: 300, clientY: 200 });
    await waitFor(() => expect(argsOf("create_plants")).toHaveLength(1));
    const a = argsOf("create_plants")[0]!;
    expect(a).toMatchObject({ typeId: "00000000-0000-7000-8000-000000000061", level: null });
    const at = a.at as { rotation: number; scale: number }[];
    expect(at).toHaveLength(1);
    expect(at[0]!.scale).toBeGreaterThanOrEqual(0.85);
    expect(at[0]!.scale).toBeLessThanOrEqual(1.15);
  });

  it("Base Ground sets the ground to a Site & Landscape material", async () => {
    await openVegetation();
    await userEvent.click(screen.getByRole("button", { name: /Base Ground/ }));
    const dialog = await screen.findByRole("dialog", { name: "Base Ground" });
    await userEvent.click(await within(dialog).findByRole("listitem"));
    await waitFor(() =>
      expect(argsOf("set_base_ground").at(-1)).toMatchObject({
        material: null,
        preset: "site-lawn",
      }),
    );
  });

  it("sizes read as a landscape schedule gives them", () => {
    expect(formatFeet(15240)).toBe("50'");
    expect(formatFeet(305)).toBe('12"');
  });
});

describe("plants and grass in 3D (ADR-064)", () => {
  it("renders leave the working views' proxies out (the full models go in)", () => {
    const tri = [0, 0, 0, 1000, 0, 0, 0, 1000, 0];
    const mesh = (category: string) => ({
      el: category,
      category,
      exterior: false,
      color: null,
      material: null,
      level: null,
      positions: tri,
      edges: [],
    });
    const scene = buildScene([mesh("Planting"), mesh("Floor")] as never, {
      environment: new THREE.DataTexture(new Float32Array(4), 1, 1),
      rotation: 0,
      ground: "grass",
      imagery: null,
      groundZ: 0,
    });
    // The floor and the ground plane; no proxy.
    expect(scene.children.filter((c) => c instanceof THREE.Mesh)).toHaveLength(2);
  });

  it("a plant part becomes indexed geometry; solid colours turn linear", () => {
    const g = partGeometry(
      {
        positions: [0, 0, 0, 1, 0, 0, 0, 1, 0],
        normals: [0, 0, 1, 0, 0, 1, 0, 0, 1],
        uvs: [0, 0, 1, 0, 0, 1],
        colors: [0.5, 0.5, 0.5, 1, 1, 1, 0, 0, 0],
        indices: [0, 1, 2],
      },
      true,
    )!;
    expect(g.getIndex()!.count).toBe(3);
    expect(g.getAttribute("color").getX(0)).toBeCloseTo(0.214, 3);
    // RGBA, as the path tracer gives meshes without colours: RGB would misalign its merge.
    expect(g.getAttribute("color").itemSize).toBe(4);
    expect(g.getAttribute("color").getW(0)).toBe(1);
    expect(clumpGeometry().getAttribute("color").itemSize).toBe(4);
    expect(
      partGeometry({ positions: [], normals: [], uvs: [], colors: [], indices: [] }),
    ).toBeNull();
    const m = instanceMatrix({ at: [100, 200, 30], rotation: Math.PI / 2, scale: 2 });
    const p = new THREE.Vector3(1, 0, 1).applyMatrix4(m);
    expect(p.x).toBeCloseTo(100, 6);
    expect(p.y).toBeCloseTo(202, 6);
    expect(p.z).toBeCloseTo(32, 6);
  });

  it("grass grows densely near the camera, not under paving, fading at its edge", () => {
    // A 40 m lawn, and a 4 m square slab in its middle.
    const r = 20_000;
    const lawn = [-r, -r, 0, r, -r, 0, r, r, 0, -r, -r, 0, r, r, 0, -r, r, 0];
    const s = 2000;
    const slab = [-s, -s, 30, s, -s, 30, s, s, 30, -s, -s, 30, s, s, 30, -s, s, 30];
    const covered = coverMask([slab], new THREE.Vector3(), 45_000);
    expect(covered(0, 0, 0)).toBe(true);
    expect(covered(5000, 0, 0)).toBe(false);
    // Well above the slab (a roof over the lawn) doesn't cover it.
    expect(covered(0, 0, -3000)).toBe(false);
    // A 6" slab on grade, its top wound up and its underside down, with a floor over it:
    // its top covers the lawn graded flush with it, not its underside (ADR-118).
    const top = [-s, -s, 0, s, -s, 0, s, s, 0, -s, -s, 0, s, s, 0, -s, s, 0];
    const under = [-s, -s, -152, s, s, -152, s, -s, -152, -s, -s, -152, -s, s, -152, s, s, -152];
    const upper = top.map((v, i) => (i % 3 === 2 ? 3048 : v));
    const house = coverMask([top, under, upper], new THREE.Vector3(), 45_000);
    expect(house(0, 0, -20)).toBe(true);
    expect(house(0, 0, 0)).toBe(true);
    const field = new GrassField(
      [{ positions: lawn, grass: { height: 60, variation: 0.3 }, color: [90, 120, 60] }],
      new THREE.Vector3(),
      10_000,
      [slab],
    );
    expect(field.count).toBeGreaterThan(50_000);
    for (let i = 0; i < field.count; i++) {
      const [x, y] = [field.data[i * 5]!, field.data[i * 5 + 1]!];
      expect(Math.max(Math.abs(x), Math.abs(y)) > s - 250).toBe(true);
    }
    const mesh = new THREE.InstancedMesh(clumpGeometry(), undefined, 5000);
    field.fill(mesh, 10_000, 10_000, 5000);
    expect(mesh.count).toBe(5000);
    // The nearest are full height, the farthest shrink toward nothing.
    const scaleOf = (i: number) => {
      const m = new THREE.Matrix4();
      mesh.getMatrixAt(i, m);
      return new THREE.Vector3().setFromMatrixScale(m).z;
    };
    expect(scaleOf(0)).toBeGreaterThan(35);
    expect(scaleOf(4999)).toBeLessThan(10);
    // Deterministic.
    const a = rng(5);
    const b = rng(5);
    expect(a()).toBe(b());
  });
});
