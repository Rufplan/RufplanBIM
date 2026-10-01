import { activeViewInfo, useAppStore } from "../store";
import { ActiveWorkset } from "./Worksets";
import { ViewCanvas } from "./ViewCanvas";
import { View3D } from "./View3D";
import { ScheduleView } from "./ScheduleView";
import { EditModelButton, useEditModelAllowed } from "./EditModel";

const TYPE_LABEL = {
  Plan: "Floor Plan",
  CeilingPlan: "Ceiling Plan",
  Elevation: "Elevation",
  ThreeD: "3D",
  Section: "Section",
  Schedule: "Schedule",
  Sheet: "Sheet",
  Drafting: "Drafting View",
  Rendering: "Rendering",
} as const;

export function Workspace() {
  const app = useAppStore((s) => s.app);
  const editAllowed = useEditModelAllowed();
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
      <div className={`view-area${editAllowed ? " has-edit" : ""}`}>
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
        <EditModelButton />
      </div>
    </section>
  );
}

export function StatusBar() {
  const prompt = useAppStore((s) => (s.tool === "select" && s.hoverLabel) || s.prompt);
  const cursor = useAppStore((s) => s.cursor);
  const view = useAppStore(activeViewInfo);
  const selected = useAppStore((s) => s.selection.length);
  return (
    <footer className="statusbar">
      <span className="status-prompt">{prompt}</span>
      <span className="status-cursor">{cursor}</span>
      {selected > 0 && (
        <button
          className="status-filter"
          title="Filter the selection by category"
          aria-label={`Filter the selection (${selected} selected)`}
          onClick={() => useAppStore.getState().setUi({ viewDialog: "filter" })}
        >
          <svg viewBox="0 0 24 24" width="14" height="14" aria-hidden>
            <path d="M3 4h18l-7 8.5V19l-4 2v-8.5z" fill="currentColor" />
          </svg>
          {selected}
        </button>
      )}
      <ActiveWorkset compact />
      {view && <span className="status-scale">{view.scaleLabel}</span>}
    </footer>
  );
}
