import { beforeEach, describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import * as THREE from "three";
import { App } from "./App";
import { useAppStore } from "./store";
import { shortcut, toolAllowed } from "./tools";
import { boxPlanes, dragCoordinate, meshColor } from "./components/View3D";
import { installFakeBackend, type FakeBackend } from "./test/fakeBackend";

let fake: FakeBackend;

beforeEach(() => {
  useAppStore.setState({
    app: null,
    error: null,
    confirm: null,
    rufplan: null,
    paramsOpen: false,
    openViews: [],
    activeView: null,
    selection: [],
    tool: "select",
  });
  fake = installFakeBackend();
});

async function openProject() {
  render(<App />);
  await userEvent.click(screen.getByRole("button", { name: "New Project" }));
  await screen.findByRole("toolbar", { name: "Tools" });
}

describe("detailing, materials and the section box (ADR-020)", () => {
  it("room separators and callouts have shortcuts and the right views", () => {
    expect(shortcut("R", "S").tool).toBe("roomSeparator");
    expect(shortcut("C", "A").tool).toBe("callout");
    expect(toolAllowed("roomSeparator", "Plan")).toBe(true);
    expect(toolAllowed("roomSeparator", "Section")).toBe(false);
    for (const v of ["Plan", "CeilingPlan", "Elevation", "Section"] as const)
      expect(toolAllowed("callout", v)).toBe(true);
    expect(toolAllowed("callout", "ThreeD")).toBe(false);
  });

  it("New Material creates one and selects it for editing", async () => {
    await openProject();
    await userEvent.click(screen.getByRole("tab", { name: "Materials" }));
    await userEvent.click(screen.getByRole("button", { name: "New Material" }));
    expect(fake.calls.find((c) => c.cmd === "create_material")?.args).toMatchObject({
      from: null,
    });
    expect(useAppStore.getState().selection).toEqual(["00000000-0000-7000-8000-000000000052"]);
  });

  it("the project browser lists materials and the new families", async () => {
    await openProject();
    await userEvent.click(screen.getByRole("button", { name: /Materials/ }));
    await userEvent.click(screen.getByRole("button", { name: "Brick" }));
    expect(useAppStore.getState().selection).toEqual(["00000000-0000-7000-8000-000000000050"]);
    expect(screen.getByRole("button", { name: /Structural Columns/ })).toBeTruthy();
  });

  it("section box planes keep the inside, and handles drag along their axis", () => {
    const planes = boxPlanes({ min: [0, 0, 0], max: [1000, 2000, 3000] });
    expect(planes).toHaveLength(6);
    const inside = new THREE.Vector3(500, 1000, 1500);
    const outside = new THREE.Vector3(1500, 1000, 1500);
    expect(planes.every((p) => p.distanceToPoint(inside) >= 0)).toBe(true);
    expect(planes.some((p) => p.distanceToPoint(outside) < 0)).toBe(true);
    expect(boxPlanes(null)).toEqual([]);
    // Looking down from above the x axis handle at x = 1000, pointing at x = 1400.
    const x = dragCoordinate(
      new THREE.Vector3(1400, 1000, 9000),
      new THREE.Vector3(0, 0, -1),
      new THREE.Vector3(1000, 1000, 1500),
      0,
    );
    expect(x).toBeCloseTo(1400);
  });

  it("3D colors come from materials, except glass and ceilings", () => {
    const m = { el: "x", category: "Wall", exterior: true, positions: [], color: [168, 82, 60] };
    expect(meshColor(m as never)).toBe(0xa8523c);
    expect(meshColor({ ...m, category: "Window" } as never)).toBe(0x9fe3f7);
    expect(meshColor({ ...m, color: null } as never)).toBe(0xe9e7e2);
  });
});
