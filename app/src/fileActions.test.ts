import { beforeEach, describe, expect, it } from "vitest";
import { newProject, openProject, saveProject, saveProjectAs } from "./fileActions";
import { useAppStore } from "./store";
import { commandsCalled, installFakeBackend, status, type FakeBackend } from "./test/fakeBackend";

let fake: FakeBackend;

beforeEach(() => {
  useAppStore.setState({ project: null, error: null });
  fake = installFakeBackend();
});

describe("file actions", () => {
  it("cancelling Open does not touch the project", async () => {
    fake.openPath = null;
    await openProject();
    expect(commandsCalled(fake)).toEqual(["plugin:dialog|open"]);
    expect(useAppStore.getState().project).toBeNull();
  });

  it("Open passes the chosen path to Rust", async () => {
    fake.openPath = "C:\\p\\A.rfproj";
    await openProject();
    expect(fake.calls.find((c) => c.cmd === "project_open")?.args).toEqual({
      path: "C:\\p\\A.rfproj",
    });
    expect(useAppStore.getState().project?.name).toBe("A");
  });

  it("Save on an untitled project asks where to save", async () => {
    await newProject();
    fake.savePath = "C:\\p\\B.rfproj";
    await saveProject();
    expect(commandsCalled(fake)).toEqual(["project_new", "plugin:dialog|save", "project_save"]);
    expect(useAppStore.getState().project?.path).toBe("C:\\p\\B.rfproj");
  });

  it("Save on a saved project does not show a dialog", async () => {
    useAppStore.setState({ project: status("C:\\p\\C.rfproj") });
    await saveProject();
    expect(commandsCalled(fake)).toEqual(["project_save"]);
    expect(fake.calls[0]?.args).toEqual({ path: null });
  });

  it("cancelling Save As changes nothing", async () => {
    useAppStore.setState({ project: status("C:\\p\\C.rfproj") });
    fake.savePath = null;
    await saveProjectAs();
    expect(commandsCalled(fake)).toEqual(["plugin:dialog|save"]);
  });

  it("Save does nothing without an open project", async () => {
    await saveProject();
    expect(fake.calls).toEqual([]);
  });

  it("errors land in the store", async () => {
    fake.failWith = "disk full";
    await newProject();
    expect(useAppStore.getState().error).toBe("disk full");
  });
});
