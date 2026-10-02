import { describe, expect, it } from "vitest";
import { clumpGeometry } from "./render/grass";

describe("Lawn blades (ADR-101)", () => {
  it("are finite, face up and cover the ground", () => {
    for (const kind of ["Lawn", "LushLawn"] as const) {
      const g = clumpGeometry(kind, [98, 126, 38]);
      const p = g.getAttribute("position").array;
      const n = g.getAttribute("normal").array;
      const c = g.getAttribute("color").array;
      for (const a of [p, n, c])
        for (let i = 0; i < a.length; i++) expect(Number.isFinite(a[i])).toBe(true);
      let up = 0;
      for (let i = 2; i < n.length; i += 3) if (n[i]! > 0) up++;
      expect(up / (n.length / 3)).toBeGreaterThan(0.95);
      // The leaves reach out from the clump: its footprint is wide, not a tuft.
      let rmax = 0;
      let zmax = 0;
      for (let i = 0; i < p.length; i += 3) {
        rmax = Math.max(rmax, Math.hypot(p[i]!, p[i + 1]!));
        zmax = Math.max(zmax, p[i + 2]!);
      }
      expect(rmax).toBeGreaterThan(1.1);
      expect(zmax).toBeLessThan(1.25);
      expect(zmax).toBeGreaterThan(0.5);
      // Each face points the way its normals do (a path tracer flips them otherwise).
      const ix = g.getIndex()!.array;
      let agree = 0;
      for (let t = 0; t < ix.length; t += 3) {
        const [a, b, c2] = [ix[t]!, ix[t + 1]!, ix[t + 2]!].map((v) => v * 3);
        const e1 = [p[b!]! - p[a!]!, p[b! + 1]! - p[a! + 1]!, p[b! + 2]! - p[a! + 2]!];
        const e2 = [p[c2!]! - p[a!]!, p[c2! + 1]! - p[a! + 1]!, p[c2! + 2]! - p[a! + 2]!];
        const fn = [
          e1[1]! * e2[2]! - e1[2]! * e2[1]!,
          e1[2]! * e2[0]! - e1[0]! * e2[2]!,
          e1[0]! * e2[1]! - e1[1]! * e2[0]!,
        ];
        const vn = [n[a!]!, n[a! + 1]!, n[a! + 2]!];
        if (fn[0]! * vn[0]! + fn[1]! * vn[1]! + fn[2]! * vn[2]! > 0) agree++;
      }
      expect(agree / (ix.length / 3)).toBeGreaterThan(0.97);
    }
  });
});
