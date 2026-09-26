import { activeViewInfo, useAppStore } from "../store";
import { ViewCanvas } from "./ViewCanvas";
import { View3D } from "./View3D";
import { ScheduleView } from "./ScheduleView";

const TYPE_LABEL = {
  Plan: "Floor Plan",
  CeilingPlan: "Ceiling Plan",
  Elevation: "Elevation",
  ThreeD: "3D",
  Section: "Section",
  Schedule: "Schedule",
  Sheet: "Sheet",
} as const;

export function Workspace() {
  const app = useAppStore((s) => s.app);
  const openViews = useAppStore((s) => s.openViews);
  const activeView = useAppStore((s) => s.activeView);
  const openView = useAppStore((s) => s.openView);
  const closeView = useAppStore((s) => s.closeView);
  const tabView = useAppStore((s) => s.app?.views.find((v) => v.id === s.activeView) ?? null);
  const activated = useAppStore((s) =>
    s.activeViewport && s.activeViewport.sheet === s.activeView ? s.activeViewport : null,
  );
  const inner = useAppStore(activeViewInfo);
  const view = tabView;
  if (!app) return null;
  return (
    <section className="workspace">
      <div className="tabs" role="tablist" aria-label="Open views">
        {openViews.map((id) => {
          const v = app.views.find((x) => x.id === id);
          if (!v) return null;
          return (
            <div
              key={id}
              className={`tab${id === activeView ? " active" : ""}`}
              role="tab"
              aria-selected={id === activeView}
            >
              <button className="tab-label" onClick={() => openView(id)}>
                <span className="tab-kind">{TYPE_LABEL[v.viewType]}</span>
                {v.name}
              </button>
              <button
                className="tab-close"
                aria-label={`Close ${v.name}`}
                onClick={() => closeView(id)}
              >
                ×
              </button>
            </div>
          );
        })}
      </div>
      <div className="view-area">
        {view ? (
          view.viewType === "ThreeD" ? (
            <View3D key={view.id} view={view} />
          ) : view.viewType === "Schedule" ? (
            <ScheduleView key={view.id} view={view} />
          ) : activated && inner && inner.id !== view.id ? (
            // A viewport activated on the sheet: its view, in place (ADR-039).
            <ViewCanvas key={`act:${activated.viewport}`} view={inner} onSheet={activated} />
          ) : (
            <ViewCanvas key={view.id} view={view} />
          )
        ) : (
          <div className="view-empty">Open a view from the Project Browser.</div>
        )}
      </div>
    </section>
  );
}

export function StatusBar() {
  const prompt = useAppStore((s) => s.prompt);
  const cursor = useAppStore((s) => s.cursor);
  const view = useAppStore(activeViewInfo);
  return (
    <footer className="statusbar">
      <span className="status-prompt">{prompt}</span>
      <span className="status-cursor">{cursor}</span>
      {view && <span className="status-scale">{view.scaleLabel}</span>}
    </footer>
  );
}
