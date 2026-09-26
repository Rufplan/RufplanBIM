import { beforeEach, describe, expect, it } from "vitest";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { App } from "./App";
import { meshColor } from "./components/View3D";
import { surfaceFor } from "./render/pathtrace";
import { useAppStore } from "./store";
import { installFakeBackend, type FakeBackend } from "./test/fakeBackend";

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
});

async function openLibrary() {
  render(<App />);
  await userEvent.click(screen.getByRole("button", { name: "New Project" }));
  await screen.findByRole("toolbar", { name: "Tools" });
  await userEvent.click(screen.getByRole("tab", { name: "Architecture" }));
  await userEvent.click(screen.getByRole("button", { name: "Load Windows" }));
  return screen.findByRole("dialog", { name: "Window Library" });
}

const loads = () =>
  fake.calls
    .filter((c) => c.cmd === "load_window_types")
    .map((c) => c.args as { specs: unknown[] });

describe("Window Library (ADR-031)", () => {
  it("lists families and sizes, and loads the checked sizes with options", async () => {
    const dialog = await openLibrary();
    // Double-hung first, with its sizes as elevation thumbnails.
    const sizes = await within(dialog).findByRole("list", { name: "Double-Hung sizes" });
    expect(within(sizes).getAllByRole("listitem")).toHaveLength(2);
    expect(within(sizes).getByRole("img", { name: 'Double Hung 36" x 60"' })).toBeTruthy();
    // Colonial grilles in black, on both sizes.
    await userEvent.selectOptions(within(dialog).getByLabelText("Grille"), "Colonial");
    await userEvent.click(within(dialog).getByRole("radio", { name: "Black" }));
    await userEvent.click(within(dialog).getByLabelText('Select Double Hung 30" x 60"'));
    await userEvent.click(within(dialog).getByLabelText('Select Double Hung 36" x 60"'));
    await userEvent.click(within(dialog).getByRole("button", { name: "Load Checked (2)" }));
    expect(loads()[0]!.specs).toMatchObject([
      { family: "DoubleHung", units: 1, grille: "Colonial", finish: "Black" },
      { family: "DoubleHung", width: 36 * 25.4, grille: "Colonial", finish: "Black" },
    ]);
    expect(useAppStore.getState().app?.windowTypes.map((t) => t.name)).toContain("Loaded window 2");
  });

  it("previews a custom size and places it", async () => {
    const dialog = await openLibrary();
    await userEvent.click(await within(dialog).findByRole("button", { name: /Storefront/ }));
    // Storefront takes no grilles.
    expect((within(dialog).getByLabelText("Grille") as HTMLSelectElement).disabled).toBe(true);
    await userEvent.type(within(dialog).getByLabelText("Width (in)"), "144");
    await userEvent.type(within(dialog).getByLabelText("Height (in)"), "108");
    await userEvent.type(within(dialog).getByLabelText("Sill (in)"), "6");
    expect(await within(dialog).findByRole("heading", { name: 'Window 144" x 108"' })).toBeTruthy();
    await userEvent.click(within(dialog).getByRole("button", { name: "Load & Place" }));
    expect(loads()[0]!.specs).toEqual([
      {
        family: "Storefront",
        units: 1,
        width: 144 * 25.4,
        height: 108 * 25.4,
        sill: 6 * 25.4,
        grille: "None",
        finish: "White",
      },
    ]);
    // The window tool picks up the new type.
    const s = useAppStore.getState();
    expect(s.tool).toBe("window");
    expect(s.toolTypes.window).toBe("00000000-0000-7000-8000-0000000000b0");
    expect(screen.queryByRole("dialog", { name: "Window Library" })).toBeNull();
  });

  it("draws frames opaque in their finish and glass see-through", () => {
    const frame = { category: "Window", color: [34, 34, 34], exterior: false } as const;
    const glass = { category: "Window", color: null, exterior: false } as const;
    expect(meshColor(frame as never)).toBe(0x222222);
    expect(surfaceFor(frame as never).transmission).toBe(0);
    expect(surfaceFor(glass as never).transmission).toBe(1);
  });
});
