import { useEffect, useState, type ReactNode } from "react";
import type { Category } from "../bindings/Category";
import type { ElementId } from "../bindings/ElementId";
import type { LineStyle } from "../bindings/LineStyle";
import { apply } from "../fileActions";
import { ipc } from "../ipc";
import { activeViewInfo, LINE_STYLES, useAppStore } from "../store";
import { openPicker, runAction } from "../actions";
import { editBoundary } from "../sketch";
import { Icons } from "./Icons";

// Revit's contextual Modify tab (ADR-055): selecting elements turns the Modify tab into
// "Modify | Walls" (green), with the panels for what's selected after the usual ones.

/** The contextual tab's name for each category, as Revit words it. Categories left out
 * (types, materials, sheets, project settings) don't get a contextual tab. */
const PLURAL: Partial<Record<Category, string>> = {
  Wall: "Walls",
  Floor: "Floors",
  Ceiling: "Ceilings",
  Door: "Doors",
  Window: "Windows",
  Room: "Rooms",
  Roof: "Roofs",
  Stair: "Stairs",
  Column: "Structural Columns",
  Beam: "Structural Framing",
  Railing: "Railings",
  Level: "Levels",
  Grid: "Grids",
  Dimension: "Dimensions",
  TextNote: "Text Notes",
  Tag: "Tags",
  View: "Views",
  Viewport: "Viewports",
  RoomSeparator: "Room Separation Lines",
  ElevationMarker: "Elevations",
  SpotElevation: "Spot Elevations",
  SpotSlope: "Spot Slopes",
  NorthArrow: "North Arrows",
  GraphicScale: "Graphic Scales",
  KeyPlan: "Key Plans",
  DetailLine: "Lines",
  ModelLine: "Lines",
  Site: "Toposolid",
};

/** Model categories, which Paint and Hide Category apply to. */
const MODEL: Category[] = [
  "Wall",
  "Floor",
  "Ceiling",
  "Door",
  "Window",
  "Roof",
  "Stair",
  "Column",
  "Beam",
  "Railing",
  "Site",
];
const TAGGABLE: Category[] = ["Door", "Window", "Room", "Column", "Beam"];

/** "Modify | Walls", "Modify | Lines", or "Modify | Multi-Select"; null when the selection
 * has nothing a contextual tab is for. */
export function contextLabel(cats: Category[]): string | null {
  const names = [...new Set(cats.map((c) => PLURAL[c]).filter(Boolean))];
  if (names.length === 0) return null;
  return `Modify | ${names.length === 1 ? names[0] : "Multi-Select"}`;
}

/** The selection's categories, fetched for the ids they belong to so a stale answer is
 * never used for a new selection. */
export function useSelectionCategories(): Category[] {
  const selection = useAppStore((s) => s.selection);
  const app = useAppStore((s) => s.app);
  const key = selection.join(",");
  const [got, setGot] = useState<{ key: string; cats: Category[] }>({ key: "", cats: [] });
  useEffect(() => {
    if (!key || !app) return;
    let live = true;
    ipc
      .selectionCategories(key.split(","))
      .then((cats) => live && setGot({ key, cats }))
      .catch(() => live && setGot({ key, cats: [] }));
    return () => {
      live = false;
    };
  }, [key, app]);
  return key && got.key === key ? got.cats : [];
}

let returnTab: string | null = null;

/** Revit's tab switching: a selection opens the contextual tab, and clearing it goes back
 * to the tab the user was on. */
export function useContextualSwitch(label: string | null) {
  const key = useAppStore((s) => (label ? s.selection.join(",") : ""));
  useEffect(() => {
    const s = useAppStore.getState();
    if (key) {
      if (s.ribbonTab !== "Modify") {
        returnTab = s.ribbonTab;
        s.setRibbonTab("Modify");
      }
    } else if (returnTab) {
      if (s.ribbonTab === "Modify") s.setRibbonTab(returnTab);
      returnTab = null;
    }
  }, [key]);
}

function Group({ title, children }: { title: string; children: ReactNode }) {
  return (
    <div className="rb-group">
      <div className="rb-items">{children}</div>
      <div className="rb-title">{title}</div>
    </div>
  );
}

