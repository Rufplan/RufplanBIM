import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import * as THREE from "three";
import { App } from "./App";
import { useAppStore } from "./store";
import { installFakeBackend, type FakeBackend } from "./test/fakeBackend";
import { fixtureLight, LUX_PER_UNIT } from "./render/pathtrace";
import { SCHEMES } from "./components/RenderDialog";
import { photometrics } from "./components/LightPicker";
import type { LightInfo } from "./bindings/LightInfo";

let fake: FakeBackend;

// The canvas needs a size to have a camera: a ResizeObserver that reports 800 x 600.
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
  });
  fake = installFakeBackend();
});
afterEach(() => {
  globalThis.ResizeObserver = RealObserver;
  vi.restoreAllMocks();
});

async function openLighting() {
  render(<App />);
  await userEvent.click(screen.getByRole("button", { name: "New Project" }));
  await screen.findByRole("toolbar", { name: "Tools" });
  await userEvent.click(screen.getByRole("tab", { name: "Lighting" }));
}

const argsOf = (cmd: string) =>
  fake.calls.filter((c) => c.cmd === cmd).map((c) => c.args as Record<string, unknown>);

const light = (over: Partial<LightInfo>): LightInfo => ({
  el: "l1",
  type_id: "00000000-0000-7000-8000-000000000042",
  level: "00000000-0000-7000-8000-000000000001",
  at: [0, 0, 2700],
  dir: [0, 0, -1],
  axis: [1, 0],
  size: [190.5, 190.5],
  shape: "Point",
  distribution: "Spherical",
  lumens: 1000,
  color: [255, 255, 255],
  beam: 180,
  on: true,
  dimming: 1,
  exterior: false,
  ...over,
});

describe("Lighting tab (ADR-057)", () => {
  it("has Revit's fixture, sun, artificial light and render tools", async () => {
    await openLighting();
    for (const name of [
      "Lighting Fixture",
      /Load Fixtures/,
      /Sun Settings/,
      /Artificial Lights/,
      /^Render$/,
    ])
      expect(screen.getByRole("button", { name })).toBeInTheDocument();
  });

  it("Lighting Fixture opens the picker; the library filters by group and building type", async () => {
    await openLighting();
    await userEvent.click(screen.getByRole("button", { name: "Lighting Fixture" }));
    const dialog = await screen.findByRole("dialog", { name: "Lighting Fixture Types" });
    expect(useAppStore.getState().tool).toBe("light");
    const project = within(dialog).getByRole("list", { name: "Project lighting fixtures" });
    expect(within(project).getAllByRole("listitem")).toHaveLength(3);
    await userEvent.click(within(dialog).getByRole("tab", { name: "Lighting Library" }));
    const lib = await within(dialog).findByRole("list", { name: "Library fixtures" });
    await waitFor(() => expect(within(lib).getAllByRole("listitem")).toHaveLength(3));
    await userEvent.click(within(dialog).getByRole("button", { name: "Site & Exterior" }));
    expect(
      within(lib)
        .getAllByRole("listitem")
        .map((i) => i.textContent),
    ).toEqual(['42" Bollard']);
    await userEvent.click(within(dialog).getByRole("button", { name: "All Fixtures" }));
    await userEvent.selectOptions(within(dialog).getByLabelText("Building type"), "Office");
    expect(within(lib).getAllByRole("listitem")).toHaveLength(2);
    // The list layout shows each fixture's description.
    await userEvent.click(within(dialog).getByRole("radio", { name: "List" }));
    expect(within(dialog).getByText("Lay-in 2'x4' troffer")).toBeInTheDocument();
    await userEvent.click(within(dialog).getByRole("listitem", { name: /2x4 LED Troffer/ }));
    const specs = within(dialog).getByRole("table", { name: "Photometrics" });
    expect(specs.textContent).toContain("4,000 lm");
    expect(specs.textContent).toContain("Rectangle, Hemispherical");
    await userEvent.click(within(dialog).getByRole("button", { name: "Load & Place" }));
    await waitFor(() =>
      expect(argsOf("load_lighting_types").at(-1)).toMatchObject({ names: ["2x4 LED Troffer"] }),
    );
    expect(useAppStore.getState().toolTypes.light).toBe("00000000-0000-7000-8000-000000000044");
    expect(screen.queryByRole("dialog", { name: "Lighting Fixture Types" })).toBeNull();
  });

  it("clicking a plan with the tool places the fixture of the picked type", async () => {
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
    useAppStore.getState().setToolType("light", "00000000-0000-7000-8000-000000000042");
    useAppStore.getState().setTool("light");
    fireEvent.mouseDown(canvas, { button: 0, clientX: 300, clientY: 200 });
    fireEvent.mouseUp(canvas, { button: 0, clientX: 300, clientY: 200 });
    await waitFor(() => expect(argsOf("create_lighting_fixture")).toHaveLength(1));
    expect(argsOf("create_lighting_fixture")[0]).toMatchObject({
      typeId: "00000000-0000-7000-8000-000000000042",
      level: null,
      elevation: null,
    });
  });

  it("Sun Settings saves a Still date and time, or a Lighting azimuth and altitude", async () => {
    await openLighting();
    await userEvent.click(screen.getByRole("button", { name: /Sun Settings/ }));
    const dialog = await screen.findByRole("dialog", { name: "Sun Settings" });
    await userEvent.selectOptions(
      within(dialog).getByLabelText("Presets"),
      "Winter Solstice, Noon",
    );
    expect(within(dialog).getByText("Time: 12:00 PM")).toBeInTheDocument();
    await userEvent.click(within(dialog).getByRole("button", { name: "OK" }));
    await waitFor(() =>
      expect(argsOf("set_sun_settings").at(-1)).toMatchObject({
        settings: { mode: "Still", month: 12, day: 21, hour: 12 },
      }),
    );
    await userEvent.click(screen.getByRole("button", { name: /Sun Settings/ }));
    const again = await screen.findByRole("dialog", { name: "Sun Settings" });
    await userEvent.click(within(again).getByRole("radio", { name: "Lighting" }));
    fireEvent.change(within(again).getByLabelText("Altitude"), { target: { value: "60" } });
    await userEvent.click(within(again).getByRole("button", { name: "OK" }));
    await waitFor(() =>
      expect(argsOf("set_sun_settings").at(-1)).toMatchObject({
        settings: { mode: "Lighting", altitude: 60, azimuth: 225 },
      }),
    );
  });

  it("Artificial Lights switches a type's fixtures, one or all", async () => {
    fake.lights = [light({ el: "a" }), light({ el: "b", on: false })];
    await openLighting();
    await userEvent.click(screen.getByRole("button", { name: /Artificial Lights/ }));
    const dialog = await screen.findByRole("dialog", { name: "Artificial Lights" });
    const group = await within(dialog).findByRole("checkbox", { name: '6" LED Downlight on' });
    expect(group).not.toBeChecked();
    expect((group as HTMLInputElement).indeterminate).toBe(true);
    await userEvent.click(group);
    expect(argsOf("set_lights").at(-1)).toMatchObject({ ids: ["a", "b"], on: true });
    await userEvent.click(within(dialog).getByRole("checkbox", { name: '6" LED Downlight 1 on' }));
    expect(argsOf("set_lights").at(-1)).toMatchObject({ ids: ["a"], on: false });
    await userEvent.click(within(dialog).getByRole("button", { name: "All Off" }));
    expect(argsOf("set_lights").at(-1)).toMatchObject({ ids: ["a", "b"], on: false });
  });
});

