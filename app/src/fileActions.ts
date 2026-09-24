import { dialogs, errorMessage, ipc, type AppState } from "./ipc";
import { useAppStore } from "./store";

// Menu item ids, mirrored in app/src-tauri/src/menu.rs.
export const MENU = {
  new: "file.new",
  open: "file.open",
  save: "file.save",
  saveAs: "file.save_as",
  exit: "file.exit",
  undo: "edit.undo",
  redo: "edit.redo",
  delete: "edit.delete",
} as const;

/** Runs a command that returns a new app state; errors land in the store. */
export async function apply(
  action: () => Promise<AppState | null | undefined>,
  fresh = false,
): Promise<boolean> {
  const { setApp, setError } = useAppStore.getState();
  try {
    const state = await action();
    if (state !== undefined) setApp(state, fresh);
    return state !== undefined;
  } catch (err) {
    setError(errorMessage(err));
    return false;
  }
}

function askToSave(message: string): Promise<"save" | "discard" | "cancel"> {
  return new Promise((resolve) => {
    useAppStore.getState().setConfirm({
      message,
      resolve: (choice) => {
        useAppStore.getState().setConfirm(null);
        resolve(choice);
      },
    });
  });
}

/** Resolves true when it is OK to discard the open project (saved, or user said so). */
export async function confirmDiscard(): Promise<boolean> {
  const project = useAppStore.getState().app?.project;
  if (!project?.dirty) return true;
  const choice = await askToSave(`Save changes to ${project.name}?`);
  if (choice === "cancel") return false;
  if (choice === "discard") return true;
  return saveProject();
}

export const newProject = async () => {
  if (!(await confirmDiscard())) return false;
  return apply(() => ipc.projectNew(), true);
};

export const sampleProject = async () => {
  if (!(await confirmDiscard())) return false;
  return apply(() => ipc.projectSample(), true);
};

export const openProject = async () => {
  if (!(await confirmDiscard())) return false;
  const path = await dialogs.pickProjectToOpen();
  if (!path) return false;
  return apply(() => ipc.projectOpen(path), true);
};

export const saveProjectAs = async () => {
  const current = useAppStore.getState().app?.project;
  if (!current) return false;
  const path = await dialogs.pickProjectSaveLocation(current.name);
  if (!path) return false;
  return apply(() => ipc.projectSave(path));
};

export const saveProject = async () => {
  const current = useAppStore.getState().app?.project;
  if (!current) return false;
  if (!current.path) return saveProjectAs();
  return apply(() => ipc.projectSave());
};

export const exitApp = async () => {
  if (!(await confirmDiscard())) return;
  await ipc.exit(true);
};

export const undo = () => apply(() => ipc.undo());
export const redo = () => apply(() => ipc.redo());

export const deleteSelection = async () => {
  const { selection, select } = useAppStore.getState();
  if (selection.length === 0) return false;
  const ok = await apply(() => ipc.deleteElements(selection));
  if (ok) select([]);
  return ok;
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
    case MENU.exit:
      return exitApp();
    case MENU.undo:
      return undo();
    case MENU.redo:
      return redo();
    case MENU.delete:
      return deleteSelection();
  }
}
