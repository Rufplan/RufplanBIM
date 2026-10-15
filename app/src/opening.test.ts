import { afterEach, describe, expect, it } from "vitest";
import { appState, FAKE_IDS } from "./test/fakeBackend";
import { opening, useAppStore } from "./store";

describe("Opening a project", () => {
  afterEach(() => {
    opening.threeD = false;
    useAppStore.setState({ app: null, openViews: [], activeView: null });
  });

  it("starts in its 3D view, and a new project's browser starts afresh", () => {
    opening.threeD = true;
    useAppStore.setState({ app: null, openViews: [], activeView: null });
    const epoch = useAppStore.getState().projectEpoch;
    useAppStore.getState().setApp(appState("C:/a.ruf"), true);
    const s = useAppStore.getState();
    expect(s.activeView).toBe(FAKE_IDS.v3d);
    expect(s.openViews).toEqual([FAKE_IDS.v3d]);
    expect(s.projectEpoch).toBe(epoch + 1);
    // The same project refreshing keeps its views and its browser.
    useAppStore.getState().setApp(appState("C:/a.ruf"));
    expect(useAppStore.getState().projectEpoch).toBe(epoch + 1);
    expect(useAppStore.getState().activeView).toBe(FAKE_IDS.v3d);
  });

  it("starts in its first view when it has no 3D view", () => {
    opening.threeD = true;
    const app = appState("C:/b.ruf");
    const views = app.views.filter((v) => v.viewType !== "ThreeD");
    useAppStore.getState().setApp({ ...app, views }, true);
    expect(useAppStore.getState().activeView).toBe(views[0]?.id);
  });
});
