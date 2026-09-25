import { beforeEach, describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import * as THREE from "three";
import { App } from "./App";
import { groundGrid, prompt3d } from "./components/View3D";
import { startSketch } from "./sketch";
import { useAppStore } from "./store";
import { toolAllowed } from "./tools";
import { appState, installFakeBackend, type FakeBackend } from "./test/fakeBackend";

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
    grid3d: true,
  });
  fake = installFakeBackend();
});

let fake: FakeBackend;

describe("editing in 3D and the ground grid (ADR-022)", () => {
  it("building tools work in 3D; plan-only tools don't", () => {
    for (const t of [
      "wall",
      "door",
      "window",
      "floorAuto",
      "ceilingAuto",
      "roof",
      "column",
      "room",
    ] as const)
      expect(toolAllowed(t, "ThreeD")).toBe(true);
    for (const t of ["dimension", "level", "grid", "stair"] as const)
      expect(toolAllowed(t, "ThreeD")).toBe(false);
    expect(prompt3d("door")).toContain("Hover over a wall");
    expect(prompt3d("wall")).toContain("work plane");
  });

  it("the ground grid is hairline cyan, 4' squares with a stronger line every 20'", () => {
    const g = groundGrid(new THREE.Vector3(0, 0, 0), 12192, 0);
    const [minor, major] = g.children as THREE.LineSegments[];
    const mm = minor!.material as THREE.LineBasicMaterial;
    const mj = major!.material as THREE.LineBasicMaterial;
    expect(mm.color.getHex()).toBe(0x3ecff7);
    expect(mm.opacity).toBeLessThan(mj.opacity);
    // 40' each way of center: 21 lines per direction, 5 of them major.
    const segs = (l: THREE.LineSegments) => l.geometry.getAttribute("position").count / 2;
    expect(segs(major!)).toBe(5 * 2);
    expect(segs(minor!)).toBe(16 * 2);
    expect(g.children.every((c) => c.position.z === 0)).toBe(true);
  });

  it("the View tab toggles the grid, and marker types are families in the browser", async () => {
    render(<App />);
    await userEvent.click(screen.getByRole("button", { name: "New Project" }));
    await screen.findByRole("toolbar", { name: "Tools" });
    useAppStore.getState().setGrid3d(false);
    expect(useAppStore.getState().grid3d).toBe(false);
    await userEvent.click(screen.getByRole("button", { name: /Elevation Marks/ }));
    expect(screen.getByRole("button", { name: "Building Elevation" })).toBeTruthy();
  });

  it("sketches floors and moves elements from 3D (ADR-025)", async () => {
    for (const t of ["sketch", "move", "copy"] as const)
      expect(toolAllowed(t, "ThreeD")).toBe(true);
    const app = appState(null);
    fake.state = app;
    const v3d = app.views.find((v) => v.viewType === "ThreeD")!;
    const plan = app.views.find((v) => v.viewType === "Plan")!;
    useAppStore.setState({ app, openViews: [v3d.id], activeView: v3d.id, level3d: null });
    await startSketch("Floor");
    const call = fake.calls.find((c) => c.cmd === "sketch_begin")!;
    // The options bar's level (the first when none is picked) sets the work plane.
    expect(call.args).toMatchObject({ view: v3d.id, kind: "Floor", level: app.levels[0]!.id });
    const s = useAppStore.getState();
    expect(s.tool).toBe("sketch");
    expect(s.sketchUi.mode).toBe("PickWalls");
    expect(s.app?.sketch?.view).toBe(plan.id);
    expect(prompt3d("sketch")).toContain("work plane");
    expect(prompt3d("move", 1)).toContain("end point");
  });
});
