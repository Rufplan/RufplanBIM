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
    ifcReport: null,
    viewDialog: null,
  });
  fake = installFakeBackend();
});

describe("IFC import (ADR-035)", () => {
  it("opens a Revit IFC from the welcome screen and reports what came in", async () => {
    render(<App />);
    fake.openPath = "C:/models/Duplex_A.ifc";
    await userEvent.click(screen.getByRole("button", { name: "Open IFC (Revit)" }));
    const dialog = await screen.findByRole("dialog", { name: "IFC Import" });
    expect(fake.calls.find((c) => c.cmd === "project_import_ifc")?.args).toEqual({
      path: "C:/models/Duplex_A.ifc",
    });
    expect(within(dialog).getByText(/Autodesk Revit Architecture 2011/)).toBeTruthy();
    expect(within(dialog).getByText("57")).toBeTruthy();
    expect(within(dialog).getByText(/Not brought in: 2 stair/)).toBeTruthy();
    expect(within(dialog).getByText(/skylights/)).toBeTruthy();
    expect(within(dialog).getByText("Working with Revit")).toBeTruthy();
    await userEvent.click(within(dialog).getByRole("button", { name: "OK" }));
    expect(screen.queryByRole("dialog", { name: "IFC Import" })).toBeNull();
    // The model is open.
    expect(await screen.findByRole("toolbar", { name: "Tools" })).toBeTruthy();
  });
});
