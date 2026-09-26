import { describe, expect, it } from "vitest";
import { useState } from "react";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import * as THREE from "three";
import { VisualStyleToggle } from "./components/VisualStyleToggle";
import {
  styleMaterial,
  VISUAL_STYLES,
  type MeshInfo,
  type VisualStyle,
} from "./render/visualStyle";
import { runAction } from "./actions";
import { styleOf, useAppStore } from "./store";
import type { Mesh } from "./ipc";

function Harness({ start = "shaded" as VisualStyle }) {
  const [v, setV] = useState<VisualStyle>(start);
  return <VisualStyleToggle value={v} onChange={setV} />;
}

const wall: Mesh = {
  el: "w1",
  category: "Wall",
  exterior: true,
  color: [168, 154, 124],
  material: null,
  level: null,
  positions: [],
  edges: [],
};
const info: MeshInfo = { mesh: wall, base: 0xa89a7c, map: null };

describe("visual styles (ADR-038)", () => {
  it("the pill shows the current style and opens to all five on hover", async () => {
    render(<Harness />);
    const group = screen.getByRole("radiogroup", { name: "Visual Style" });
    // At rest: only the current style.
    expect(screen.getAllByRole("radio").map((b) => b.getAttribute("aria-label"))).toEqual([
      "Shaded",
    ]);
    await userEvent.hover(group);
    const all = screen.getAllByRole("radio");
    expect(all.map((b) => b.getAttribute("aria-label"))).toEqual(VISUAL_STYLES.map((s) => s.label));
    expect(screen.getByRole("radio", { name: "Shaded" }).getAttribute("aria-checked")).toBe("true");
    // Choosing keeps it open while the pointer is over it.
    await userEvent.click(screen.getByRole("radio", { name: "Realistic" }));
    expect(screen.getByRole("radio", { name: "Realistic" }).getAttribute("aria-checked")).toBe(
      "true",
    );
    expect(screen.getAllByRole("radio")).toHaveLength(5);
    await userEvent.unhover(group);
    expect(screen.getAllByRole("radio").map((b) => b.getAttribute("aria-label"))).toEqual([
      "Realistic",
    ]);
  });

  it("each style draws faces its own way", () => {
    const m = (s: VisualStyle) => styleMaterial(s, info, [], null);
    // Wireframe: no faces, but still pickable (a material that doesn't draw).
    expect(m("wireframe").visible).toBe(false);
    const hidden = m("hiddenLine") as THREE.MeshBasicMaterial;
    expect(hidden).toBeInstanceOf(THREE.MeshBasicMaterial);
    expect(hidden.color.getHex()).toBe(0xffffff);
    const shaded = m("shaded") as THREE.MeshLambertMaterial;
    expect(shaded).toBeInstanceOf(THREE.MeshLambertMaterial);
    expect(shaded.color.getHex()).toBe(0xa89a7c);
    // Consistent Colors: the same colour, unlit.
    const flat = m("consistent") as THREE.MeshBasicMaterial;
    expect(flat).toBeInstanceOf(THREE.MeshBasicMaterial);
    expect(flat.color.getHex()).toBe(0xa89a7c);
    expect(flat.toneMapped).toBe(false);
    // Realistic without a project material: a physically based surface.
    expect(m("realistic")).toBeInstanceOf(THREE.MeshStandardMaterial);
    // Glass stays see-through in Shaded.
    const glass = styleMaterial(
      "shaded",
      { ...info, mesh: { ...wall, category: "Window", color: null } },
      [],
      null,
    );
    expect(glass.transparent).toBe(true);
  });

  it("the style belongs to each 3D view; the View tab and shortcuts set the active one's", async () => {
    useAppStore.setState({ activeView: "v1", visualStyles: {} });
    expect(styleOf(useAppStore.getState(), "v1")).toBe("shaded");
    await runAction("realistic");
    expect(styleOf(useAppStore.getState(), "v1")).toBe("realistic");
    expect(styleOf(useAppStore.getState(), "v2")).toBe("shaded");
    await runAction("consistent");
    expect(styleOf(useAppStore.getState(), "v1")).toBe("consistent");
  });
});
