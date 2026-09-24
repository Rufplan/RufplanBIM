import { create } from "zustand";
import type { ProjectStatus } from "./ipc";

// UI state only. The project itself lives in Rust; this mirrors its last reported status.
interface AppState {
  project: ProjectStatus | null;
  error: string | null;
  setProject: (project: ProjectStatus | null) => void;
  setError: (error: string | null) => void;
}

export const useAppStore = create<AppState>((set) => ({
  project: null,
  error: null,
  setProject: (project) => set({ project, error: null }),
  setError: (error) => set({ error }),
}));