function Btn({
  label,
  icon,
  title,
  onClick,
  disabled,
  small,
}: {
  label: string;
  icon: ReactNode;
  title: string;
  onClick: () => void;
  disabled?: boolean;
  small?: boolean;
}) {
  return (
    <button
      className={`rb-btn${small ? " rb-small" : ""}`}
      onClick={onClick}
      disabled={disabled}
      title={title}
    >
      {icon}
      <span>{label}</span>
    </button>
  );
}

/** The ids of the selection in these categories. */
async function ofCategory(ids: ElementId[], cats: Category[]): Promise<ElementId[]> {
  const got = await Promise.all(ids.map((id) => ipc.selectionCategories([id]).catch(() => [])));
  return ids.filter((_, i) => got[i]!.some((c) => cats.includes(c)));
}

async function tagSelection() {
  const s = useAppStore.getState();
  const view = activeViewInfo(s);
  if (!view) return;
  const targets = await ofCategory(s.selection, TAGGABLE);
  await apply(() => ipc.tagElements(view.id, targets));
}

async function setLineStyle(style: LineStyle) {
  const s = useAppStore.getState();
  const lines = await ofCategory(s.selection, ["DetailLine", "ModelLine"]);
  await apply(async () => {
    let last = null;
    for (const id of lines) last = await ipc.setProperty(id, "style", style);
    return last;
  });
}

/** Type Properties: selects the element's type so Properties edits it. */
async function editType(id: ElementId) {
  const sheet = await ipc.properties(id).catch(() => null);
  if (sheet?.typeId) useAppStore.getState().select([sheet.typeId]);
  else useAppStore.getState().setError("This element has no type to edit.");
}

/** Opens a selected section, elevation or callout; on a sheet, a selected viewport's view. */
async function goToView(id: ElementId, activate: boolean) {
  const s = useAppStore.getState();
  const vp = await ipc.viewportInfo(id).catch(() => null);
  const target = vp ? vp.view : id;
  const view = s.app?.views.find((v) => v.id === target);
  if (!view) {
    s.setError("That view isn't in the project.");
    return;
  }
  const sheet = activeViewInfo(s);
  if (activate && vp && sheet?.viewType === "Sheet")
    s.activateViewport({ sheet: sheet.id, viewport: id, view: vp.view, center: vp.center });
  else s.openView(view.id);
}

/** The panels before the usual Modify tools: Properties. */
export function ContextPropertiesGroup() {
  const selection = useAppStore((s) => s.selection);
  return (
    <Group title="Properties">
      <Btn
        label="Properties"
        icon={Icons.params}
        title="Show or hide the Properties palette (PP)"
        onClick={() => void runAction("properties")}
      />
      <Btn
        label="Type Properties"
        icon={Icons.params}
        title="Edit the selected element's type"
        onClick={() => selection[0] && void editType(selection[0])}
        disabled={selection.length !== 1}
      />
    </Group>
  );
}

/** The panels after the usual Modify tools: View, Create, then one for each selected
 * category that has its own tools. */
