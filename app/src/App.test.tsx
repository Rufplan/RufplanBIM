import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { mockIPC } from "@tauri-apps/api/mocks";
import { App } from "./App";

describe("App", () => {
  it("shows the core version returned by Rust", async () => {
    mockIPC((cmd) => {
      if (cmd === "core_version") {
        return { app: "0.0.1", core: "0.0.1", io: "0.0.1", schemaVersion: 1 };
      }
    });
    render(<App />);
    await userEvent.click(screen.getByRole("button", { name: "Core version" }));
    expect(await screen.findByTestId("version")).toHaveTextContent("schema 1");
  });

  it("shows an error when the command fails", async () => {
    mockIPC(() => {
      throw new Error("boom");
    });
    render(<App />);
    await userEvent.click(screen.getByRole("button", { name: "Core version" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("boom");
  });
});
