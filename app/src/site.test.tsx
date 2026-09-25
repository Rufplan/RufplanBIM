import { beforeEach, describe, expect, it } from "vitest";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { App } from "./App";
import type { ImageryFrame } from "./bindings/ImageryFrame";
import { uvAt } from "./imagery";
import { useAppStore } from "./store";
import { installFakeBackend, type FakeBackend } from "./test/fakeBackend";

let fake: FakeBackend;

beforeEach(() => {
  useAppStore.setState({
    app: null,
    error: null,
    confirm: null,
    rufplan: null,
    paramsOpen: false,
    openViews: [],
    activeView: null,
    selection: [],
    tool: "select",
    siteDialog: null,
  });
  fake = installFakeBackend();
});

async function openProject() {
  render(<App />);
  await userEvent.click(screen.getByRole("button", { name: "New Project" }));
  await screen.findByRole("toolbar", { name: "Tools" });
}

describe("Site tab (ADR-023)", () => {
  it("comes first, before Architecture", async () => {
    await openProject();
    const tabs = within(screen.getByRole("tablist", { name: "Ribbon tabs" }))
      .getAllByRole("tab")
      .map((t) => t.textContent);
    expect(tabs.slice(0, 2)).toEqual(["Site", "Architecture"]);
    await userEvent.click(screen.getByRole("tab", { name: "Site" }));
    for (const name of ["Find Lot", "Site Plan", "Get Topo", "API Keys"])
      expect(screen.getByRole("button", { name })).toBeTruthy();
  });

  it("Find Lot asks for the Google key first; API Keys saves them", async () => {
    await openProject();
    await userEvent.click(screen.getByRole("tab", { name: "Site" }));
    await userEvent.click(screen.getByRole("button", { name: "Find Lot" }));
    const dialog = await screen.findByRole("dialog", { name: "Find Lot" });
    await within(dialog).findByText("Google Maps needs your API key.");
    await userEvent.click(within(dialog).getByRole("button", { name: "Add API Keys" }));
    const keys = await screen.findByRole("dialog", { name: "API Keys" });
    await userEvent.type(within(keys).getByLabelText("Google Maps API key"), "AIzaTEST");
    await userEvent.type(within(keys).getByLabelText("Regrid API token"), "rg-token");
    await userEvent.click(within(keys).getByRole("button", { name: "Save" }));
    expect(fake.calls.find((c) => c.cmd === "site_set_keys")?.args).toEqual({
      google: "AIzaTEST",
      regrid: "rg-token",
    });
    expect(fake.googleKey).toBe("AIzaTEST");
    expect(fake.regrid).toBe(true);
    expect(screen.queryByRole("dialog", { name: "API Keys" })).toBeNull();
  });

  it("Satellite toggles the imagery overlay once there is a lot (ADR-026)", async () => {
    await openProject();
    await userEvent.click(screen.getByRole("tab", { name: "Site" }));
    const button = screen.getByRole("button", { name: "Satellite" });
    expect((button as HTMLButtonElement).disabled).toBe(true);
    const app = useAppStore.getState().app!;
    useAppStore.setState({
      app: {
        ...app,
        site: { id: "s1", address: "1 Main St", lat: 37.7, lon: -122.4, hasTopo: true, ring: [] },
      },
    });
    const on = await screen.findByRole("button", { name: "Satellite" });
    expect((on as HTMLButtonElement).disabled).toBe(false);
    await userEvent.click(on);
    expect(useAppStore.getState().satellite).toBe(true);
    expect(on.getAttribute("aria-pressed")).toBe("true");
    await userEvent.click(on);
    expect(useAppStore.getState().satellite).toBe(false);
  });

  it("maps plan points onto the satellite image, rotated with the site", () => {
    const square: ImageryFrame = {
      lat: 0,
      lon: 0,
      zoom: 20,
      width: 400,
      height: 400,
      corners: [
        { x: 0, y: 0 },
        { x: 1000, y: 0 },
        { x: 1000, y: 2000 },
        { x: 0, y: 2000 },
      ],
    };
    expect(uvAt(square, 0, 0)).toEqual([0, 0]);
    expect(uvAt(square, 500, 2000)).toEqual([0.5, 1]);
    // Turned 90°: the image's lower edge runs up the y axis.
    const turned: ImageryFrame = {
      ...square,
      corners: [
        { x: 0, y: 0 },
        { x: 0, y: 1000 },
        { x: -2000, y: 1000 },
        { x: -2000, y: 0 },
      ],
    };
    const [u, v] = uvAt(turned, -1000, 250);
    expect(u).toBeCloseTo(0.25);
    expect(v).toBeCloseTo(0.5);
  });
});
