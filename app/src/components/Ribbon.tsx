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
import { dialogs, errorMessage, ipc } from "../ipc";
import { refreshCloud } from "../rufplan";
import { activeViewInfo, styleOf, useAppStore, type Tool } from "../store";
import { setSeason } from "../vegetation";
import { toolAllowed } from "../tools";
import { Icons } from "./Icons";
import { SketchRibbon } from "./SketchRibbon";
import { InPlaceRibbon } from "./InPlaceRibbon";
import { TextRibbon } from "./TextRibbon";
import { NewViewMenu } from "./NewViewMenu";
import { ActiveWorkset } from "./Worksets";
import type { Discipline } from "../bindings/Discipline";
import { modelInPlace } from "../inplace";
import { startFilledRegion } from "../details";
import { startComponent } from "../components";
import { ProjectInfoRibbon } from "./ProjectInfo";
import { SpecsRibbon } from "./Specs";
import { StandardsRibbon } from "./Standards";
import {
  ContextPanels,
  ContextPropertiesGroup,
  contextLabel,
  useContextualSwitch,
  useSelectionCategories,
} from "./ModifyContext";
import { runAction, openPicker, startTool } from "../actions";

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
const STYLE_KEYS: Partial<Record<VisualStyle, string>> = {
  shaded: "SD",
  hiddenLine: "HL",
  wireframe: "WF",
};
import { startSketch } from "../sketch";
import { VISUAL_STYLES, type VisualStyle } from "../render/visualStyle";
import { StyleIcon } from "./VisualStyleToggle";
import { clock } from "./LightingDialogs";

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
  const allowed = toolAllowed(tool, view);
  return (
    <button
      className={`rb-btn${active ? " active" : ""}`}
      onClick={() => void startTool(tool)}
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

/** Annotate > Keynote (ADR-081): Revit's three keynote tools, the Keynote Manager and
 * the Keynote Legend. */
function KeynoteGroup() {
  const app = useAppStore((s) => s.app);
  const setUi = useAppStore((s) => s.setUi);
  const openLegend = async () => {
    const s = useAppStore.getState();
    try {
      const [id, state] = await ipc.keynoteLegend();
      if (state) s.setApp(state);
      useAppStore.getState().openView(id);
    } catch (e) {
      s.setError(errorMessage(e));
    }
  };
  return (
    <Group title="Keynote">
      <ToolButton
        tool="keynoteElement"
        label="Element"
        icon={Icons.keynoteElement}
        keys="KE — the type's keynote"
      />
      <ToolButton
        tool="keynoteMaterial"
        label="Material"
        icon={Icons.keynoteMaterial}
        keys="KM — a material's keynote"
      />
      <ToolButton
        tool="keynoteUser"
        label="User"
        icon={Icons.keynoteUser}
        keys="KU — any keynote"
      />
      <button
        className="rb-btn"
        disabled={!app}
        title="Keynote Manager: browse, edit, load and assign keynotes"
        onClick={() => setUi({ viewDialog: "keynotes" })}
      >
        {Icons.keynoteManager}
        <span>Manager</span>
      </button>
      <button
        className="rb-btn"
        disabled={!app}
        title="Keynote Legend: the keynotes used, filtered to its sheet when placed"
        onClick={() => void openLegend()}
      >
        {Icons.keynoteLegend}
        <span>Legend</span>
      </button>
    </Group>
  );
}

/** MEPT (ADR-082): Mechanical, Electrical, Plumbing and Technology, each with its
 * Suggest, overlay and export; one opacity for them all. */
function MeptRibbon() {
  const app = useAppStore((s) => s.app);
  const on = useAppStore((s) => s.mepOverlay);
  const setOn = useAppStore((s) => s.setMepOverlay);
  const alpha = useAppStore((s) => s.structuralAlpha);
  const setAlpha = useAppStore((s) => s.setStructuralAlpha);
  const setUi = useAppStore((s) => s.setUi);
  const setDiscipline = useAppStore((s) => s.setMepDiscipline);
  const icons: Record<Discipline, ReactNode> = {
    Mechanical: Icons.mepMechanical,
    Electrical: Icons.mepElectrical,
    Plumbing: Icons.mepPlumbing,
    Technology: Icons.mepTechnology,
  };
  const exportJson = async (d: Discipline) => {
    const s = useAppStore.getState();
    const path = await dialogs.pickJsonLocation(
      `${app?.projectName || "Project"} ${d} (Preliminary)`,
    );
    if (!path) return;
    try {
      s.setPrompt(
        `Exported the preliminary ${d.toLowerCase()} layer: ${await ipc.mepExportJson(d, path)}`,
      );
    } catch (e) {
      s.setError(errorMessage(e));
    }
  };
  return (
    <>
      {(["Mechanical", "Electrical", "Plumbing", "Technology"] as Discipline[]).map((d) => {
        const has = !!app?.mepLayers.includes(d);
        const shown = on.includes(d);
        return (
          <Group key={d} title={d}>
            <button
              className="rb-btn"
              disabled={!app}
              title={`Suggest ${d}: rank the systems for this model (preliminary)`}
              onClick={() => {
                setDiscipline(d);
                setUi({ viewDialog: "mep" });
              }}
            >
              {icons[d]}
              <span>Suggest</span>
            </button>
            <button
              className={`rb-btn${shown ? " active" : ""}`}
              aria-pressed={shown}
              aria-label={`${d} overlay`}
              disabled={!has}
              title={
                has
                  ? `Show the ${d.toLowerCase()} layer over the greyed-out architecture`
                  : "Generate a layer from Suggest first"
              }
              onClick={() => setOn(d, !shown)}
            >
              {Icons.structureOverlay}
              <span>Overlay</span>
            </button>
            <div className="rb-stack">
              <button
                className="rb-text"
                disabled={!has}
                aria-label={`Export ${d} JSON`}
                onClick={() => void exportJson(d)}
              >
                Export JSON
              </button>
            </div>
          </Group>
        );
      })}
      <Group title="Overlay">
        <label className="rb-field" title="Overlay transparency">
          <span>Opacity</span>
          <input
            type="range"
            aria-label="MEPT overlay opacity"
            min={20}
            max={100}
            value={Math.round(alpha * 100)}
            onChange={(e) => setAlpha(Number(e.target.value) / 100)}
          />
        </label>
      </Group>
    </>
  );
}

/** Structure > Analyze (ADR-080): Suggest Structure and the structural overlay. */
function StructureAnalyze() {
  const app = useAppStore((s) => s.app);
  const has = !!app?.structuralLayer;
  const on = useAppStore((s) => s.structuralOverlay);
  const alpha = useAppStore((s) => s.structuralAlpha);
  const setOn = useAppStore((s) => s.setStructuralOverlay);
  const setAlpha = useAppStore((s) => s.setStructuralAlpha);
  const setUi = useAppStore((s) => s.setUi);
  const exportAs = async (kind: "json" | "ifc") => {
    const s = useAppStore.getState();
    const name = `${app?.projectName || "Project"} Structure (Preliminary)`;
    const path =
      kind === "json" ? await dialogs.pickJsonLocation(name) : await dialogs.pickIfcLocation(name);
    if (!path) return;
    try {
      const done =
        kind === "json"
          ? await ipc.structuralExportJson(path)
          : await ipc.structuralExportIfc(path);
      s.setPrompt(`Exported the preliminary structural layer: ${done}`);
    } catch (e) {
      s.setError(errorMessage(e));
    }
  };
  return (
    <Group title="Analyze">
      <button
        className="rb-btn"
        disabled={!app}
        title="Suggest Structure: score six structural systems against this model (preliminary)"
        onClick={() => setUi({ viewDialog: "structure" })}
      >
        {Icons.structureSuggest}
        <span>Suggest Structure</span>
      </button>
      <button
        className={`rb-btn${on ? " active" : ""}`}
        aria-pressed={on}
        disabled={!has}
        title={
          has
            ? "Show the structural layer over the greyed-out architecture"
            : "Generate a layer from Suggest Structure first"
        }
        onClick={() => setOn(!on)}
      >
        {Icons.structureOverlay}
        <span>Overlay</span>
      </button>
      <label className="rb-field" title="Overlay transparency">
        <span>Opacity</span>
        <input
          type="range"
          aria-label="Overlay opacity"
          min={20}
          max={100}
          value={Math.round(alpha * 100)}
          disabled={!has}
          onChange={(e) => setAlpha(Number(e.target.value) / 100)}
        />
      </label>
      <div className="rb-stack">
        <button className="rb-text" disabled={!has} onClick={() => void exportAs("json")}>
          Export JSON
        </button>
        <button className="rb-text" disabled={!has} onClick={() => void exportAs("ifc")}>
          Export IFC
        </button>
      </div>
    </Group>
  );
}

/** Collaborate (ADR-079): Revit's Manage Collaboration panel. */
function CollaborateRibbon() {
  const has = useAppStore((s) => (s.app?.worksets.length ?? 0) > 0);
  const gray = useAppStore((s) => s.grayInactive);
  const setGray = useAppStore((s) => s.setGrayInactive);
  const setUi = useAppStore((s) => s.setUi);
  return (
    <Group title="Manage Collaboration">
      <button
        className="rb-btn"
        disabled={!has}
        title="Worksets: new, rename, delete, visibility"
        onClick={() => setUi({ viewDialog: "worksets" })}
      >
        {Icons.worksets}
        <span>Worksets</span>
      </button>
      <ActiveWorkset />
      <button
        className={`rb-btn${gray ? " active" : ""}`}
        aria-pressed={gray}
        disabled={!has}
        title="Gray Inactive Workset Graphics: fade what isn't on the active workset"
        onClick={() => setGray(!gray)}
      >
        {Icons.grayInactive}
        <span>Gray Inactive</span>
      </button>
    </Group>
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

type Tab =
  | "Project Info"
  | "Standards"
  | "Manage"
  | "Site"
  | "Architecture"
  | "Openings"
  | "Lighting"
  | "Modify"
  | "Details"
  | "Structure"
  | "MEPT"
  | "Materials"
  | "Landscape"
  | "Rendering"
  | "Annotate"
  | "Views"
  | "Sheets"
  | "Specifications"
  | "Collaborate"
  | "Rufplan";

/** The ribbon's tabs in numbered workflow groups (the grouped tab bar). */
const TAB_GROUPS: { label: string; tabs: Tab[] }[] = [
  { label: "SETUP", tabs: ["Project Info", "Standards", "Manage"] },
  { label: "MODEL", tabs: ["Site", "Architecture", "Openings", "Lighting", "Modify"] },
  { label: "DETAILS", tabs: ["Details"] },
  { label: "CONSULTANTS", tabs: ["Structure", "MEPT"] },
  { label: "VISUALIZE", tabs: ["Materials", "Landscape", "Rendering"] },
  { label: "DOCUMENT", tabs: ["Annotate", "Views", "Sheets", "Specifications"] },
  { label: "TEAM", tabs: ["Collaborate", "Rufplan"] },
];

export function Ribbon() {
  const tab = useAppStore((s) => s.ribbonTab) as Tab;
  const setTab = useAppStore((s) => s.setRibbonTab);
  // The group whose label or number is hovered (highlights its tabs).
  const [hoverGroup, setHoverGroup] = useState<string | null>(null);
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
  const activeIsDrafting = useAppStore((s) => activeViewInfo(s)?.viewType === "Drafting");
  const grid3d = useAppStore((s) => s.grid3d);
  const activeTool = useAppStore((s) => s.tool);
  const setGrid3d = useAppStore((s) => s.setGrid3d);
  const setUi = useAppStore((s) => s.setUi);
  const satellite = useAppStore((s) => s.satellite);
  const setSatellite = useAppStore((s) => s.setSatellite);
  const thinLines = useAppStore((s) => s.thinLines);
  const visualStyle = useAppStore((s) => styleOf(s, s.activeView));
  const sun = app?.sun;
  const sunLabel = !sun
    ? ""
    : sun.mode === "Still"
      ? `Still, ${["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"][sun.month - 1]} ${sun.day}, ${clock(sun.hour)}`
      : `Lighting, ${sun.azimuth}° azimuth, ${sun.altitude}° altitude`;
  // Selecting elements turns the Modify tab into Revit's "Modify | Walls" (ADR-055).
  const selectedCats = useSelectionCategories();
  const ctxLabel = contextLabel(selectedCats);
  useContextualSwitch(ctxLabel);
  // Sketch mode replaces the ribbon with its contextual tab, as in Revit.
  if (app?.sketch) return <SketchRibbon />;
  // So does the In-Place Editor (ADR-068).
  if (app?.inPlace) return <InPlaceRibbon />;
  // Text's contextual tab (ADR-070).
  if (activeTool === "text") return <TextRibbon />;
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
        {TAB_GROUPS.map((g, i) => {
          const hover = {
            onMouseEnter: () => setHoverGroup(g.label),
            onMouseLeave: () => setHoverGroup(null),
          };
          return (
            <div
              key={g.label}
              role="presentation"
              className={`rb-tabgroup${hoverGroup === g.label ? " hover" : ""}`}
            >
              {i > 0 && <span className="rb-tabdiv" aria-hidden />}
              <div className="rb-tabcol" role="presentation">
                <div className="rb-grouplabel" aria-hidden {...hover}>
                  {g.label}
                </div>
                <div className="rb-tabrow" role="presentation">
                  <span className="rb-groupnum" aria-hidden {...hover}>
                    {String(i + 1).padStart(2, "0")}
                  </span>
                  {g.tabs.map((t) => (
                    <button
                      key={t}
                      role="tab"
                      aria-selected={tab === t}
                      className={`rb-tab${tab === t ? " active" : ""}${t === "Modify" && ctxLabel ? " contextual" : ""}`}
                      onClick={() => {
                        setTab(t);
                        if (t === "Rufplan" && !useAppStore.getState().cloud) void refreshCloud();
                      }}
                    >
                      {t === "Modify" && ctxLabel ? ctxLabel : t}
                    </button>
                  ))}
                </div>
              </div>
            </div>
          );
        })}
      </div>
      <div className="rb-body">
        {tab === "Project Info" && <ProjectInfoRibbon />}
        {tab === "Specifications" && <SpecsRibbon />}
        {tab === "Standards" && <StandardsRibbon />}
        {/* Modify, Move and Delete live on the Modify tab only (Esc still returns to Modify). */}
        {tab === "Modify" && (
          <Group title="Select">
            <ToolButton tool="select" label="Modify" icon={Icons.select} keys="MD / Esc" />
          </Group>
        )}
        {tab === "Architecture" && (
          <>
            <Group title="Generate">
              <button
                className="rb-btn rb-claude"
                onClick={() => setUi({ viewDialog: "generate" })}
                disabled={!app}
                title="Generate with Claude: describe a building (type, stories, style, references) and it's built as a model"
              >
                {Icons.sparkle}
                <span>Generate</span>
              </button>
              <button
                className="rb-btn rb-claude"
                onClick={() => setUi({ viewDialog: "plans" })}
                disabled={!app}
                title="Plans to 3D: give floor plans (images or a PDF) and Claude traces them into a model"
              >
                {Icons.plans}
                <span>Plans to 3D</span>
              </button>
            </Group>
            <Group title="Build">
              <ToolButton tool="wall" label="Wall" icon={Icons.wall} keys="WA" />
              {sketchButton("Floor", "Floor", Icons.floorAuto, "SB")}
              <ToolButton
                tool="ceilingAuto"
                label="Ceiling"
                icon={Icons.ceiling}
                keys="CL — auto room"
              />
              {sketchButton("Ceiling", "Sketch Ceiling", Icons.floor, "CS")}
              <button
                className="rb-btn"
                onClick={modelInPlace}
                disabled={!app}
                title="Model In-Place: model a one-off element (a counter, a curved wall, a canopy…) from extrusions, blends and sweeps, as a wall, door, furniture or any category"
              >
                {Icons.inPlace}
                <span>Model In-Place</span>
              </button>
            </Group>
            <Group title="Model">
              <ToolButton
                tool="modelLine"
                label="Model Line"
                icon={Icons.modelLine}
                keys="LI — on the level, seen in every view"
              />
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
        {tab === "Openings" && (
          <>
            <Group title="Door">
              <ToolButton tool="door" label="Door" icon={Icons.door} keys="DR" />
              <button
                className="rb-btn"
                onClick={() => void openPicker("Door", "library")}
                disabled={!app}
                title="Door Library: single and double swing, French, entry with sidelites, sliding glass, pocket, barn, bifold, folding glass wall, storefront and garage doors"
              >
                {Icons.door}
                <span>Load Doors</span>
              </button>
            </Group>
            <Group title="Window">
              <ToolButton tool="window" label="Window" icon={Icons.window} keys="WN" />
              <button
                className="rb-btn"
                onClick={() => void openPicker("Window", "library")}
                disabled={!app}
                title="Window Library: double-hung, casement, slider, bay, storefront and other US window types at standard or custom sizes"
              >
                {Icons.window}
                <span>Load Windows</span>
              </button>
            </Group>
            <Group title="Opening">
              <ToolButton
                tool="wallOpening"
                label="Wall Opening"
                icon={Icons.wallOpening}
                keys="any shape, in an elevation, section or 3D"
              />
            </Group>
          </>
        )}
        {tab === "Lighting" && (
          <>
            <Group title="Lighting Fixture">
              <ToolButton
                tool="light"
                label="Lighting Fixture"
                icon={Icons.light}
                keys="ceiling, wall, floor or site"
              />
              <button
                className="rb-btn"
                onClick={() => void openPicker("Light", "library")}
                disabled={!app}
                title="Lighting Library: downlights, troffers, pendants, chandeliers, linear, sconces, lamps, high bays, exit signs and site lights for every building type"
              >
                {Icons.light}
                <span>Load Fixtures</span>
              </button>
            </Group>
            <Group title="Sun">
              <button
                className="rb-btn"
                onClick={() => setUi({ viewDialog: "sunSettings" })}
                disabled={!app}
                title={`Sun Settings: ${sunLabel}`}
              >
                {Icons.sun}
                <span>Sun Settings</span>
              </button>
            </Group>
            <Group title="Artificial Lights">
              <button
                className="rb-btn"
                onClick={() => setUi({ viewDialog: "artificialLights" })}
                disabled={!app}
                title="Artificial Lights: switch and dim the fixtures, by type"
              >
                {Icons.bulb}
                <span>Artificial Lights</span>
              </button>
            </Group>
            <Group title="Render">
              <button
                className="rb-btn"
                onClick={() => void runAction("render")}
                disabled={!activeIs3d}
                title="Render (RR): choose Exterior or Interior, sun and artificial lights"
              >
                {Icons.render}
                <span>Render</span>
              </button>
            </Group>
          </>
        )}
        {tab === "Landscape" && (
          <>
            <Group title="Asset Library">
              <button
                className="rb-btn"
                onClick={() => {
                  useAppStore.getState().setAssetFilter(null);
                  void openPicker("Plant", "library");
                }}
                disabled={!app}
                title="Asset Library: trees, bushes, hedges, grasses, flowers and succulents, with season variants (Enscape's)"
              >
                {Icons.assets}
                <span>Asset Library</span>
              </button>
              <ToolButton tool="plant" label="Place Plant" icon={Icons.tree} keys="PL" />
              <ToolButton
                tool="grassBrush"
                label="Grass Brush"
                icon={Icons.grass}
                keys="GB, in 3D: drag to paint grass (D5's)"
              />
            </Group>
            <Group title="Trees & Plants">
              {(
                [
                  ["Trees", "Trees", null, Icons.tree, "Deciduous, flowering and evergreen trees"],
                  [
                    "Conifers",
                    "Trees",
                    "Conifer",
                    Icons.conifer,
                    "Pines, spruces, firs, cedars and cypresses",
                  ],
                  ["Palms", "Trees", "Palm", Icons.palm, "Palms and cycads"],
                  ["Shrubs", "Bushes", null, Icons.shrub, "Shrubs, bushes and hedges"],
                  [
                    "Grasses",
                    "Grass & Flowers",
                    null,
                    Icons.grass,
                    "Ornamental grasses and flowers",
                  ],
                ] as const
              ).map(([label, category, group, icon, title]) => (
                <button
                  key={label}
                  className="rb-btn"
                  onClick={() => {
                    useAppStore.getState().setAssetFilter({ category, group });
                    void openPicker("Plant", "library");
                  }}
                  disabled={!app}
                  title={`${title}: open the Asset Library`}
                >
                  {icon}
                  <span>{label}</span>
                </button>
              ))}
            </Group>
            <Group title="Season">
              <label
                className="rb-field"
                title="Every deciduous and flowering tree's season (Enscape's season variants)"
              >
                {Icons.season}
                <select
                  aria-label="Season"
                  defaultValue=""
                  disabled={!app?.plantingTypes.length}
                  onChange={(e) => {
                    const v = e.target.value as "Spring" | "Summer" | "Autumn" | "Winter" | "";
                    if (v) void setSeason(v);
                    e.target.value = "";
                  }}
                >
                  <option value="">Set season…</option>
                  <option value="Spring">Spring (in bloom)</option>
                  <option value="Summer">Summer</option>
                  <option value="Autumn">Autumn colour</option>
                  <option value="Winter">Winter (bare)</option>
                </select>
              </label>
            </Group>
            <Group title="Ground">
              <button
                className="rb-btn"
                onClick={() => setUi({ viewDialog: "ground" })}
                disabled={!app}
                title="Base Ground: lawn, meadow, pine straw, mulch, gravel, asphalt, pavers… for the topography or the ground around the model; lawns grow 3D grass in Realistic"
              >
                {Icons.ground}
                <span>Base Ground</span>
              </button>
              <button
                className="rb-btn"
                onClick={() => void startSketch("GroundRegion")}
                disabled={!app}
                title="Ground Region: sketch a drive, a patio, a lawn or a bed on the ground (Revit's subregion); pick its material in Properties"
              >
                {Icons.region}
                <span>Ground Region</span>
              </button>
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
        {tab === "Materials" && (
          <>
            <Group title="Library">
              <button
                className="rb-btn"
                onClick={() => setUi({ viewDialog: "materials" })}
                disabled={!app}
                title="Material Browser: V-Ray-style materials for residential, hospitality and multifamily work, from typical to high-end"
              >
                {Icons.material}
                <span>Material Browser</span>
              </button>
            </Group>
            <Group title="Project">
              <button
                className="rb-btn"
                onClick={() => void newMaterial(null)}
                disabled={!app}
                title="Add a blank material; edit it in Properties"
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
            <Group title="Apply">
              <button
                className="rb-btn"
                onClick={() => void runAction("paint")}
                disabled={!app}
                title="Paint (PT): pick a material, then click walls, floors, ceilings, roofs, columns or beams to paint them; Shift-click paints the whole type"
              >
                {Icons.paint}
                <span>Paint</span>
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
        {tab === "Structure" && <StructureAnalyze />}
        {tab === "MEPT" && <MeptRibbon />}
        {tab === "Modify" && (
          <>
            {ctxLabel && <ContextPropertiesGroup />}
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
            </Group>
          </>
        )}
        {tab === "Modify" && (
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
        )}
        {tab === "Modify" && ctxLabel && <ContextPanels cats={selectedCats} />}
        {tab === "Annotate" && (
          <Group title="Annotate">
            <ToolButton tool="dimension" label="Aligned" icon={Icons.dimension} keys="DI" />
            <ToolButton tool="dimensionLinear" label="Linear" icon={Icons.dimLinear} keys="" />
            <ToolButton tool="dimensionAngular" label="Angular" icon={Icons.dimAngular} keys="" />
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
        {tab === "Details" && (
          <>
            <Group title="Create">
              <button
                className="rb-btn"
                onClick={() => setUi({ viewDialog: "draftingView" })}
                disabled={!app}
                title="Drafting View: a new 2D view for details, drawn with detail lines, filled regions and text at a scale"
              >
                {Icons.draftingView}
                <span>Drafting View</span>
              </button>
              <button
                className="rb-btn"
                onClick={() => setUi({ viewDialog: "details" })}
                disabled={!app}
                title="Detail Library: typical construction details, each drawn at its usual scale; insert one as a drafting view to edit"
              >
                {Icons.detailLibrary}
                <span>Detail Library</span>
              </button>
              <button
                className="rb-btn"
                onClick={() => setUi({ viewDialog: "saveDetail" })}
                disabled={!activeIsDrafting}
                title="Save to Library: keep this drafting view as one of your details, to insert in any project"
              >
                {Icons.saveDetail}
                <span>Save to Library</span>
              </button>
              <ToolButton
                tool="callout"
                label="Callout"
                icon={Icons.callout}
                keys="a detail view of the model"
              />
            </Group>
            <Group title="Detail">
              <ToolButton
                tool="detailLine"
                label="Detail Line"
                icon={Icons.detailLine}
                keys="DL — this view only"
              />
              <ToolButton
                tool="component"
                label="Detail Component"
                icon={Icons.detailComponent}
                keys="CM — break lines, lumber, sheathing, steel…"
              />
              <button
                className="rb-btn"
                onClick={() => startComponent("brick-mod")}
                disabled={!app}
                title="Repeating Detail: brick or CMU coursing along a line"
              >
                {Icons.repeatingDetail}
                <span>Repeating Detail</span>
              </button>
              <button
                className="rb-btn"
                onClick={() => startComponent("batt-55")}
                disabled={!app}
                title="Insulation: batt insulation along a line, at a width"
              >
                {Icons.insulation}
                <span>Insulation</span>
              </button>
              <button
                className="rb-btn"
                onClick={() => void startFilledRegion()}
                disabled={!app}
                title="Filled Region: sketch an area and fill it with a pattern (concrete, earth, insulation…)"
              >
                {Icons.filledRegion}
                <span>Filled Region</span>
              </button>
              <ToolButton tool="text" label="Text" icon={Icons.text} keys="TX" />
              <ToolButton
                tool="dimension"
                label="Dimension"
                icon={Icons.dimension}
                keys="DI — to detail lines and components"
              />
            </Group>
          </>
        )}
        {tab === "Annotate" && <KeynoteGroup />}
        {tab === "Annotate" && (
          <Group title="Detail">
            <ToolButton
              tool="detailLine"
              label="Detail Line"
              icon={Icons.detailLine}
              keys="DL — this view only"
            />
          </Group>
        )}
        {tab === "Annotate" && (
          <Group title="Symbol">
            <ToolButton
              tool="spotElevation"
              label="Spot Elevation"
              icon={Icons.spotElevation}
              keys="click the point, then the text"
            />
            <ToolButton
              tool="spotSlope"
              label="Spot Slope"
              icon={Icons.spotSlope}
              keys="roofs, ramps, sidewalks, ground"
            />
            <ToolButton
              tool="northArrow"
              label="North Arrow"
              icon={Icons.northArrow}
              keys="plans and sheets"
            />
            <ToolButton
              tool="graphicScale"
              label="Graphic Scale"
              icon={Icons.graphicScale}
              keys="follows the view scale"
            />
            <ToolButton tool="keyPlan" label="Key Plan" icon={Icons.keyPlan} keys="on sheets" />
          </Group>
        )}
        {tab === "Views" && (
          <>
            <Group title="Create">
              <NewViewMenu />
            </Group>
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
                VISUAL_STYLES.map(({ id, label }) => (
                  <button
                    key={id}
                    className={`rb-btn rb-small${visualStyle === id ? " active" : ""}`}
                    onClick={() => void runAction(id)}
                    title={STYLE_KEYS[id] ? `${label} (${STYLE_KEYS[id]})` : label}
                  >
                    <StyleIcon style={id} size={16} />
                    <span>{label}</span>
                  </button>
                ))}
            </Group>
          </>
        )}
        {tab === "Collaborate" && <CollaborateRibbon />}
        {tab === "Sheets" && (
          <>
            <Group title="Sheet Composition">
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
              <ToolButton tool="keyPlan" label="Key Plan" icon={Icons.keyPlan} keys="on sheets" />
            </Group>
            <Group title="Sets">
              <button
                className="rb-btn"
                onClick={() => setUi({ viewDialog: "sheetSets" })}
                disabled={!app}
                title="Sheet Sets: each phase's deliverables (SD, DD, Permit, Bid…) and their sheets for the building type, created and exported"
              >
                {Icons.issue}
                <span>Sheet Sets</span>
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
            <Group title="Export">
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
            </Group>
          </>
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
