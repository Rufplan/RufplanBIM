import { useState } from "react";
import { activeViewInfo, useAppStore } from "../store";
import { duplicateView, new3dView, newPlanView } from "../views";
import { ContextMenu, type MenuItem } from "./ContextMenu";
import { Icons } from "./Icons";

// View > New View (ADR-074): Revit's Create panel in one menu. Plans and ceiling plans of
// any level, 3D, sections, elevations, callouts, drafting views, and Duplicate View.

export function NewViewMenu() {
  const [at, setAt] = useState<{ x: number; y: number } | null>(null);
  const app = useAppStore((s) => s.app);
  const active = useAppStore(activeViewInfo);
  const setTool = useAppStore((s) => s.setTool);
  const setUi = useAppStore((s) => s.setUi);
  const levels = app?.levels ?? [];
  const drawable = active && active.viewType !== "Sheet";
  const items: MenuItem[] = [
    {
      label: "Floor Plan",
      items: levels.map((l) => ({ label: l.name, onClick: () => void newPlanView(l.id, false) })),
    },
    {
      label: "Reflected Ceiling Plan",
      items: levels.map((l) => ({ label: l.name, onClick: () => void newPlanView(l.id, true) })),
    },
    { label: "3D View", onClick: () => void new3dView() },
    { label: "Section", separator: true, onClick: () => setTool("section") },
    { label: "Elevation", onClick: () => setTool("elevation") },
    { label: "Callout", onClick: () => setTool("callout") },
    { label: "Drafting View…", onClick: () => setUi({ viewDialog: "draftingView" }) },
    {
      label: "Duplicate View",
      separator: true,
      disabled: !drawable,
      items: active
        ? [
            { label: "Duplicate", onClick: () => void duplicateView(active.id, false) },
            {
              label: "Duplicate with Detailing",
              disabled: active.viewType === "ThreeD" || active.viewType === "Schedule",
              onClick: () => void duplicateView(active.id, true),
            },
          ]
        : [],
    },
  ];
  return (
    <>
      <button
        className="rb-btn"
        disabled={!app}
        aria-haspopup="menu"
        aria-expanded={!!at}
        title="New View: a plan, ceiling plan, 3D view, section, elevation, callout or drafting view"
        onClick={(e) => {
          const r = e.currentTarget.getBoundingClientRect();
          setAt(at ? null : { x: r.left, y: r.bottom + 2 });
        }}
      >
        {Icons.newView}
        <span>New View ▾</span>
      </button>
      {at && (
        <ContextMenu x={at.x} y={at.y} label="New View" items={items} onClose={() => setAt(null)} />
      )}
    </>
  );
}
