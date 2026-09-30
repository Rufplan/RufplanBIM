import { useState, type ReactNode } from "react";
import type { ViewType } from "../bindings/ViewType";
import type { ViewInfo } from "../bindings/ViewInfo";
import { ContextMenu, type MenuItem } from "./ContextMenu";
import { RenameDialog } from "./RenameDialog";
import { deleteView, duplicateSheet, duplicateView, viewProperties } from "../views";
import { useAppStore } from "../store";
import { placeGroup } from "./Groups";

function Section({
  title,
  children,
  start = true,
}: {
  title: string;
  children: ReactNode;
  start?: boolean;
}) {
  const [open, setOpen] = useState(start);
  return (
    <div className="pb-section">
      <button className="pb-head" onClick={() => setOpen(!open)} aria-expanded={open}>
        <span className="pb-caret">{open ? "▾" : "▸"}</span>
        {title}
      </button>
      {open && <div className="pb-items">{children}</div>}
    </div>
  );
}

const VIEW_GROUPS: [ViewType, string][] = [
  ["Plan", "Floor Plans"],
  ["CeilingPlan", "Ceiling Plans"],
  ["Elevation", "Elevations"],
  ["Section", "Sections"],
  ["ThreeD", "3D Views"],
  ["Drafting", "Drafting Views"],
  ["Schedule", "Schedules"],
];

