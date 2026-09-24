// The only module that talks to Rust. Payload types are generated from Rust by ts-rs.
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open, save } from "@tauri-apps/plugin-dialog";
import type { CommandError } from "./bindings/CommandError";
import type { CoreVersion } from "./bindings/CoreVersion";
import type { ProjectStatus } from "./bindings/ProjectStatus";

export type { CommandError, CoreVersion, ProjectStatus };

const PROJECT_FILTER = [{ name: "Rufplan Studio project", extensions: ["rfproj"] }];

export const ipc = {
  coreVersion: () => invoke<CoreVersion>("core_version"),
  projectStatus: () => invoke<ProjectStatus | null>("project_status"),
  projectNew: () => invoke<ProjectStatus | null>("project_new"),
  projectOpen: (path: string) => invoke<ProjectStatus | null>("project_open", { path }),
  /** Omit `path` to save to the project's current location. */
  projectSave: (path?: string) =>
    invoke<ProjectStatus | null>("project_save", { path: path ?? null }),

  /** Native menu clicks, forwarded by Rust as the menu item id. */
  onMenu: (handler: (id: string) => void): Promise<UnlistenFn> =>
    listen<string>("menu", (event) => handler(event.payload)),
};

export const dialogs = {
  /** Resolves to the chosen path, or null if cancelled. */
  pickProjectToOpen: async (): Promise<string | null> => {
    const picked = await open({ multiple: false, directory: false, filters: PROJECT_FILTER });
    return typeof picked === "string" ? picked : null;
  },
  pickProjectSaveLocation: (defaultName: string): Promise<string | null> =>
    save({ defaultPath: `${defaultName}.rfproj`, filters: PROJECT_FILTER }),
};

/** Turns anything a command rejects with into a user-facing message. */
export function errorMessage(err: unknown): string {
  if (typeof err === "object" && err !== null && "message" in err) {
    return String((err as CommandError).message);
  }
  return String(err);
}
