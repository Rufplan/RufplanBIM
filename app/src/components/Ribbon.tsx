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
import { SketchRibbon } from "./SketchRibbon";
import { runAction } from "../actions";

const TEMP_LABELS = {
  hideElement: "Hide Element",
  isolateElement: "Isolate Element",
  hideCategory: "Hide Category",
  isolateCategory: "Isolate Category",
} as const;
const TEMP_KEYS = {
  hideElement: "HH",
  isolateElement: "HI",
  hideCategory: "HC",
  isolateCategory: "IC",
} as const;
const STYLE_LABELS = {
  shaded: "Shaded",
  hiddenLine: "Hidden Line",
  wireframe: "Wireframe",
} as const;
const STYLE_KEYS = { shaded: "SD", hiddenLine: "HL", wireframe: "WF" } as const;
import { editBoundary, startSketch } from "../sketch";

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

/** Creates a material (a copy of `from`) and selects it so Properties can edit it. */
async function newMaterial(from: string | null) {
  const before = new Set(useAppStore.getState().app?.materials.map((m) => m.id) ?? []);
  if (await apply(() => ipc.createMaterial(from))) {
    const s = useAppStore.getState();
    const made = s.app?.materials.find((m) => !before.has(m.id));
    if (made) s.select([made.id]);
  }
}

function Group({ title, children }: { title: string; children: ReactNode }) {
  return (
    <div className="rb-group">
      <div className="rb-items">{children}</div>
      <div className="rb-title">{title}</div>
    </div>
  );
}

type Tab =
  | "Site"
  | "Architecture"
  | "Rendering"
  | "Structure"
  | "Modify"
  | "Annotate"
  | "View"
  | "Manage"
  | "Rufplan";
