import { beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { App } from "./App";
import { guessLevel } from "./render/planImages";
import { useAppStore } from "./store";
import { installFakeBackend, type FakeBackend } from "./test/fakeBackend";

// jsdom can't decode images or draw canvases: sheets come in as prepared pages.
vi.mock("./render/planImages", async (orig) => {
  const real = await orig<typeof import("./render/planImages")>();
  return {
    ...real,
    readPlanFiles: async (files: File[]) =>
      files.map((f) => ({
        name: f.name,
        url: `blob:${f.name}`,
        mediaType: "image/jpeg",
        data: "AAAA",
        width: 1568,
        height: 1210,
      })),
  };
});

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
    viewDialog: null,
  });
  fake = installFakeBackend();
  fake.claudeKey = true;
});

describe("Plans to 3D (ADR-036)", () => {
  it("takes plan sheets with their levels, sends them, and reports the model", async () => {
    render(<App />);
    await userEvent.click(screen.getByRole("button", { name: "New Project" }));
    await screen.findByRole("toolbar", { name: "Tools" });
    await userEvent.click(screen.getByRole("tab", { name: "Architecture" }));
    await userEvent.click(screen.getByRole("button", { name: "Plans to 3D" }));
    const dialog = await screen.findByRole("dialog", { name: "Plans to 3D" });
    const go = within(dialog).getByRole("button", { name: "Build Model" }) as HTMLButtonElement;
    expect(go.disabled).toBe(true);
    await userEvent.upload(within(dialog).getByLabelText("Add plans"), [
      new File(["x"], "fallingwater-first-floor.jpg", { type: "image/jpeg" }),
      new File(["x"], "fallingwater-second-floor.jpg", { type: "image/jpeg" }),
    ]);
    // Levels are guessed from the file names, and can be changed.
    const lv1 = (await within(dialog).findByLabelText("Level of sheet 1")) as HTMLInputElement;
    expect(lv1.value).toBe("First Floor");
    expect((within(dialog).getByLabelText("Level of sheet 2") as HTMLInputElement).value).toBe(
      "Second Floor",
    );
    await userEvent.type(within(dialog).getByLabelText("Elevation of sheet 2"), "11.5");
    await userEvent.type(within(dialog).getByLabelText("Project name"), "Fallingwater");
    await userEvent.type(within(dialog).getByLabelText("Notes"), "stone walls");
    await userEvent.click(within(dialog).getByRole("button", { name: "Build Model" }));
    expect(fake.plans).toMatchObject({
      name: "Fallingwater",
      notes: "stone walls",
      model: "claude-opus-5-5",
      sheets: [
        { level: "First Floor", elevation: null, width: 1568, height: 1210 },
        { level: "Second Floor", elevation: 11.5 },
      ],
    });
    expect(await within(dialog).findByText(/82 walls · 8 doors · 6 windows/)).toBeTruthy();
    expect(within(dialog).getByText(/has no wall near it/)).toBeTruthy();
    expect(within(dialog).getByRole("button", { name: "Open 3D View" })).toBeTruthy();
  });

  it("guesses levels from sheet names", () => {
    expect(guessLevel("Plan - Basement.pdf p1", 0)).toBe("Basement");
    expect(guessLevel("house 2nd floor.png", 0)).toBe("Second Floor");
    expect(guessLevel("scan-003.jpg", 2)).toBe("Third Floor");
  });
});
