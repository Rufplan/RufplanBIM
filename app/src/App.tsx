import { useEffect, useState } from "react";
import { errorMessage, ipc, type CoreVersion } from "./ipc";
import { handleMenu, newProject, openProject, saveProject, saveProjectAs } from "./fileActions";
import { useAppStore } from "./store";

export function App() {
  const project = useAppStore((s) => s.project);
  const error = useAppStore((s) => s.error);
  const setProject = useAppStore((s) => s.setProject);
  const setError = useAppStore((s) => s.setError);
  const [version, setVersion] = useState<CoreVersion | null>(null);

  useEffect(() => {
    ipc.projectStatus().then(setProject, (err) => setError(errorMessage(err)));
    const unlisten = ipc.onMenu((id) => void handleMenu(id));
    return () => {
      unlisten.then((stop) => stop());
    };
  }, [setProject, setError]);

  async function checkVersion() {
    try {
      setVersion(await ipc.coreVersion());
    } catch (err) {
      setError(errorMessage(err));
    }
  }

  return (
    <main className="app">
      <header className="toolbar">
        <button onClick={newProject}>New</button>
        <button onClick={openProject}>Open…</button>
        <button onClick={saveProject} disabled={!project}>
          Save
        </button>
        <button onClick={saveProjectAs} disabled={!project}>
          Save As…
        </button>
        <span className="spacer" />
        <button onClick={checkVersion}>Core version</button>
      </header>

      {error && (
        <p role="alert" className="error">
          {error} <button onClick={() => setError(null)}>Dismiss</button>
        </p>
      )}

      {project ? (
        <section aria-label="Project" className="panel">
          <h1>{project.name}</h1>
          <dl>
            <dt>File</dt>
            <dd data-testid="project-path">{project.path ?? "Not saved yet"}</dd>
            <dt>Schema</dt>
            <dd>{project.schemaVersion}</dd>
            <dt>Saved by</dt>
            <dd>Rufplan Studio {project.appVersion}</dd>
          </dl>
        </section>
      ) : (
        <section aria-label="Welcome" className="panel">
          <h1>Rufplan Studio</h1>
          <p>Create a new project or open an existing .rfproj file.</p>
        </section>
      )}

      {version && (
        <p data-testid="version" className="muted">
          App {version.app} · core {version.core} · io {version.io} · schema {version.schemaVersion}
        </p>
      )}
    </main>
  );
}