export function ProjectBrowser() {
  const app = useAppStore((s) => s.app);
  const activeView = useAppStore((s) => s.activeView);
  const selection = useAppStore((s) => s.selection);
  const openView = useAppStore((s) => s.openView);
  const select = useAppStore((s) => s.select);
  // "all" or a stage id: show only the sheets in that stage's deliverable set.
  const [stageFilter, setStageFilter] = useState("all");
  // Revit's right-click menu on views and sheets (ADR-074), and its Rename dialog.
  const [menu, setMenu] = useState<{ x: number; y: number; view: ViewInfo } | null>(null);
  const [renaming, setRenaming] = useState<ViewInfo | null>(null);
  const openViews = useAppStore((s) => s.openViews);
  if (!app) return null;
  const menuItems = (v: ViewInfo): MenuItem[] => {
    const isSheet = v.viewType === "Sheet";
    const open = openViews.includes(v.id);
    const common: MenuItem[] = [
      { label: "Open", onClick: () => openView(v.id) },
      {
        label: "Close",
        disabled: !open,
        onClick: () => useAppStore.getState().closeView(v.id),
      },
    ];
    const dup: MenuItem = isSheet
      ? {
          label: "Duplicate Sheet",
          separator: true,
          items: [
            { label: "Duplicate Empty Sheet", onClick: () => void duplicateSheet(v.id, "Empty") },
            {
              label: "Duplicate with Detailing",
              onClick: () => void duplicateSheet(v.id, "WithDetailing"),
            },
            {
              label: "Duplicate with Views",
              onClick: () => void duplicateSheet(v.id, "WithViews"),
            },
          ],
        }
      : {
          label: "Duplicate View",
          separator: true,
          items: [
            { label: "Duplicate", onClick: () => void duplicateView(v.id, false) },
            {
              label: "Duplicate with Detailing",
              disabled: v.viewType === "Schedule" || v.viewType === "ThreeD",
              onClick: () => void duplicateView(v.id, true),
            },
          ],
        };
    const extra: MenuItem[] =
      v.viewType === "Drafting"
        ? [
            {
              label: "Save to Library…",
              onClick: () => {
                openView(v.id);
                useAppStore.getState().setUi({ viewDialog: "saveDetail" });
              },
            },
          ]
        : [];
    return [
      ...common,
      dup,
      ...extra,
      { label: "Rename…", separator: true, onClick: () => setRenaming(v) },
      { label: isSheet ? "Delete Sheet" : "Delete", onClick: () => void deleteView(v.id) },
      { label: "Properties", separator: true, onClick: () => viewProperties(v.id) },
    ];
  };
  const onMenu = (e: React.MouseEvent, id: string) => {
    const v = app.views.find((x) => x.id === id);
    if (!v) return;
    e.preventDefault();
    setMenu({ x: e.clientX, y: e.clientY, view: v });
  };
  const sheets = app.views.filter(
    (v) => v.viewType === "Sheet" && (stageFilter === "all" || v.stages.includes(stageFilter)),
  );

  const item = (id: string, label: string, onClick: () => void, active: boolean, sub?: string) => (
    <button
      key={id}
      className={`pb-item${active ? " active" : ""}`}
      onClick={onClick}
      onContextMenu={(e) => onMenu(e, id)}
      title={label}
    >
      <span className="pb-label">{label}</span>
      {sub && <span className="pb-sub">{sub}</span>}
    </button>
  );

  return (
    <aside className="panel browser" aria-label="Project browser">
      {menu && (
        <ContextMenu
          x={menu.x}
          y={menu.y}
          label={`${menu.view.name} menu`}
          items={menuItems(menu.view)}
          onClose={() => setMenu(null)}
        />
      )}
      {renaming && <RenameDialog view={renaming} onClose={() => setRenaming(null)} />}
      <div className="panel-title">Project Browser</div>
      <div className="panel-body">
        <Section title="Views">
          {VIEW_GROUPS.map(([type, label]) => {
            const views = app.views.filter((v) => v.viewType === type && v.calloutOf === null);
            if (views.length === 0) return null;
            return (
              <Section key={type} title={label}>
                {views.map((v) => item(v.id, v.name, () => openView(v.id), v.id === activeView))}
              </Section>
            );
          })}
          {app.views.some((v) => v.calloutOf !== null) && (
            <Section title="Callouts">
              {app.views
                .filter((v) => v.calloutOf !== null)
                .map((v) => item(v.id, v.name, () => openView(v.id), v.id === activeView))}
            </Section>
          )}
        </Section>
        <Section title="Sheets">
          <select
            className="pb-filter"
            aria-label="Filter sheets by design stage"
            value={stageFilter}
            onChange={(e) => setStageFilter(e.target.value)}
          >
            <option value="all">All sheets</option>
            {app.stages.map((s) => (
              <option key={s.id} value={s.id}>
                {s.abbreviation} set
              </option>
            ))}
          </select>
          {sheets.map((v) => item(v.id, v.name, () => openView(v.id), v.id === activeView))}
          {sheets.length === 0 && (
            <div className="pb-empty">
              {stageFilter === "all"
                ? "No sheets yet — use New Sheet."
                : "No sheets in this set. Add sheets via their Stage Sets properties."}
            </div>
          )}
        </Section>
        <Section title="Families">
          <Section title="Walls" start={false}>
            {app.wallTypes.map((t) =>
              item(t.id, t.name, () => select([t.id]), selection.includes(t.id)),
            )}
          </Section>
          <Section title="Floors" start={false}>
            {app.floorTypes.map((t) =>
              item(t.id, t.name, () => select([t.id]), selection.includes(t.id)),
            )}
          </Section>
          <Section title="Doors" start={false}>
            {app.doorTypes.map((t) =>
              item(t.id, t.name, () => select([t.id]), selection.includes(t.id)),
            )}
          </Section>
          <Section title="Windows" start={false}>
            {app.windowTypes.map((t) =>
              item(t.id, t.name, () => select([t.id]), selection.includes(t.id)),
            )}
          </Section>
          <Section title="Ceilings" start={false}>
            {app.ceilingTypes.map((t) =>
              item(t.id, t.name, () => select([t.id]), selection.includes(t.id)),
            )}
          </Section>
          {(
            [
              ["Roofs", app.roofTypes],
              ["Structural Columns", app.columnTypes],
              ["Lighting Fixtures", app.lightingFixtureTypes],
              ["Structural Framing", app.beamTypes],
              ["Railings", app.railingTypes],
              ["Elevation Marks", app.elevationMarkerTypes],
            ] as const
          ).map(([title, types]) => (
            <Section key={title} title={title} start={false}>
              {types.map((t) => item(t.id, t.name, () => select([t.id]), selection.includes(t.id)))}
            </Section>
          ))}
        </Section>
        <Section title="Groups" start={false}>
          {(["Model", "Detail"] as const).map((kind) => (
            <Section key={kind} title={kind} start={false}>
              {app.groupTypes
                .filter((t) => t.kind === kind)
                .map((t) => (
                  <div key={t.id} className="pb-row-wrap">
                    {item(
                      t.id,
                      `${t.name} (${t.instances})`,
                      () => select(app.groups.filter((g) => g.typeId === t.id).map((g) => g.id)),
                      app.groups.some((g) => g.typeId === t.id && selection.includes(g.id)),
                    )}
                    <button
                      className="pb-mini"
                      title={t.instances ? "Place an instance" : "No instance to copy"}
                      aria-label={`Place ${t.name}`}
                      disabled={t.instances === 0}
                      onClick={() => placeGroup(kind, t.id)}
                    >
                      +
                    </button>
                  </div>
                ))}
              {!app.groupTypes.some((t) => t.kind === kind) && (
                <div className="pb-empty">None yet — select elements and Create Group (GP).</div>
              )}
            </Section>
          ))}
        </Section>
        <Section title="Materials" start={false}>
          {app.materials.map((m) =>
            item(m.id, m.name, () => select([m.id]), selection.includes(m.id)),
          )}
        </Section>
        <Section title="Project">
          {app.projectInfo &&
            item(
              app.projectInfo,
              "Project Information",
              () => select([app.projectInfo!]),
              selection.includes(app.projectInfo),
            )}
          <Section title="Issuances">
            {app.issuances.map((i) =>
              item(i.id, i.name, () => select([i.id]), selection.includes(i.id)),
            )}
            {app.issuances.length === 0 && <div className="pb-empty">Nothing issued yet.</div>}
          </Section>
          <Section title="Design Stages">
            {app.stages.map((s) =>
              item(
                s.id,
                s.name,
                () => select([s.id]),
                selection.includes(s.id),
                s.id === app.currentStage ? "Current" : s.abbreviation,
              ),
            )}
          </Section>
          <Section title="Levels" start={false}>
            {app.levels.map((l) =>
              item(l.id, l.name, () => select([l.id]), selection.includes(l.id)),
            )}
          </Section>
        </Section>
      </div>
    </aside>
  );
}
