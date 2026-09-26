import { useEffect, useState } from "react";
import { ipc } from "../ipc";
import { useAppStore } from "../store";
import { PaintIcon } from "./MaterialBrowser";

// The Paint tool's chip (ADR-034): which material the brush carries, and how to use it.

export function PaintChip() {
  const tool = useAppStore((s) => s.tool);
  const material = useAppStore((s) => s.paintMaterial);
  const revision = useAppStore((s) => s.app?.revision ?? 0);
  const [info, setInfo] = useState<{ name: string; color: [number, number, number] } | null>(null);
  useEffect(() => {
    if (tool !== "paint" || !material) return;
    let live = true;
    ipc.renderMaterials().then(
      (list) => {
        const m = list.find((x) => x.id === material);
        if (live) setInfo(m ? { name: m.name, color: m.color } : null);
      },
      () => {},
    );
    return () => {
      live = false;
    };
  }, [tool, material, revision]);
  if (tool !== "paint") return null;
  const s = useAppStore.getState();
  return (
    <div className="paint-chip" role="status" aria-label="Paint">
      <span className="paint-chip-icon">
        <PaintIcon size={18} />
      </span>
      <span
        className="paint-chip-swatch"
        style={{ background: info ? `rgb(${info.color.join(",")})` : "#ccc" }}
      />
      <span className="paint-chip-text">
        <strong>{info?.name ?? "Pick a material"}</strong>
        <span>Click to paint · Shift-click paints the whole type · Esc to finish</span>
      </span>
      <button className="btn-ghost" onClick={() => s.setUi({ viewDialog: "materials" })}>
        Change
      </button>
      <button className="btn-outline" onClick={() => s.setTool("select")}>
        Done
      </button>
    </div>
  );
}
