import { beforeEach, describe, expect, it } from "vitest";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { App } from "./App";
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
});
