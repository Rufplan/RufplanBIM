import { dialogs, errorMessage, ipc, type ProjectStatus } from "./ipc";
import { useAppStore } from "./store";

// Menu item ids, mirrored in app/src-tauri/src/menu.rs.
export const MENU = {
  new: "file.new",
  open: "file.open",
  save: "file.save",
  saveAs: "file.save_as",
} as const;

async function run(action: () => Promise<ProjectStatus | null | undefined>) {
  const { setProject, setError } = useAppStore.getState();
  try {
    const status = await action();
    if (status !== undefined) setProject(status);
  } catch (err) {
    setError(errorMessage(err));
  }
}

// Returning undefined means the user cancelled a dialog and nothing changed.

export const newProject = () => run(() => ipc.projectNew());

export const openProject = () =>
  run(async () => {
    const path = await dialogs.pickProjectToOpen();
    return path ? ipc.projectOpen(path) : undefined;
  });

export const saveProjectAs = () =>
  run(async () => {
    const current = useAppStore.getState().project;
    if (!current) return undefined;
    const path = await dialogs.pickProjectSaveLocation(current.name);
    return path ? ipc.projectSave(path) : undefined;
  });

export const saveProject = () => {
  const current = useAppStore.getState().project;
  if (!current) return Promise.resolve();
  if (!current.path) return saveProjectAs();
  return run(() => ipc.projectSave());
};

export function handleMenu(id: string) {
  switch (id) {
    case MENU.new:
      return newProject();
    case MENU.open:
      return openProject();
    case MENU.save:
      return saveProject();
    case MENU.saveAs:
      return saveProjectAs();
  }
}
