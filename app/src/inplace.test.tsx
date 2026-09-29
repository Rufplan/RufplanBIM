import { beforeEach, describe, expect, it } from "vitest";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { App } from "./App";
import { useAppStore } from "./store";
import { installFakeBackend, IN_PLACE_ID, type FakeBackend } from "./test/fakeBackend";
import { formName, ftIn } from "./inplace";

let fake: FakeBackend;
beforeEach(() => {
  useAppStore.setState({
    app: null,
    error: null,
    openViews: [],
    activeView: null,
    selection: [],
    tool: "select",
    picker: null,
    viewDialog: null,
    ribbonTab: "Architecture",
  });
  fake = installFakeBackend();
});

async function openProject() {
  render(<App />);
  await userEvent.click(screen.getByRole("button", { name: "New Project" }));
  await screen.findByRole("toolbar", { name: "Tools" });
  await userEvent.click(screen.getByRole("tab", { name: "Architecture" }));
}

const argsOf = (cmd: string) =>
  fake.calls.filter((c) => c.cmd === cmd).map((c) => c.args as Record<string, unknown>);

describe("Model In-Place (ADR-068)", () => {
  it("asks the category and name, then opens the In-Place Editor", async () => {
    await openProject();
    await userEvent.click(screen.getByRole("button", { name: /Model In-Place/ }));
    const dialog = await screen.findByRole("dialog", { name: "Family Category and Parameters" });
    const list = within(dialog).getByRole("listbox", { name: "Family Category" });
    await within(list).findByRole("option", { name: "Casework" });
    // Filtering narrows the list; the name follows the category until it's typed.
    await userEvent.type(within(dialog).getByLabelText("Filter list"), "case");
    expect(
      within(list)
        .getAllByRole("option")
        .map((o) => o.textContent),
    ).toEqual(["Casework"]);
    await userEvent.click(within(list).getByRole("option", { name: "Casework" }));
    await waitFor(() => expect(within(dialog).getByLabelText("Name")).toHaveValue("Casework 1"));
    const name = within(dialog).getByLabelText("Name");
    await userEvent.clear(name);
    await userEvent.type(name, "Kitchen Counter");
    await userEvent.click(within(dialog).getByRole("button", { name: "OK" }));
    expect(argsOf("in_place_begin")[0]).toMatchObject({
      category: "Casework",
      name: "Kitchen Counter",
    });
    // The editor replaces the ribbon.
    expect(
      await screen.findByRole("tab", { name: "In-Place Editor | Casework : Kitchen Counter" }),
    ).toBeInTheDocument();
    expect(screen.queryByRole("dialog", { name: "Family Category and Parameters" })).toBeNull();
    // Finish needs a form first.
    await userEvent.click(screen.getByRole("button", { name: "Finish Model" }));
    await waitFor(() => expect(useAppStore.getState().error).toContain("Add a form"));
  });

  it("sketches forms with their heights, lists them and finishes the model", async () => {
    await openProject();
    useAppStore.getState().setUi({ viewDialog: "inPlace" });
    const dialog = await screen.findByRole("dialog", { name: "Family Category and Parameters" });
    await within(dialog).findByRole("option", { name: "Walls" });
    await userEvent.click(within(dialog).getByRole("option", { name: "Walls" }));
    await userEvent.click(within(dialog).getByRole("button", { name: "OK" }));
    await screen.findByRole("button", { name: "Finish Model" });

    // Extrusion: the sketch tab shows its start and end.
    await userEvent.click(screen.getByRole("button", { name: /^Extrusion/ }));
    expect(argsOf("in_place_form_begin")[0]).toMatchObject({ kind: "Extrusion", index: null });
    expect(await screen.findByRole("tab", { name: "Modify | Create Extrusion" })).toBeTruthy();
    expect(screen.getByLabelText("Extrusion End")).toHaveValue(`1' 0"`);
    const start = screen.getByLabelText("Extrusion Start");
    expect(start).toHaveValue(`0' 0"`);
    // (The fake reads every length but 0" as 1'.)
    await userEvent.clear(start);
    await userEvent.type(start, "1'{Enter}");
    await waitFor(() =>
      expect(argsOf("sketch_set_form").at(-1)).toMatchObject({
        kind: { Extrusion: { start: 304.8, end: 304.8 } },
      }),
    );
    // Draw something and finish: the form is listed.
    const { ipc } = await import("./ipc");
    await ipc.sketchDraw(
      "Rectangle",
      [
        { x: 0, y: 0 },
        { x: 1000, y: 200 },
      ],
      {
        offset: 0,
        radius: null,
        sides: 6,
      },
    );
    useAppStore.setState({ app: fake.state });
    await userEvent.click(screen.getByRole("button", { name: "Finish" }));
    const forms = await screen.findByRole("list", { name: "Forms in the model" });
    expect(within(forms).getByText("Extrusion 1")).toBeInTheDocument();

    // A sweep's profile, and a void.
    await userEvent.click(screen.getByRole("button", { name: /^Sweep/ }));
    expect(await screen.findByRole("tab", { name: "Modify | Create Sweep Path" })).toBeTruthy();
    expect(screen.getByLabelText("Width")).toHaveValue(`0' 6"`);
    await userEvent.selectOptions(screen.getByLabelText("Profile"), "Circle");
    await waitFor(() =>
      expect(argsOf("sketch_set_form").at(-1)).toMatchObject({
        kind: { Sweep: { profile: { Circle: { diameter: 152.4 } } } },
      }),
    );
    await userEvent.click(screen.getByRole("button", { name: "Cancel" }));
    await userEvent.click(await screen.findByRole("button", { name: /Void Extrusion/ }));
    expect(await screen.findByRole("tab", { name: "Modify | Create Void Extrusion" })).toBeTruthy();
    await userEvent.click(screen.getByRole("button", { name: "Cancel" }));

    // Edit Sketch and Delete for each listed form.
    await userEvent.click(
      await screen.findByRole("button", { name: "Edit Sketch of Extrusion 1" }),
    );
    expect(argsOf("in_place_form_begin").at(-1)).toMatchObject({ kind: "Extrusion", index: 0 });
    expect(
      await screen.findByRole("tab", { name: "Modify | Extrusion > Edit Extrusion" }),
    ).toBeTruthy();
    await userEvent.click(screen.getByRole("button", { name: "Cancel" }));
    await userEvent.click(await screen.findByRole("button", { name: "Finish Model" }));
    await waitFor(() => expect(useAppStore.getState().app?.inPlace).toBeNull());
    expect(argsOf("in_place_finish")).toHaveLength(1);
    // The ribbon is back.
    expect(await screen.findByRole("tab", { name: "Architecture" })).toBeInTheDocument();
  });

  it("offers Edit In-Place for a selected in-place element; Cancel Model discards", async () => {
    await openProject();
    useAppStore.getState().select([IN_PLACE_ID]);
    await userEvent.click(await screen.findByRole("button", { name: /Edit In-Place/ }));
    expect(argsOf("in_place_edit")[0]).toMatchObject({ id: IN_PLACE_ID });
    const forms = await screen.findByRole("list", { name: "Forms in the model" });
    await userEvent.click(within(forms).getByRole("button", { name: "Delete Extrusion 1" }));
    expect(argsOf("in_place_delete_form")[0]).toMatchObject({ index: 0 });
    await userEvent.click(screen.getByRole("button", { name: "Cancel Model" }));
    await waitFor(() => expect(useAppStore.getState().app?.inPlace).toBeNull());
  });

  it("formats lengths and reads form labels", () => {
    expect(ftIn(304.8)).toBe(`1' 0"`);
    expect(ftIn(1676.4)).toBe(`5' 6"`);
    expect(ftIn(114.3)).toBe(`0' 4 1/2"`);
    expect(formName("Void Extrusion 2")).toBe("VoidExtrusion");
    expect(formName("Blend 1")).toBe("Blend");
    expect(formName("Sweep 3")).toBe("Sweep");
    expect(formName("Extrusion 1")).toBe("Extrusion");
  });
});
