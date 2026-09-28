import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { NavBar, SunPanel } from "./components/View3DPanels";
import { useAppStore } from "./store";
import { appState, installFakeBackend, type FakeBackend } from "./test/fakeBackend";
import { glareWeight } from "./render/pathtrace";

let fake: FakeBackend;
beforeEach(() => {
  fake = installFakeBackend();
  useAppStore.setState({ app: appState(null), sunPreview: null, exposure3d: 1, nav3d: "orbit" });
});
afterEach(() => vi.useRealTimers());

const argsOf = (cmd: string) =>
  fake.calls.filter((c) => c.cmd === cmd).map((c) => c.args as Record<string, unknown>);

describe("the 3D view's sun panel (ADR-063)", () => {
  it("previews the time as it's dragged and saves it once left alone", async () => {
    render(<SunPanel />);
    expect(screen.getByRole("button", { name: "Sun" }).textContent).toContain("3:00 PM");
    await userEvent.click(screen.getByRole("button", { name: "Sun" }));
    vi.useFakeTimers();
    fireEvent.change(screen.getByLabelText("Time of day"), { target: { value: "18.5" } });
    // The view shows it at once; nothing's saved yet.
    expect(useAppStore.getState().sunPreview?.hour).toBe(18.5);
    expect(argsOf("set_sun_settings")).toHaveLength(0);
    fireEvent.change(screen.getByLabelText("Time of day"), { target: { value: "19" } });
    await act(async () => {
      vi.advanceTimersByTime(800);
    });
    vi.useRealTimers();
    await waitFor(() => expect(argsOf("set_sun_settings")).toHaveLength(1));
    expect(argsOf("set_sun_settings")[0]).toMatchObject({ settings: { hour: 19, month: 6 } });
    expect(argsOf("sun_for").length).toBeGreaterThan(0);
  });

  it("sets the exposure for the session and applies presets", async () => {
    render(<SunPanel />);
    await userEvent.click(screen.getByRole("button", { name: "Sun" }));
    fireEvent.change(screen.getByLabelText("Exposure"), { target: { value: "1.4" } });
    expect(useAppStore.getState().exposure3d).toBe(1.4);
    await userEvent.selectOptions(screen.getByLabelText("Presets"), "Winter Solstice, Noon");
    expect(useAppStore.getState().sunPreview).toMatchObject({ month: 12, day: 21, hour: 12 });
  });
});

describe("Enscape's Orbit, Walk and Fly (ADR-063)", () => {
  it("switches mode and shows the keys", async () => {
    render(<NavBar />);
    expect(screen.getByRole("radio", { name: "Orbit" })).toHaveAttribute("aria-checked", "true");
    await userEvent.click(screen.getByRole("radio", { name: "Walk" }));
    expect(useAppStore.getState().nav3d).toBe("walk");
    expect(screen.getByText(/W A S D move/)).toBeInTheDocument();
    await userEvent.click(screen.getByRole("radio", { name: "Fly" }));
    expect(screen.getByText(/Q E down\/up/)).toBeInTheDocument();
  });
});

describe("render lens effects (ADR-063)", () => {
  it("glare blooms only from the brightest pixels", () => {
    expect(glareWeight(128, 128, 128)).toBe(0);
    expect(glareWeight(255, 255, 255)).toBeCloseTo(1, 9);
    expect(glareWeight(230, 230, 230)).toBeGreaterThan(0);
    expect(glareWeight(230, 230, 230)).toBeLessThan(1);
  });
});