export function ContextPanels({ cats }: { cats: Category[] }) {
  const selection = useAppStore((s) => s.selection);
  const viewType = useAppStore((s) => activeViewInfo(s)?.viewType);
  const inPlan = viewType === "Plan";
  const one = selection.length === 1;
  const has = (...c: Category[]) => cats.some((x) => c.includes(x));
  const only = (...c: Category[]) => cats.length > 0 && cats.every((x) => c.includes(x));
  const [lineStyle, setStyle] = useState<LineStyle | "">("");
  return (
    <>
      <Group title="View">
        <Btn
          small
          label="Hide in View"
          icon={Icons.eye}
          title="Hide in View > Elements (EH)"
          onClick={() => void runAction("hideInView")}
        />
        <Btn
          small
          label="Hide Element"
          icon={Icons.eye}
          title="Temporary Hide Element (HH)"
          onClick={() => void runAction("hideElement")}
        />
        <Btn
          small
          label="Isolate Element"
          icon={Icons.eye}
          title="Temporary Isolate Element (HI)"
          onClick={() => void runAction("isolateElement")}
        />
        <Btn
          small
          label="Isolate Category"
          icon={Icons.eye}
          title="Temporary Isolate Category (IC)"
          onClick={() => void runAction("isolateCategory")}
        />
      </Group>
      <Group title="Create">
        <Btn
          label="Create Similar"
          icon={Icons.copy}
          title="Create Similar (CS): place another of the selected element's type"
          onClick={() => void runAction("createSimilar")}
          disabled={!one}
        />
        <Btn
          label="Select All"
          icon={Icons.select}
          title="Select All Instances (SA) of the selected element's type"
          onClick={() => void runAction("selectAll")}
          disabled={!one}
        />
        <Btn
          label="Edit with Claude"
          icon={Icons.sparkle}
          title="Describe a change to the selection (⌘K)"
          onClick={() => useAppStore.getState().setEditModel({ open: true })}
        />
      </Group>
      {has(...MODEL) && (
        <Group title="Geometry">
          <Btn
            label="Paint"
            icon={Icons.paint}
            title="Paint (PT): a material on the selected elements' faces"
            onClick={() => void runAction("paint")}
          />
        </Group>
      )}
      {has("Wall") && (
        <Group title="Modify Wall">
          <Btn
            label="Attach Top"
            icon={Icons.attach}
            title="Attach the selected walls' tops to the roof above"
            onClick={() => void apply(() => ipc.attachWallTops(selection, true))}
          />
          <Btn
            label="Detach Top"
            icon={Icons.del}
            title="Detach the selected walls' tops from the roof"
            onClick={() => void apply(() => ipc.attachWallTops(selection, false))}
          />
          <Btn
            label="Flip"
            icon={Icons.flip}
            title="Flip the walls' orientation (Space)"
            onClick={() => void apply(() => ipc.flipSelection(selection))}
          />
        </Group>
      )}
      {has("Floor", "Ceiling") && (
        <Group title="Mode">
          <Btn
            label="Edit Boundary"
            icon={has("Floor") ? Icons.floor : Icons.ceiling}
            title="Edit the boundary sketch (or double-click the element)"
            onClick={() => selection[0] && void editBoundary(selection[0])}
            disabled={!one}
          />
        </Group>
      )}
      {(has("Door") || has("Window")) && (
        <Group title={only("Door") ? "Door" : only("Window") ? "Window" : "Openings"}>
          <Btn
            label="Flip"
            icon={Icons.flip}
            title="Flip the facing (Space)"
            onClick={() => void apply(() => ipc.flipSelection(selection))}
          />
          {only("Door") || only("Window") ? (
            <Btn
              label="Browse Types"
              icon={only("Door") ? Icons.door : Icons.window}
              title="Change the selected elements' type in the type picker"
              onClick={() => void openPicker(only("Door") ? "Door" : "Window")}
            />
          ) : null}
        </Group>
      )}
      {has(...TAGGABLE) && (
        <Group title="Tag">
          <Btn
            label={only("Room") ? "Tag Room" : "Tag"}
            icon={Icons.tag}
            title={
              inPlan
                ? "Tag the selected elements that aren't tagged in this plan"
                : "Tags go in floor plans"
            }
            onClick={() => void tagSelection()}
            disabled={!inPlan}
          />
        </Group>
      )}
      {has("DetailLine", "ModelLine") && (
        <Group title="Line Style">
          <label className="rb-place">
            <select
              aria-label="Selected Line Style"
              value={lineStyle}
              onChange={(e) => {
                const v = e.target.value as LineStyle;
                setStyle(v);
                void setLineStyle(v);
              }}
            >
              <option value="" disabled>
                Line Style…
              </option>
              {LINE_STYLES.map(([id, label]) => (
                <option key={id} value={id}>
                  {label}
                </option>
              ))}
            </select>
          </label>
        </Group>
      )}
      {one && has("View", "Viewport") && (
        <Group title="Views">
          <Btn
            label="Go to View"
            icon={Icons.view3d}
            title="Open the selected view"
            onClick={() => selection[0] && void goToView(selection[0], false)}
          />
          {has("Viewport") && viewType === "Sheet" && (
            <Btn
              label="Activate View"
              icon={Icons.sheet}
              title="Work in the view on the sheet (or double-click it)"
              onClick={() => selection[0] && void goToView(selection[0], true)}
            />
          )}
        </Group>
      )}
    </>
  );
}