const TABS: Tab[] = [
  "Site",
  "Architecture",
  "Rendering",
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
  const view3d = app?.views.find((v) => v.viewType === "ThreeD" && !v.camera);
  const cameraViews = app?.views.filter((v) => v.camera) ?? [];
  const activeIsSheet = useAppStore((s) => activeViewInfo(s)?.viewType === "Sheet");
  const activeIsPlan = useAppStore((s) => activeViewInfo(s)?.viewType === "Plan");
  const cloud = useAppStore((s) => s.cloud);
  const setRufplan = useAppStore((s) => s.setRufplan);
  const linked = app?.rufplan ?? null;
  const selection = useAppStore((s) => s.selection);
  const setParamsOpen = useAppStore((s) => s.setParamsOpen);
  const setSiteDialog = useAppStore((s) => s.setSiteDialog);
  const select = useAppStore((s) => s.select);
  const sitePlan = app?.views.find((v) => v.name === "Site" && v.viewType === "Plan");
  const selectedMaterial =
    selection.length === 1 && app?.materials.some((m) => m.id === selection[0])
      ? selection[0]!
      : null;
  // Drawing views go on one sheet only; schedules can repeat; 3D and sheets can't be placed.
  const placeable = (app?.views ?? []).filter(
    (v) =>
      v.viewType === "Schedule" ||
      (v.viewType !== "ThreeD" && v.viewType !== "Sheet" && v.onSheet === null),
  );
  const activeIsPlanLike = useAppStore((s) => {
    const t = activeViewInfo(s)?.viewType;
    return t === "Plan" || t === "CeilingPlan" || t === "ThreeD";
  });
  const activeIs3d = useAppStore((s) => activeViewInfo(s)?.viewType === "ThreeD");
  const grid3d = useAppStore((s) => s.grid3d);
  const setGrid3d = useAppStore((s) => s.setGrid3d);
  const satellite = useAppStore((s) => s.satellite);
  const setSatellite = useAppStore((s) => s.setSatellite);
  const thinLines = useAppStore((s) => s.thinLines);
  const visualStyle = useAppStore((s) => s.visualStyle);
  // Sketch mode replaces the ribbon with its contextual tab, as in Revit.
  if (app?.sketch) return <SketchRibbon />;
  const sketchButton = (
    kind: "Floor" | "Ceiling",
    label: string,
    icon: ReactNode,
    keys: string,
  ) => (
    <button
      className="rb-btn"
      onClick={() => void startSketch(kind)}
      disabled={!activeIsPlanLike}
      title={`${label} (${keys}) — sketch the boundary: pick walls, lines, rectangles, arcs…`}
    >
      {icon}
      <span>{label}</span>
    </button>
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
              {sketchButton("Floor", "Floor", Icons.floorAuto, "SB")}
              <ToolButton
                tool="ceilingAuto"
                label="Ceiling"
                icon={Icons.ceiling}
                keys="CL — auto room"
              />
              {sketchButton("Ceiling", "Sketch Ceiling", Icons.floor, "CS")}
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
              <ToolButton
                tool="roomSeparator"
                label="Room Separator"
                icon={Icons.separator}
                keys="RS — open plans"
              />
            </Group>
          </>
        )}
        {tab === "Site" && (
          <>
            <Group title="Location">
              <button
                className="rb-btn"
                onClick={() => setSiteDialog("find")}
                title="Find the lot on Google Maps and take its boundary from Regrid"
              >
                {Icons.mapPin}
                <span>Find Lot</span>
              </button>
              <button
                className="rb-btn"
                onClick={() => sitePlan && openView(sitePlan.id)}
                disabled={!sitePlan}
                title="Open the Site plan: contours, property lines, north"
              >
                {Icons.sitePlan}
                <span>Site Plan</span>
              </button>
            </Group>
            <Group title="Topography">
              <button
                className="rb-btn"
                onClick={() => setSiteDialog("find")}
                disabled={!app?.site}
                title="Get or refresh the preliminary topography from USGS 3DEP"
              >
                {Icons.topo}
                <span>{app?.site?.hasTopo ? "Refresh Topo" : "Get Topo"}</span>
              </button>
              <button
                className="rb-btn"
                onClick={() => app?.site && select([app.site.id])}
                disabled={!app?.site}
                title="Site properties: offset, angle to true north, Level 1 elevation, contour interval"
              >
                {Icons.params}
                <span>Site Settings</span>
              </button>
              <button
                className={`rb-btn${satellite && app?.site ? " active" : ""}`}
                onClick={() => setSatellite(!satellite)}
                disabled={!app?.site}
                aria-pressed={satellite}
                title="Satellite overlay: Google imagery over the topography in the Site plan and 3D"
              >
                {Icons.mapPin}
                <span>Satellite</span>
              </button>
            </Group>
            <Group title="Settings">
              <button
                className="rb-btn"
                onClick={() => setSiteDialog("keys")}
                title="Google Maps key and Regrid token (stored on this computer)"
              >
                {Icons.key}
                <span>API Keys</span>
              </button>
            </Group>
          </>
        )}
        {tab === "Rendering" && (
          <>
            <Group title="Camera">
              <ToolButton
                tool="camera"
                label="Camera"
                icon={Icons.camera}
                keys="in a plan: eye, then target"
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
              {cameraViews.length > 0 && (
                <label className="rb-select" title="Open a camera view">
                  <span>Camera Views</span>
                  <select
                    aria-label="Camera views"
                    value=""
                    onChange={(e) => e.target.value && openView(e.target.value)}
                  >
                    <option value="">Open…</option>
                    {cameraViews.map((v) => (
                      <option key={v.id} value={v.id}>
                        {v.name}
                      </option>
                    ))}
                  </select>
                </label>
              )}
            </Group>
            <Group title="Render">
              <button
                className="rb-btn"
                onClick={() => void runAction("render")}
                disabled={!activeIs3d}
                title="Render (RR): a photoreal, path-traced image of this 3D or camera view"
              >
                {Icons.render}
                <span>Render</span>
              </button>
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
                onClick={() => void runAction("pin")}
                disabled={selection.length === 0}
                title="Pin (PN): keep the selection from moving"
              >
                {Icons.pin}
                <span>Pin</span>
              </button>
              <button
                className="rb-btn"
                onClick={() => void runAction("unpin")}
                disabled={selection.length === 0}
                title="Unpin (UP)"
              >
                {Icons.unpin}
                <span>Unpin</span>
              </button>
              <button
                className="rb-btn"
                onClick={() => selection[0] && void editBoundary(selection[0])}
                disabled={selection.length !== 1}
                title="Edit the selected floor's or ceiling's boundary sketch (or double-click it)"
              >
                {Icons.floor}
                <span>Edit Boundary</span>
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
            <ToolButton tool="tag" label="Tag" icon={Icons.tag} keys="TG — by category" />
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
              <ToolButton
                tool="elevation"
                label="Elevation"
                icon={Icons.elevation}
                keys="EL — interior or building"
              />
              <ToolButton
                tool="callout"
                label="Callout"
                icon={Icons.callout}
                keys="CA — detail view"
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
                className={`rb-btn${activeIs3d && grid3d ? " active" : ""}`}
                onClick={() => setGrid3d(!grid3d)}
                disabled={!activeIs3d}
                aria-pressed={grid3d}
                title="Show the ground plane's grid in 3D"
              >
                {Icons.grid}
                <span>Ground Grid</span>
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
            <Group title="Graphics">
              <button
                className="rb-btn"
                onClick={() => void runAction("visibility")}
                disabled={!app || activeIsSheet}
                title="Visibility/Graphics (VV): categories shown in this view"
              >
                {Icons.eye}
                <span>Visibility</span>
              </button>
              <button
                className={`rb-btn${thinLines ? " active" : ""}`}
                aria-pressed={thinLines}
                onClick={() => void runAction("thinLines")}
                title="Thin Lines (TL)"
              >
                {Icons.thin}
                <span>Thin Lines</span>
              </button>
              {(["hideElement", "isolateElement", "hideCategory", "isolateCategory"] as const).map(
                (a) => (
                  <button
                    key={a}
                    className="rb-btn rb-small"
                    onClick={() => void runAction(a)}
                    disabled={selection.length === 0}
                    title={`Temporary ${TEMP_LABELS[a]} (${TEMP_KEYS[a]})`}
                  >
                    {Icons.eye}
                    <span>{TEMP_LABELS[a]}</span>
                  </button>
                ),
              )}
              <button
                className="rb-btn rb-small"
                onClick={() => void runAction("resetTemporary")}
                title="Reset Temporary Hide/Isolate (HR)"
              >
                {Icons.eye}
                <span>Reset Hide</span>
              </button>
              {activeIs3d &&
                (["shaded", "hiddenLine", "wireframe"] as const).map((v) => (
                  <button
                    key={v}
                    className={`rb-btn rb-small${visualStyle === v ? " active" : ""}`}
                    onClick={() => void runAction(v)}
                    title={`${STYLE_LABELS[v]} (${STYLE_KEYS[v]})`}
                  >
                    {Icons.view3d}
                    <span>{STYLE_LABELS[v]}</span>
                  </button>
                ))}
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
          <Group title="Materials">
            <button
              className="rb-btn"
              onClick={() => void newMaterial(null)}
              title="Add a material (cut pattern, surface pattern and color); edit it in Properties"
            >
              {Icons.material}
              <span>New Material</span>
            </button>
            <button
              className="rb-btn"
              onClick={() => void newMaterial(selectedMaterial)}
              disabled={!selectedMaterial}
              title="Duplicate the selected material"
            >
              {Icons.copy}
              <span>Duplicate</span>
            </button>
          </Group>
        )}
        {tab === "Manage" && (
          <Group title="Settings">
            <button
              className="rb-btn"
              onClick={() => void runAction("keyboard")}
              title="Keyboard Shortcuts (KS): Revit's two-letter keys, and your own"
            >
              {Icons.key}
              <span>Shortcuts</span>
            </button>
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
