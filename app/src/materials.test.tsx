import { beforeEach, describe, expect, it } from "vitest";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import * as THREE from "three";
import { App } from "./App";
import { matchesPreset, presetThumb } from "./components/MaterialBrowser";
import { boxUv, HEX_H, physicalMaterial } from "./render/materials";
import { useAppStore } from "./store";
import { installFakeBackend, LIBRARY, type FakeBackend } from "./test/fakeBackend";

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
  });
  fake = installFakeBackend();
});

async function openProject() {
  render(<App />);
  await userEvent.click(screen.getByRole("button", { name: "New Project" }));
  await screen.findByRole("toolbar", { name: "Tools" });
}

describe("Materials tab and Material Browser (ADR-029)", () => {
  it("sits between Architecture and Rendering", async () => {
    await openProject();
    const tabs = within(screen.getByRole("tablist", { name: "Ribbon tabs" }))
      .getAllByRole("tab")
      .map((t) => t.textContent);
    expect(tabs.slice(0, 4)).toEqual(["Site", "Architecture", "Materials", "Rendering"]);
    await userEvent.click(screen.getByRole("tab", { name: "Materials" }));
    for (const name of ["Material Browser", "New Material", "Duplicate", "Apply Material"])
      expect(screen.getByRole("button", { name })).toBeTruthy();
    // Nothing selected: nothing to apply to.
    expect(
      (screen.getByRole("button", { name: "Apply Material" }) as HTMLButtonElement).disabled,
    ).toBe(true);
    // The Manage tab no longer has them.
    await userEvent.click(screen.getByRole("tab", { name: "Manage" }));
    expect(screen.queryByRole("button", { name: "New Material" })).toBeNull();
  });

  it("filters the library, previews on cubes, adds and applies", async () => {
    await openProject();
    useAppStore.setState({ selection: ["wall-1"] });
    await userEvent.click(screen.getByRole("tab", { name: "Materials" }));
    await userEvent.click(screen.getByRole("button", { name: "Apply Material" }));
    const dialog = await screen.findByRole("dialog", { name: "Material Browser" });
    const grid = within(dialog).getByRole("list");
    await within(grid).findByText("White Oak Plank Flooring, Matte");
    // Library previews are the bundled cube renders.
    const img = within(grid).getByAltText("White Oak Plank Flooring, Matte") as HTMLImageElement;
    expect(img.getAttribute("src")).toBe(presetThumb("wood-white-oak-floor"));
    // Filters: finish level, building type, category and search.
    await userEvent.click(within(dialog).getByLabelText("High-end"));
    expect(within(grid).queryByText("Clear Anodized Aluminum")).toBeNull();
    await userEvent.click(within(dialog).getByLabelText("High-end"));
    await userEvent.click(within(dialog).getByLabelText("Residential"));
    expect(within(grid).queryByText("Clear Anodized Aluminum")).toBeNull();
    await userEvent.click(within(dialog).getByLabelText("Residential"));
    await userEvent.click(within(dialog).getByRole("button", { name: "Metal" }));
    expect(within(grid).queryByText("White Oak Plank Flooring, Matte")).toBeNull();
    await userEvent.click(within(dialog).getByRole("button", { name: "All" }));
    await userEvent.type(within(dialog).getByLabelText("Search materials"), "oak");
    expect(within(grid).getAllByRole("listitem")).toHaveLength(1);
    // Pick it: details, V-Ray settings, then add and apply to the selection.
    await userEvent.click(within(grid).getByText("White Oak Plank Flooring, Matte"));
    expect(within(dialog).getByText("Reflection glossiness")).toBeTruthy();
    expect(within(dialog).getByText(/Photo, 2K/)).toBeTruthy();
    await userEvent.click(within(dialog).getByRole("button", { name: "Add & Apply to Selection" }));
    expect(fake.calls.some((c) => c.cmd === "add_library_material")).toBe(true);
    expect(fake.applied).toEqual([[["wall-1"], "00000000-0000-7000-8000-0000000000a1"]]);
    expect(screen.queryByRole("dialog", { name: "Material Browser" })).toBeNull();
  });

  it("knows an unedited preset from an edited material", () => {
    const p = LIBRARY[0]!;
    const m = { id: "m", name: p.name, color: p.color, appearance: p.appearance };
    expect(matchesPreset(m, p)).toBe(true);
    expect(matchesPreset({ ...m, appearance: { ...p.appearance, roughness: 0.2 } }, p)).toBe(false);
    expect(matchesPreset(m, undefined)).toBe(false);
  });

  it("builds V-Ray-like physical materials with real-world box mapping", () => {
    const glass = physicalMaterial(
      { ...LIBRARY[1]!.appearance, metalness: 0, refraction: 1, ior: 1.52, roughness: 0 },
      [245, 250, 248],
      null,
    );
    expect(glass.transmission).toBe(1);
    expect(glass.thickness).toBe(0);
    expect((glass as unknown as { castShadow?: boolean }).castShadow).toBe(false);
    const metal = physicalMaterial(LIBRARY[1]!.appearance, [190, 192, 196], null);
    expect(metal.metalness).toBe(1);
    // Box mapping: a 1 m floor square under a 500 mm tile spans two repeats; a wall facing
    // south maps its height to v.
    const floor = new THREE.BufferGeometry();
    floor.setAttribute(
      "position",
      new THREE.Float32BufferAttribute([0, 0, 0, 1000, 0, -1000, 1000, 0, 0], 3),
    );
    boxUv(floor, 500);
    const uv = floor.getAttribute("uv");
    expect([uv.getX(1), uv.getY(1)]).toEqual([2, 2]);
    const wall = new THREE.BufferGeometry();
    wall.setAttribute(
      "position",
      new THREE.Float32BufferAttribute([0, 0, 0, 1000, 0, 0, 1000, 2000, 0], 3),
    );
    boxUv(wall, 1000, 0.5);
    const w = wall.getAttribute("uv");
    expect([w.getX(2), w.getY(2)]).toEqual([1, 4]);
    expect(wall.getAttribute("tangent").getX(0)).toBe(1);
    // Hexagon tiles hold whole rows.
    expect(HEX_H).toBe(Math.round(7 * Math.sqrt(3) * (1024 / 12)));
  });
});
