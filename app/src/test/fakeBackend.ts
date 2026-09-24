import { mockIPC } from "@tauri-apps/api/mocks";
import type { ProjectStatus } from "../ipc";

export interface FakeBackend {
  calls: { cmd: string; args: unknown }[];
  /** Path returned by the next open dialog; null simulates Cancel. */
  openPath: string | null;
  /** Path returned by the next save dialog; null simulates Cancel. */
  savePath: string | null;
  failWith: string | null;
}

export const status = (path: string | null): ProjectStatus => ({
  name: path ? (path.split(/[\\/]/).pop() ?? "").replace(/\.rfproj$/, "") : "Untitled",
  path,
  schemaVersion: 1,
  appVersion: "0.0.1",
});

/** Installs an in-memory stand-in for the Rust commands and dialog plugin. */
export function installFakeBackend(): FakeBackend {
  const fake: FakeBackend = { calls: [], openPath: null, savePath: null, failWith: null };
  let current: ProjectStatus | null = null;

  mockIPC(
    (cmd, args) => {
      fake.calls.push({ cmd, args });
      if (fake.failWith) throw { message: fake.failWith };
      const a = args as Record<string, unknown>;
      switch (cmd) {
        case "core_version":
          return { app: "0.0.1", core: "0.0.1", io: "0.0.1", schemaVersion: 1 };
        case "project_status":
          return current;
        case "project_new":
          return (current = status(null));
        case "project_open":
          return (current = status(a.path as string));
        case "project_save":
          return (current = status((a.path as string | null) ?? current?.path ?? null));
        case "plugin:dialog|open":
          return fake.openPath;
        case "plugin:dialog|save":
          return fake.savePath;
      }
    },
    { shouldMockEvents: true },
  );
  return fake;
}

export const commandsCalled = (fake: FakeBackend) =>
  fake.calls.map((c) => c.cmd).filter((c) => !c.startsWith("plugin:event"));
