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
    picker: null,
  });
  fake = installFakeBackend();
});

async function openProject() {
  render(<App />);
  await userEvent.click(screen.getByRole("button", { name: "New Project" }));
  await screen.findByRole("toolbar", { name: "Tools" });
  await userEvent.click(screen.getByRole("tab", { name: "Architecture" }));
}

const calls = (cmd: string) =>
  fake.calls.filter((c) => c.cmd === cmd).map((c) => c.args as Record<string, unknown>);

describe("Door and window type pickers (ADR-031, ADR-033)", () => {
  it("Door opens the picker of the project's door types, rendered, and places the pick", async () => {
    await openProject();
    await userEvent.click(screen.getByRole("button", { name: "Door" }));
    const dialog = await screen.findByRole("dialog", { name: "Door Types" });
    const grid = within(dialog).getByRole("list", { name: "Project doors" });
    // Each type is a rendered thumbnail (Rust sends the triangles).
    expect(within(grid).getAllByRole("listitem").length).toBe(
      useAppStore.getState().app!.doorTypes.length,
    );
    expect(calls("opening_thumbnail").length).toBeGreaterThan(0);
    await userEvent.click(within(dialog).getByRole("button", { name: "Place Door" }));
    expect(screen.queryByRole("dialog", { name: "Door Types" })).toBeNull();
    expect(useAppStore.getState().tool).toBe("door");
  });

  it("loads a library door with a leaf style and finish and places it", async () => {
    await openProject();
    await userEvent.click(screen.getByRole("button", { name: "Load Doors" }));
    const dialog = await screen.findByRole("dialog", { name: "Door Types" });
    const sizes = await within(dialog).findByRole("list", { name: "Sizes" });
    await userEvent.click(within(sizes).getByTitle('Single Flush 36" x 84"'));
    await userEvent.selectOptions(within(dialog).getByLabelText("Leaf style"), "SixPanel");
    await userEvent.click(within(dialog).getByRole("radio", { name: "Walnut" }));
    await userEvent.click(within(dialog).getByRole("button", { name: "Load & Place" }));
    expect(calls("load_door_types")[0]!.specs).toEqual([
      {
        family: "SingleFlush",
        leaf: "SixPanel",
        panels: 0,
        width: 36 * 25.4,
        height: 84 * 25.4,
        finish: "Walnut",
      },
    ]);
    const s = useAppStore.getState();
    expect(s.tool).toBe("door");
    expect(s.toolTypes.door).toBe("00000000-0000-7000-8000-0000000000d0");
  });

  it("windows: a custom storefront size from the library", async () => {
    await openProject();
    await userEvent.click(screen.getByRole("button", { name: "Load Windows" }));
    const dialog = await screen.findByRole("dialog", { name: "Window Types" });
    await userEvent.click(await within(dialog).findByRole("button", { name: "Storefront" }));
    expect((within(dialog).getByLabelText("Grille") as HTMLSelectElement).disabled).toBe(true);
    await userEvent.type(within(dialog).getByLabelText("Width (in)"), "144");
    await userEvent.type(within(dialog).getByLabelText("Height (in)"), "108");
    await userEvent.type(within(dialog).getByLabelText("Sill (in)"), "6");
    expect(await within(dialog).findByRole("heading", { name: 'Window 144" x 108"' })).toBeTruthy();
    await userEvent.click(within(dialog).getByRole("button", { name: "Load & Place" }));
    expect(calls("load_window_types")[0]!.specs).toEqual([
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
    expect(useAppStore.getState().tool).toBe("window");
    expect(useAppStore.getState().toolTypes.window).toBe("00000000-0000-7000-8000-0000000000b0");
  });

  it("changes the selected window's type from the picker", async () => {
    await openProject();
    const win = "00000000-0000-7000-8000-00000000f001";
    fake.properties = { [win]: { category: "Window" } };
    useAppStore.getState().select([win]);
    await userEvent.click(screen.getByRole("button", { name: "Window" }));
    const dialog = await screen.findByRole("dialog", { name: "Window Types" });
    expect(within(dialog).getByText("Changing 1 selected window")).toBeTruthy();
    await userEvent.click(within(dialog).getByRole("button", { name: "Change Selected (1)" }));
    expect(calls("set_property").at(-1)).toMatchObject({ id: win, key: "type" });
    expect(useAppStore.getState().selection).toEqual([win]);
  });

  it("draws frames opaque in their finish and glass see-through", () => {
    const frame = { category: "Window", color: [34, 34, 34], exterior: false } as const;
    const glass = { category: "Door", color: null, exterior: false } as const;
    expect(meshColor(frame as never)).toBe(0x222222);
    expect(surfaceFor(frame as never).transmission).toBe(0);
    expect(surfaceFor(glass as never).transmission).toBe(1);
    expect(meshColor(glass as never)).toBe(0x9fe3f7);
  });
});