describe("fixture lights in renders (ADR-057)", () => {
  const k = 1e6 / LUX_PER_UNIT;
  it("a spherical source is a point light of its candela", () => {
    const p = fixtureLight(light({})) as THREE.PointLight;
    expect(p).toBeInstanceOf(THREE.PointLight);
    expect(p.intensity).toBeCloseTo((1000 / (4 * Math.PI)) * k, 6);
    expect(p.decay).toBe(2);
    // z-up model to y-up.
    expect([p.position.x, p.position.y, p.position.z]).toEqual([0, 2700, -0]);
  });

  it("a spot is a cone of its beam; a hemisphere a cosine spot", () => {
    const s = fixtureLight(light({ distribution: "Spot", beam: 60 })) as THREE.SpotLight;
    expect(s.angle).toBeCloseTo(Math.PI / 6, 9);
    expect(s.intensity).toBeCloseTo((1000 / (2 * Math.PI * (1 - Math.cos(Math.PI / 6)))) * k, 6);
    expect(s.target.position.y).toBeCloseTo(1700, 9);
    const h = fixtureLight(light({ distribution: "Hemispherical" })) as THREE.SpotLight;
    expect(h.intensity).toBeCloseTo((1000 / Math.PI) * k, 6);
    expect(h.penumbra).toBe(1);
  });

  it("a troffer is an area light of its size, facing down", () => {
    const a = fixtureLight(
      light({ shape: "Rectangle", size: [609.6, 1219.2], lumens: 4000 }),
    ) as THREE.RectAreaLight;
    expect(a).toBeInstanceOf(THREE.RectAreaLight);
    expect([a.width, a.height]).toEqual([609.6, 1219.2]);
    expect(a.intensity).toBeCloseTo(4000 / (Math.PI * 0.6096 * 1.2192) / LUX_PER_UNIT, 9);
    const facing = new THREE.Vector3(0, 0, -1).applyQuaternion(a.quaternion);
    expect(facing.y).toBeCloseTo(-1, 9);
  });

  it("Revit's six lighting schemes, interiors and night brighter", () => {
    expect(SCHEMES.map(([s]) => s)).toHaveLength(6);
    const base = Object.fromEntries(SCHEMES);
    expect(base["Interior: Artificial only"]).toBeGreaterThan(base["Interior: Sun only"]!);
    expect(base["Exterior: Sun only"]).toBe(1);
  });

  it("photometrics read like Revit's type properties", () => {
    const rows = Object.fromEntries(
      photometrics({
        family: "WallSconce",
        mount: "Wall",
        width: 127,
        depth: 101.6,
        height: 304.8,
        drop: 0,
        mount_height: 1676.4,
        lumens: 800,
        watts: 10,
        kelvin: 3000,
        shape: "Point",
        distribution: "Hemispherical",
        beam: 180,
      }),
    );
    expect(rows["Wattage"]).toBe("10 W (80 lm/W)");
    expect(rows["Mounting height"]).toBe('66"');
    expect(rows["Size"]).toBe('5" x 4" x 12"');
  });
});
