import type { ReactNode } from "react";
import { deleteSelection } from "../fileActions";
import { activeViewInfo, useAppStore, type Tool } from "../store";
import { toolAllowed } from "../tools";
import { Icons } from "./Icons";

function ToolButton({
  tool,
  label,
  icon,
  keys,
}: {
  tool: Tool;
  label: string;
  icon: ReactNode;
  keys: string;
}) {
  const active = useAppStore((s) => s.tool === tool);
  const view = useAppStore((s) => activeViewInfo(s)?.viewType);
  const setTool = useAppStore((s) => s.setTool);
  const allowed = toolAllowed(tool, view);
  return (
    <button
      className={`rb-btn${active ? " active" : ""}`}
      onClick={() => setTool(tool)}
      aria-pressed={active}
      disabled={!allowed}
      title={`${label} (${keys})${allowed ? "" : " — not available in this view"}`}
    >
      {icon}
      <span>{label}</span>
    </button>
  );
}

function Group({ title, children }: { title: string; children: ReactNode }) {
  return (
    <div className="rb-group">
      <div className="rb-items">{children}</div>
      <div className="rb-title">{title}</div>
    </div>
  );
}

export function Ribbon() {
  const hasSelection = useAppStore((s) => s.selection.length > 0);
  const app = useAppStore((s) => s.app);
  const openView = useAppStore((s) => s.openView);
  const view3d = app?.views.find((v) => v.viewType === "ThreeD");
  return (
    <div className="ribbon" role="toolbar" aria-label="Tools">
      <div className="rb-tabs">
        <span className="rb-tab active">Architecture</span>
      </div>
      <div className="rb-body">
        <Group title="Select">
          <ToolButton tool="select" label="Modify" icon={Icons.select} keys="MD / Esc" />
        </Group>
        <Group title="Build">
          <ToolButton tool="wall" label="Wall" icon={Icons.wall} keys="WA" />
          <ToolButton tool="door" label="Door" icon={Icons.door} keys="DR" />
          <ToolButton tool="window" label="Window" icon={Icons.window} keys="WN" />
          <ToolButton
            tool="floorAuto"
            label="Floor"
            icon={Icons.floorAuto}
            keys="FP — pick walls"
          />
          <ToolButton tool="floor" label="Floor Sketch" icon={Icons.floor} keys="SB" />
          <ToolButton
            tool="ceilingAuto"
            label="Ceiling"
            icon={Icons.ceiling}
            keys="CL — auto room"
          />
          <ToolButton tool="ceiling" label="Ceiling Sketch" icon={Icons.floor} keys="CS" />
        </Group>
        <Group title="Datum">
          <ToolButton tool="level" label="Level" icon={Icons.level} keys="LL" />
          <ToolButton tool="grid" label="Grid" icon={Icons.grid} keys="GR" />
        </Group>
        <Group title="Modify">
          <button
            className="rb-btn"
            onClick={() => void deleteSelection()}
            disabled={!hasSelection}
            title="Delete (Del)"
          >
            {Icons.del}
            <span>Delete</span>
          </button>
        </Group>
        <Group title="View">
          <button
            className="rb-btn"
            onClick={() => view3d && openView(view3d.id)}
            disabled={!view3d}
            title="Default 3D view"
          >
            {Icons.view3d}
            <span>3D View</span>
          </button>
          <button
            className="rb-btn"
            onClick={() => window.dispatchEvent(new Event("view-fit"))}
            title="Zoom to fit (ZF)"
          >
            {Icons.fit}
            <span>Zoom Fit</span>
          </button>
        </Group>
      </div>
    </div>
  );
}
