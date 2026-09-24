import { beforeEach, describe, expect, it } from "vitest";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { emit } from "@tauri-apps/api/event";
import { App } from "./App";
import { useAppStore } from "./store";
import { FAKE_IDS, commandsCalled, installFakeBackend, type FakeBackend } from "./test/fakeBackend";

let fake: FakeBackend;

beforeEach(() => {
  useAppStore.setState({
    app: null,
    error: null,
    confirm: null,
    openViews: [],
    activeView: null,
    selection: [],
    tool: "select",
  });
  fake = installFakeBackend();
});

describe("App", () => {
  it("starts on the welcome screen", async () => {
    render(<App />);
    expect(screen.getByRole("button", { name: "New Project" })).toBeInTheDocument();
    await waitFor(() => expect(commandsCalled(fake)).toContain("app_state"));
  });

  it("New Project shows the workspace with browser, ribbon and a plan tab", async () => {
    render(<App />);
    await userEvent.click(screen.getByRole("button", { name: "New Project" }));
    expect(await screen.findByRole("toolbar", { name: "Tools" })).toBeInTheDocument();
    expect(screen.getByRole("complementary", { name: "Project browser" })).toBeInTheDocument();
    const views = screen.getByRole("tablist", { name: "Open views" });
    expect(within(views).getByRole("tab", { selected: true })).toHaveTextContent("Level 1");
  });

  it("changing the design stage calls set_property on project info", async () => {
    render(<App />);
    await userEvent.click(screen.getByRole("button", { name: "New Project" }));
    await userEvent.selectOptions(await screen.findByLabelText("Design stage"), FAKE_IDS.dd);
    const call = fake.calls.find((c) => c.cmd === "set_property");
    expect(call?.args).toEqual({ id: FAKE_IDS.info, key: "current_stage", value: FAKE_IDS.dd });
  });

  it("keyboard shortcut WA picks the wall tool", async () => {
    render(<App />);
    await userEvent.click(screen.getByRole("button", { name: "New Project" }));
    await screen.findByRole("toolbar", { name: "Tools" });
    await userEvent.keyboard("wa");
    expect(useAppStore.getState().tool).toBe("wall");
    expect(screen.getByRole("button", { name: "Wall", pressed: true })).toBeInTheDocument();
  });

  it("ribbon tabs switch between tool groups", async () => {
    render(<App />);
    await userEvent.click(screen.getByRole("button", { name: "New Project" }));
    await screen.findByRole("toolbar", { name: "Tools" });
    expect(screen.queryByRole("button", { name: "New Sheet" })).not.toBeInTheDocument();
    await userEvent.click(screen.getByRole("tab", { name: "View" }));
    expect(screen.getByRole("button", { name: "New Sheet" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Wall" })).not.toBeInTheDocument();
    await userEvent.click(screen.getByRole("tab", { name: "Annotate" }));
    expect(screen.getByRole("button", { name: "Dimension" })).toBeInTheDocument();
  });

  it("responds to the native File > Open menu", async () => {
    fake.openPath = "C:\\Projects\\House.rfproj";
    render(<App />);
    await waitFor(() => expect(commandsCalled(fake)).toContain("app_state"));
    await emit("menu", "file.open");
    await waitFor(() => expect(useAppStore.getState().app?.project.name).toBe("House"));
  });

  it("shows command errors", async () => {
    render(<App />);
    fake.failWith = "could not open x: not a project";
    await userEvent.click(screen.getByRole("button", { name: "New Project" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("not a project");
  });
});
