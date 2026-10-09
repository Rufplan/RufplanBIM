import { describe, expect, it } from "vitest";
import type * as THREE from "three";
import * as PT from "three-gpu-pathtracer";
import { patchShader, patchTracer } from "./ptPatch";

// Exported by the library, though its type declarations leave it out.
const PhysicalPathTracingMaterial = (
  PT as unknown as { PhysicalPathTracingMaterial: new () => THREE.ShaderMaterial }
).PhysicalPathTracingMaterial;

// The light transport edits (ADR-118) match the installed three-gpu-pathtracer exactly.
describe("path tracer patches", () => {
  it("apply to the installed library's shader, every one", () => {
    const m = new PhysicalPathTracingMaterial();
    const { shader, missing } = patchShader(m.fragmentShader);
    expect(missing).toEqual([]);
    expect(shader).toContain("uniform float envPick;");
    expect(shader).toContain("rand( 5 ) >= envSel");
    expect(shader).not.toContain("/ lightsDenom");
    expect(shader).toContain("m.diffuseTrans = s14.g > 1.5;");
    expect(shader!.match(/bool diffuseTrans;/g)).toHaveLength(2);
    expect(shader).toContain("clampMax( nee, clampIndirect )");
  });

  it("set their uniforms, and leave the stock shader when they don't match", () => {
    const m = new PhysicalPathTracingMaterial();
    expect(patchTracer(m, { envPick: 0.5, clampIndirect: 10 })).toBe(true);
    expect(m.uniforms.envPick!.value).toBe(0.5);
    const other = new PhysicalPathTracingMaterial();
    other.fragmentShader = "void main() {}";
    expect(patchTracer(other, { envPick: 0.5, clampIndirect: 10 })).toBe(false);
    expect(other.fragmentShader).toBe("void main() {}");
  });
});
