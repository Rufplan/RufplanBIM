import { activeViewInfo, useAppStore } from "../store";
import { ViewCanvas } from "./ViewCanvas";
import { View3D } from "./View3D";

const TYPE_LABEL = {
  Plan: "Floor Plan",
  CeilingPlan: "Ceiling Plan",
  Elevation: "Elevation",
  ThreeD: "3D",
} as const;

export function Workspace() {
  const app = useAppStore((s) => s.app);
  const openViews = useAppStore((s) => s.openViews);
  const activeView = useAppStore((s) => s.activeView);
  const openView = useAppStore((s) => s.openView);
  const closeView = useAppStore((s) => s.closeView);
  const view = useAppStore(activeViewInfo);
  if (!app) return null;
  return (
    <section className="workspace">
      <div className="tabs" role="tablist">
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
            <View3D key={view.id} />
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
