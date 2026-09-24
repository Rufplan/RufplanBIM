import { useState, type ReactNode } from "react";
import {
  apply,
  deleteSelection,
  exportIfc,
  exportPdf,
  issueSet,
  newSheet,
  placeOnActiveSheet,
  tagAll,
} from "../fileActions";
import { ipc } from "../ipc";
import { refreshCloud } from "../rufplan";
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

type Tab = "Architecture" | "Structure" | "Modify" | "Annotate" | "View" | "Manage" | "Rufplan";
const TABS: Tab[] = [
  "Architecture",
  "Structure",
  "Modify",
  "Annotate",
  "View",
  "Manage",
  "Rufplan",
];

export function Ribbon() {
  const [tab, setTab] = useState<Tab>("Architecture");
  const hasSelection = useAppStore((s) => s.selection.length > 0);
  const app = useAppStore((s) => s.app);
  const openView = useAppStore((s) => s.openView);
  const view3d = app?.views.find((v) => v.viewType === "ThreeD");
  const activeIsSheet = useAppStore((s) => activeViewInfo(s)?.viewType === "Sheet");
  const activeIsPlan = useAppStore((s) => activeViewInfo(s)?.viewType === "Plan");
  const cloud = useAppStore((s) => s.cloud);
  const setRufplan = useAppStore((s) => s.setRufplan);
  const linked = app?.rufplan ?? null;
  const selection = useAppStore((s) => s.selection);
  const setParamsOpen = useAppStore((s) => s.setParamsOpen);
  // Drawing views go on one sheet only; schedules can repeat; 3D and sheets can't be placed.
  const placeable = (app?.views ?? []).filter(
    (v) =>
      v.viewType === "Schedule" ||
      (v.viewType !== "ThreeD" && v.viewType !== "Sheet" && v.onSheet === null),
  );
  return (
    <div className="ribbon" role="toolbar" aria-label="Tools">
      <div className="rb-tabs" role="tablist" aria-label="Ribbon tabs">
        {TABS.map((t) => (
          <button
            key={t}
            role="tab"
            aria-selected={tab === t}
            className={`rb-tab${tab === t ? " active" : ""}`}
            onClick={() => {
              setTab(t);
              if (t === "Rufplan" && !useAppStore.getState().cloud) void refreshCloud();
            }}
          >
            {t}
          </button>
        ))}
      </div>
      <div className="rb-body">
        <Group title="Select">
          <ToolButton tool="select" label="Modify" icon={Icons.select} keys="MD / Esc" />
        </Group>
        {tab === "Architecture" && (
          <>
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
            <Group title="Roof & Circulation">
              <ToolButton tool="roof" label="Roof" icon={Icons.roof} keys="RF — by footprint" />
              <ToolButton tool="stair" label="Stair" icon={Icons.stair} keys="ST" />
              <ToolButton
                tool="railing"
                label="Railing"
                icon={Icons.railing}
                keys="RA — sketch path"
              />
              <ToolButton tool="column" label="Column" icon={Icons.column} keys="SC" />
            </Group>
            <Group title="Room">
              <ToolButton tool="room" label="Room" icon={Icons.room} keys="RM" />
            </Group>
          </>
        )}
        {tab === "Structure" && (
          <Group title="Structure">
            <ToolButton tool="column" label="Column" icon={Icons.column} keys="SC" />
            <button
              className="rb-btn"
              onClick={() => {
                const s = useAppStore.getState();
                const v = activeViewInfo(s);
                if (v) void apply(() => ipc.columnsAtGrids(v.id, s.toolTypes.column));
              }}
              disabled={!activeIsPlan}
              title="Place a column at every grid intersection on this plan's level"
            >
              {Icons.columnGrid}
              <span>At Grids</span>
            </button>
            <ToolButton tool="beam" label="Beam" icon={Icons.beam} keys="BM" />
          </Group>
        )}
        {tab === "Modify" && (
          <>
            <Group title="Modify">
              <ToolButton tool="copy" label="Copy" icon={Icons.copy} keys="CO — select first" />
              <ToolButton tool="rotate" label="Rotate" icon={Icons.rotate} keys="RO" />
              <ToolButton tool="mirror" label="Mirror" icon={Icons.mirror} keys="MM — draw axis" />
              <ToolButton tool="array" label="Array" icon={Icons.array} keys="AR" />
              <ToolButton tool="align" label="Align" icon={Icons.align} keys="AL" />
              <ToolButton tool="trim" label="Trim/Extend" icon={Icons.trim} keys="TR — corner" />
              <ToolButton tool="offset" label="Offset" icon={Icons.offset} keys="OF" />
              <ToolButton tool="split" label="Split" icon={Icons.split} keys="SL" />
              <button
                className="rb-btn"
                onClick={() => void apply(() => ipc.flipSelection(selection))}
                disabled={selection.length === 0}
                title="Flip walls, doors and windows (Space)"
              >
                {Icons.flip}
                <span>Flip</span>
              </button>
              <button
                className="rb-btn"
                onClick={() => void apply(() => ipc.attachWallTops(selection, true))}
                disabled={selection.length === 0}
                title="Attach the selected walls' tops to the roof above"
              >
                {Icons.attach}
                <span>Attach Top</span>
              </button>
              <button
                className="rb-btn"
                onClick={() => void apply(() => ipc.attachWallTops(selection, false))}
                disabled={selection.length === 0}
                title="Detach the selected walls' tops from the roof"
              >
                {Icons.del}
                <span>Detach Top</span>
              </button>
            </Group>
          </>
        )}
        <Group title="Move">
          <ToolButton tool="move" label="Move" icon={Icons.move} keys="MV — select first" />
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
        {tab === "Annotate" && (
          <Group title="Annotate">
            <ToolButton tool="dimension" label="Dimension" icon={Icons.dimension} keys="DI" />
            <ToolButton tool="text" label="Text" icon={Icons.text} keys="TX" />
            <button
              className="rb-btn"
              onClick={() => void tagAll()}
              disabled={!activeIsPlan}
              title="Tag every untagged door, window and room in this floor plan"
            >
              {Icons.tag}
              <span>Tag All</span>
            </button>
          </Group>
        )}
        {tab === "View" && (
          <>
            <Group title="View">
              <ToolButton
                tool="section"
                label="Section"
                icon={Icons.section}
                keys="draw in a plan"
              />
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
            <Group title="Sheets">
              <button className="rb-btn" onClick={() => void newSheet()} title="New ARCH D sheet">
                {Icons.sheet}
                <span>New Sheet</span>
              </button>
              <label
                className={`rb-place${activeIsSheet ? "" : " disabled"}`}
                title="Place a view on the open sheet"
              >
                {Icons.place}
                <select
                  aria-label="Place view on sheet"
                  value=""
                  disabled={!activeIsSheet}
                  onChange={(e) => e.target.value && void placeOnActiveSheet(e.target.value)}
                >
                  <option value="">Place View</option>
                  {placeable.map((v) => (
                    <option key={v.id} value={v.id}>
                      {v.name} ({v.viewType === "CeilingPlan" ? "Ceiling Plan" : v.viewType})
                    </option>
                  ))}
                </select>
              </label>
              <button
                className="rb-btn"
                onClick={() => void exportPdf()}
                title="Export all sheets to PDF (Ctrl+P)"
              >
                {Icons.pdf}
                <span>Export PDF</span>
              </button>
              <button
                className="rb-btn"
                onClick={() => void exportIfc()}
                title="Export the model to IFC4 for consultants and other BIM tools"
              >
                {Icons.ifc}
                <span>Export IFC</span>
              </button>
              <button
                className="rb-btn"
                onClick={() => void issueSet()}
                title="Issue the current design stage's sheet set: record it and export the PDF"
              >
                {Icons.issue}
                <span>Issue Set</span>
              </button>
            </Group>
          </>
        )}
        {tab === "Manage" && (
          <Group title="Settings">
            <button
              className="rb-btn"
              onClick={() => setParamsOpen(true)}
              title="Add your own parameters to categories (like Revit's Project Parameters)"
            >
              {Icons.params}
              <span>Project Parameters</span>
            </button>
          </Group>
        )}
        {tab === "Rufplan" && (
          <Group title="Rufplan.io">
            <button
              className="rb-btn"
              onClick={() => setRufplan("account")}
              title={cloud?.signedIn ? `Signed in as ${cloud.email ?? ""}` : "Sign in to Rufplan"}
            >
              {Icons.account}
              <span className="rb-clip">
                {cloud?.signedIn ? (cloud.name ?? "Account") : "Sign In"}
              </span>
            </button>
            <button
              className="rb-btn"
              onClick={() => setRufplan("link")}
              title={linked ? `Linked to ${linked.name}` : "Link this model to a Rufplan project"}
            >
              {Icons.link}
              <span className="rb-clip">{linked ? linked.name : "Link Project"}</span>
            </button>
            <button
              className="rb-btn"
              onClick={() => setRufplan("publish")}
              disabled={!linked}
              title="Publish the current stage's set (PDF + IFC) to the linked Rufplan project"
            >
              {Icons.publish}
              <span>Publish</span>
            </button>
          </Group>
        )}
      </div>
    </div>
  );
}
