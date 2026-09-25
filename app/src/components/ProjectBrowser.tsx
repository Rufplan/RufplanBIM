import { useState, type ReactNode } from "react";
import type { ViewType } from "../bindings/ViewType";
import { useAppStore } from "../store";

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
  if (!app) return null;
  const sheets = app.views.filter(
    (v) => v.viewType === "Sheet" && (stageFilter === "all" || v.stages.includes(stageFilter)),
  );

  const item = (id: string, label: string, onClick: () => void, active: boolean, sub?: string) => (
    <button
      key={id}
      className={`pb-item${active ? " active" : ""}`}
      onClick={onClick}
      title={label}
    >
      <span className="pb-label">{label}</span>
      {sub && <span className="pb-sub">{sub}</span>}
    </button>
  );

  return (
    <aside className="panel browser" aria-label="Project browser">
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
              ["Structural Framing", app.beamTypes],
              ["Railings", app.railingTypes],
            ] as const
          ).map(([title, types]) => (
            <Section key={title} title={title} start={false}>
              {types.map((t) => item(t.id, t.name, () => select([t.id]), selection.includes(t.id)))}
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
