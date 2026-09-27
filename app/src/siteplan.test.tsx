import { beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Workspace } from "./components/Workspace";
import { ipc } from "./ipc";
import { useAppStore } from "./store";
import { appState, FAKE_IDS, installFakeBackend } from "./test/fakeBackend";

beforeEach(() => {
  installFakeBackend();
});

describe("site plan grids toggle (ADR-046)", () => {
  it("hides and shows the gridlines of the site plan", async () => {
    const app = appState(null);
    const plan = app.views.find((v) => v.id === FAKE_IDS.plan1)!;
    const site = { ...plan, id: "00000000-0000-7000-8000-0000000000b1", name: "Site", site: true };
    app.views.push(site);
    useAppStore.setState({ app, openViews: [site.id], activeView: site.id, activeViewport: null });
    const spy = vi.spyOn(ipc, "setCategoryVisible").mockResolvedValue(app);
    render(<Workspace />);
    const chip = screen.getByRole("button", { name: "Grids" });
    expect(chip.getAttribute("aria-pressed")).toBe("true");
    await userEvent.click(chip);
    expect(spy).toHaveBeenCalledWith(site.id, ["Grid"], false);
    spy.mockRestore();
  });

  it("isn't offered on ordinary plans", () => {
    const app = appState(null);
    useAppStore.setState({
      app,
      openViews: [FAKE_IDS.plan1],
      activeView: FAKE_IDS.plan1,
      activeViewport: null,
    });
    render(<Workspace />);
    expect(screen.queryByRole("button", { name: "Grids" })).toBeNull();
  });
});
