import { beforeEach, describe, expect, it } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { emit } from "@tauri-apps/api/event";
import { App } from "./App";
import { useAppStore } from "./store";
import { commandsCalled, installFakeBackend, type FakeBackend } from "./test/fakeBackend";

let fake: FakeBackend;

beforeEach(() => {
  useAppStore.setState({ project: null, error: null });
  fake = installFakeBackend();
});

describe("App", () => {
  it("shows the core version returned by Rust", async () => {
    render(<App />);
    await userEvent.click(screen.getByRole("button", { name: "Core version" }));
    expect(await screen.findByTestId("version")).toHaveTextContent("schema 1");
  });

  it("starts on the welcome screen with Save disabled", async () => {
    render(<App />);
    expect(screen.getByRole("region", { name: "Welcome" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Save" })).toBeDisabled();
    await waitFor(() => expect(commandsCalled(fake)).toContain("project_status"));
  });

  it("New shows an untitled project", async () => {
    render(<App />);
    await userEvent.click(screen.getByRole("button", { name: "New" }));
    expect(await screen.findByRole("heading", { name: "Untitled" })).toBeInTheDocument();
    expect(screen.getByTestId("project-path")).toHaveTextContent("Not saved yet");
  });

  it("responds to the native File > Open menu", async () => {
    fake.openPath = "C:\\Projects\\House.rfproj";
    render(<App />);
    await waitFor(() => expect(commandsCalled(fake)).toContain("project_status"));
    await emit("menu", "file.open");
    expect(await screen.findByRole("heading", { name: "House" })).toBeInTheDocument();
  });

  it("shows command errors", async () => {
    render(<App />);
    fake.failWith = "could not open x: not a project";
    await userEvent.click(screen.getByRole("button", { name: "New" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("not a project");
  });
});
