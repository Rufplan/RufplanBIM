import { useState } from "react";
import { ipc, type CoreVersion } from "./ipc";

export function App() {
  const [version, setVersion] = useState<CoreVersion | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function checkVersion() {
    try {
      setVersion(await ipc.coreVersion());
      setError(null);
    } catch (e) {
      setError(String(e));
    }
  }

  return (
    <main className="app">
      <h1>Rufplan Studio</h1>
      <button onClick={checkVersion}>Core version</button>
      {version && (
        <p data-testid="version">
          App {version.app} · core {version.core} · io {version.io} · schema {version.schemaVersion}
        </p>
      )}
      {error && <p role="alert">{error}</p>}
    </main>
  );
}
