// The only module that talks to Rust. Payload types are generated from Rust by ts-rs.
import { invoke } from "@tauri-apps/api/core";
import type { CoreVersion } from "./bindings/CoreVersion";

export type { CoreVersion };

export const ipc = {
  coreVersion: () => invoke<CoreVersion>("core_version"),
};
