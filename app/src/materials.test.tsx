import { beforeEach, describe, expect, it } from "vitest";
import { fireEvent, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import * as THREE from "three";
import { App } from "./App";
import type { Appearance } from "./bindings/Appearance";
import { formatInches, matchesPreset, presetThumb } from "./components/MaterialBrowser";
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
    expect(tabs.slice(1, 8)).toEqual([
      "Site",
      "Vegetation",
      "Architecture",
      "Openings",
      "Lighting",
      "Materials",
      "Rendering",
    ]);
    await userEvent.click(screen.getByRole("tab", { name: "Materials" }));
    for (const name of ["Material Browser", "New Material", "Duplicate", "Paint"])
      expect(screen.getByRole("button", { name })).toBeTruthy();
    // Paint works without a selection (ADR-034).
    expect((screen.getByRole("button", { name: "Paint" }) as HTMLButtonElement).disabled).toBe(
      false,
    );
    // The Manage tab no longer has them.
    await userEvent.click(screen.getByRole("tab", { name: "Manage" }));
    expect(screen.queryByRole("button", { name: "New Material" })).toBeNull();
  });

  it("filters the library, previews on cubes, adds and applies", async () => {
    await openProject();
    useAppStore.setState({ selection: ["wall-1"] });
    await userEvent.click(screen.getByRole("tab", { name: "Materials" }));
    // Paint with no material yet opens the browser.
    await userEvent.click(screen.getByRole("button", { name: "Paint" }));
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
    // A sloped roof (ADR-061) maps along its contour and up its slope at true length,
    // whichever way it faces: an east-facing 6/12 slope, y-up, rising to the west.
    const rise = 0.5;
    const len = Math.hypot(1000, 1000 * rise);
    const roof = new THREE.BufferGeometry();
    roof.setAttribute(
      "position",
      new THREE.Float32BufferAttribute([1000, 0, 0, 1000, 0, -1000, 0, 1000 * rise, 0], 3),
    );
    boxUv(roof, 1000);
    const r = roof.getAttribute("uv");
    // Along the eave (north-south here): u moves, v stays (courses run level).
    expect(Math.abs(r.getX(1) - r.getX(0))).toBeCloseTo(1, 9);
    expect(r.getY(1)).toBeCloseTo(r.getY(0), 9);
    // Up to the ridge: v grows by the slope's true length.
    expect(r.getY(2) - r.getY(0)).toBeCloseTo(len / 1000, 6);
    const tan = roof.getAttribute("tangent");
    expect(Math.abs(tan.getZ(0))).toBeCloseTo(1, 9);
    // Hexagon tiles hold whole rows.
    expect(HEX_H).toBe(Math.round(7 * Math.sqrt(3) * (1024 / 12)));
  });

  it("sets Enscape's material Type: Grass with its height and variation (ADR-064)", async () => {
    const oak = LIBRARY[0]!.appearance as Appearance;
    oak.grass = { height: 60, variation: 0.35 };
    try {
      await openProject();
      await userEvent.click(screen.getByRole("tab", { name: "Materials" }));
      await userEvent.click(screen.getByRole("button", { name: "Material Browser" }));
      const dialog = await screen.findByRole("dialog", { name: "Material Browser" });
      const grid = within(dialog).getByRole("list");
      await userEvent.click(await within(grid).findByText("White Oak Plank Flooring, Matte"));
      expect(within(dialog).getByText(`Grass · 2 3/8" · 35% variation`)).toBeTruthy();
      await userEvent.click(within(dialog).getByRole("button", { name: "Add to Project" }));
      await userEvent.click(within(dialog).getByRole("tab", { name: /In This Project/ }));
      await userEvent.click(await within(grid).findByText("White Oak Plank Flooring, Matte"));
      const type = within(dialog).getByLabelText("Material type") as HTMLSelectElement;
      expect(type.value).toBe("grass");
      const id = "00000000-0000-7000-8000-0000000000a1";
      const sets = () => fake.calls.filter((c) => c.cmd === "set_property").map((c) => c.args);
      const height = within(dialog).getByLabelText("Grass height") as HTMLInputElement;
      expect(height.value).toBe(`2 3/8"`);
      await userEvent.clear(height);
      await userEvent.type(height, `4"{Enter}`);
      expect(sets()).toContainEqual(
        expect.objectContaining({ id, key: "grass_height", value: `4"` }),
      );
      const range = within(dialog).getByLabelText("Height variation");
      fireEvent.change(range, { target: { value: "60" } });
      fireEvent.blur(range);
      expect(sets()).toContainEqual(
        expect.objectContaining({ id, key: "grass_variation", value: "60%" }),
      );
      await userEvent.selectOptions(type, "generic");
      expect(sets()).toContainEqual(
        expect.objectContaining({ id, key: "grass", value: "generic" }),
      );
    } finally {
      delete oak.grass;
    }
    // Heights read in inches.
    expect(formatInches(60)).toBe(`2 3/8"`);
    expect(formatInches(300)).toBe(`11 3/4"`);
    expect(formatInches(1219.2)).toBe(`4'-0"`);
  });

  it("offers Grass on a generic project material", async () => {
    await openProject();
    await userEvent.click(screen.getByRole("tab", { name: "Materials" }));
    await userEvent.click(screen.getByRole("button", { name: "Material Browser" }));
    const dialog = await screen.findByRole("dialog", { name: "Material Browser" });
    await userEvent.click(within(dialog).getByRole("tab", { name: /In This Project/ }));
    await userEvent.click(await within(within(dialog).getByRole("list")).findByText("Brick"));
    const type = within(dialog).getByLabelText("Material type") as HTMLSelectElement;
    expect(type.value).toBe("generic");
    expect(within(dialog).queryByLabelText("Grass height")).toBeNull();
    await userEvent.selectOptions(type, "grass");
    expect(
      fake.calls.some(
        (c) =>
          c.cmd === "set_property" &&
          (c.args as { key: string; value: string }).key === "grass" &&
          (c.args as { value: string }).value === "grass",
      ),
    ).toBe(true);
  });
});
