import { beforeEach, describe, expect, it } from "vitest";
import { confirmDiscard, newProject, openProject, saveProject, saveProjectAs } from "./fileActions";
import { useAppStore } from "./store";
import { appState, commandsCalled, installFakeBackend, type FakeBackend } from "./test/fakeBackend";

let fake: FakeBackend;

beforeEach(() => {
  useAppStore.setState({
    app: null,
    error: null,
    confirm: null,
    openViews: [],
    activeView: null,
    selection: [],
  });
  fake = installFakeBackend();
});

describe("file actions", () => {
  it("cancelling Open does not touch the project", async () => {
    fake.openPath = null;
    await openProject();
    expect(commandsCalled(fake)).toEqual(["plugin:dialog|open"]);
    expect(useAppStore.getState().app).toBeNull();
  });

  it("Open passes the chosen path to Rust and opens the first view", async () => {
    fake.openPath = "C:\\p\\A.rfproj";
    await openProject();
    expect(fake.calls.find((c) => c.cmd === "project_open")?.args).toEqual({
      path: "C:\\p\\A.rfproj",
    });
    const s = useAppStore.getState();
    expect(s.app?.project.name).toBe("A");
    expect(s.activeView).toBe(s.app?.views[0]?.id);
  });

  it("Save on an untitled project asks where to save", async () => {
    await newProject();
    fake.savePath = "C:\\p\\B.rfproj";
    await saveProject();
    expect(commandsCalled(fake)).toEqual(["project_new", "plugin:dialog|save", "project_save"]);
    expect(useAppStore.getState().app?.project.path).toBe("C:\\p\\B.rfproj");
  });

  it("Save on a saved project does not show a dialog", async () => {
    useAppStore.getState().setApp(appState("C:\\p\\C.rfproj"), true);
    await saveProject();
    expect(commandsCalled(fake)).toEqual(["project_save"]);
    expect(fake.calls[0]?.args).toEqual({ path: null });
  });

  it("cancelling Save As changes nothing", async () => {
    useAppStore.getState().setApp(appState("C:\\p\\C.rfproj"), true);
    fake.savePath = null;
    await saveProjectAs();
    expect(commandsCalled(fake)).toEqual(["plugin:dialog|save"]);
  });

  it("errors land in the store", async () => {
    fake.failWith = "disk full";
    await newProject();
    expect(useAppStore.getState().error).toBe("disk full");
  });

  it("New with unsaved changes asks first, and Cancel keeps the project", async () => {
    useAppStore.getState().setApp(appState("C:\\p\\D.rfproj", true), true);
    const pending = newProject();
    await Promise.resolve();
    const confirm = useAppStore.getState().confirm;
    expect(confirm?.message).toContain("D");
    confirm!.resolve("cancel");
    expect(await pending).toBe(false);
    expect(commandsCalled(fake)).toEqual([]);
  });

  it("Don't Save discards without saving", async () => {
    useAppStore.getState().setApp(appState("C:\\p\\D.rfproj", true), true);
    const pending = confirmDiscard();
    await Promise.resolve();
    useAppStore.getState().confirm!.resolve("discard");
    expect(await pending).toBe(true);
    expect(commandsCalled(fake)).toEqual([]);
  });

  it("a clean project needs no confirmation", async () => {
    useAppStore.getState().setApp(appState("C:\\p\\E.rfproj"), true);
    expect(await confirmDiscard()).toBe(true);
  });
});
