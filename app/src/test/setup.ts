import "@testing-library/jest-dom/vitest";
import { afterEach } from "vitest";
import { cleanup } from "@testing-library/react";
import { clearMocks } from "@tauri-apps/api/mocks";

afterEach(async () => {
  cleanup();
  // Let effect cleanups (async event unlisteners) run before the mocks are torn down.
  await new Promise((resolve) => setTimeout(resolve, 0));
  clearMocks();
});
