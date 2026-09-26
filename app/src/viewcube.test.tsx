import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import * as THREE from "three";
import { ViewCubeOverlay } from "./components/ViewCubeOverlay";
import {
  alignedOf,
  fitDistance,
  HOME_DIR,
  savedHome,
  spotAt,
  spotDir,
  spotName,
  ViewCube,
  type CubeHost,
} from "./render/viewCube";

function host(
  box: THREE.Box3 | null = new THREE.Box3(
    new THREE.Vector3(0, 0, 0),
    new THREE.Vector3(12000, 9000, 6000),
  ),
): CubeHost {
  const camera = new THREE.PerspectiveCamera(40, 1.5, 50, 1e7);
  camera.up.set(0, 0, 1);
  camera.position.set(-20000, -30000, 15000);
  const target = new THREE.Vector3(6000, 4500, 3000);
  camera.lookAt(target);
  return { camera, target, bounds: () => box, home: () => null };
}

const dirOf = (h: CubeHost) => h.camera.position.clone().sub(h.target).normalize();

describe("ViewCube (ADR-037)", () => {
  it("has Revit's 26 hotspots: faces, edges and corners", () => {
    expect(spotAt(new THREE.Vector3(0.1, -1, 0.2))).toEqual([0, -1, 0]);
    expect(spotAt(new THREE.Vector3(0.9, -1, 0.1))).toEqual([1, -1, 0]);
    expect(spotAt(new THREE.Vector3(0.9, -1, 0.8))).toEqual([1, -1, 1]);
    expect(spotName([1, -1, 1])).toBe("Top, Front, Right");
    expect(spotName([0, 0, -1])).toBe("Bottom");
    // Top looks straight down with north up the screen: a hair to the south.
    const top = spotDir([0, 0, 1]);
    expect(top.z).toBeCloseTo(1, 6);
    expect(top.y).toBeLessThan(0);
    expect(spotDir([1, -1, 1]).x).toBeCloseTo(1 / Math.sqrt(3), 9);
  });

  it("clicking a face turns the view to it and fits the model", () => {
    const h = host();
    const cube = new ViewCube(h);
    cube.click({ kind: "cube", spot: [0, -1, 0] });
    expect(cube.turning).toBe(true);
    cube.step(Infinity);
    expect(cube.turning).toBe(false);
    const d = dirOf(h);
    expect(d.x).toBeCloseTo(0, 6);
    expect(d.y).toBeCloseTo(-1, 6);
    // Fitted: about the model's bounding sphere, at the distance that shows it whole.
    const sphere = new THREE.Box3(
      new THREE.Vector3(0, 0, 0),
      new THREE.Vector3(12000, 9000, 6000),
    ).getBoundingSphere(new THREE.Sphere());
    expect(h.target.distanceTo(sphere.center)).toBeLessThan(1e-6);
    const dist = h.camera.position.distanceTo(h.target);
    expect(dist).toBeCloseTo(fitDistance(sphere.radius, 40, 1.5), 3);
    // 40° tall: radius / sin(20°), with 2% to spare.
    expect(fitDistance(1000, 40, 1.5)).toBeCloseTo((1000 / Math.sin(Math.PI / 9)) * 1.02, 6);
  });

  it("straight at a face, the arrows lead to the faces around it", () => {
    const h = host();
    const cube = new ViewCube(h);
    cube.orient(spotDir([0, -1, 0]), { animate: false });
    const a = alignedOf(h.camera, h.target)!;
    expect(a.face).toEqual([0, -1, 0]);
    expect(a.up).toEqual([0, 0, 1]);
    expect(a.right).toEqual([1, 0, 0]);
    expect(a.left).toEqual([-1, 0, 0]);
    expect(a.down).toEqual([0, 0, -1]);
    // From the top, up the screen is north (Back).
    cube.orient(spotDir([0, 0, 1]), { animate: false });
    expect(alignedOf(h.camera, h.target)!.up).toEqual([0, 1, 0]);
    // A corner isn't straight at a face.
    cube.orient(spotDir([1, -1, 1]), { animate: false });
    expect(alignedOf(h.camera, h.target)).toBeNull();
  });

  it("a compass letter looks from that side at the same height; dragging orbits", () => {
    const h = host();
    const cube = new ViewCube(h);
    const z0 = dirOf(h).z;
    cube.click({ kind: "letter", dir: [1, 0, 0], side: "east" });
    cube.step(Infinity);
    const d = dirOf(h);
    expect(d.z).toBeCloseTo(z0, 6);
    expect(d.y).toBeCloseTo(0, 6);
    expect(d.x).toBeGreaterThan(0);
    // Dragging right turns the camera clockwise seen from above (the cube turns with the
    // pointer), keeping the distance.
    const r = h.camera.position.distanceTo(h.target);
    cube.orbit(100, 0);
    const after = dirOf(h);
    expect(after.y).toBeLessThan(0);
    expect(h.camera.position.distanceTo(h.target)).toBeCloseTo(r, 3);
  });

  it("home is the view's first fit until one is set", () => {
    const h = host();
    const cube = new ViewCube(h);
    cube.goHome(false);
    expect(dirOf(h).angleTo(HOME_DIR)).toBeLessThan(1e-6);
  });

  it("the menu sets and resets this view's home", async () => {
    const h = { ...host(), home: () => savedHome("v-home-test") };
    const cube = new ViewCube(h);
    render(<ViewCubeOverlay cube={cube} viewId="v-home-test" />);
    expect(screen.getByRole("group", { name: "ViewCube" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Home" })).toBeTruthy();
    await userEvent.click(screen.getByRole("button", { name: "ViewCube menu" }));
    const reset = screen.getByRole("menuitem", { name: "Reset Home" }) as HTMLButtonElement;
    expect(reset.disabled).toBe(true);
    await userEvent.click(screen.getByRole("menuitem", { name: "Set Current View as Home" }));
    const saved = savedHome("v-home-test")!;
    expect(saved.eye.distanceTo(h.camera.position)).toBeLessThan(1e-6);
    // Home goes back there after moving away.
    cube.orient(spotDir([0, 0, 1]), { animate: false });
    await userEvent.click(screen.getByRole("button", { name: "Home" }));
    cube.step(Infinity);
    expect(h.camera.position.distanceTo(saved.eye)).toBeLessThan(1e-3);
    await userEvent.click(screen.getByRole("button", { name: "ViewCube menu" }));
    await userEvent.click(screen.getByRole("menuitem", { name: "Reset Home" }));
    expect(savedHome("v-home-test")).toBeNull();
  });
});
