import { beforeEach, describe, expect, it } from "vitest";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { App } from "./App";
import { directory, usePi } from "./components/ProjectInfo";
import { useAppStore } from "./store";
import { installFakeBackend, type FakeBackend } from "./test/fakeBackend";

let fake: FakeBackend;

beforeEach(() => {
  useAppStore.setState({
    app: null,
    error: null,
    openViews: [],
    activeView: null,
    selection: [],
    tool: "select",
    ribbonTab: "Architecture",
  });
  usePi.setState({
    info: null,
    draft: null,
    dirty: false,
    saving: false,
    section: "overview",
    open: null,
    note: null,
  });
  fake = installFakeBackend();
});

async function openProjectInfo() {
  render(<App />);
  await userEvent.click(screen.getByRole("button", { name: "New Project" }));
  await screen.findByRole("toolbar", { name: "Tools" });
  await userEvent.click(screen.getByRole("tab", { name: "Project Info" }));
  return screen.findByRole("complementary", { name: "Project Info Browser" });
}

const lastSet = () =>
  [...fake.calls].reverse().find((c) => c.cmd === "project_info_set")?.args as
    { name: string; number: string; details: Record<string, unknown> } | undefined;

describe("Project Info tab (ADR-084)", () => {
  it("is the first tab and replaces the browser, views and properties", async () => {
    render(<App />);
    await userEvent.click(screen.getByRole("button", { name: "New Project" }));
    const tabs = within(await screen.findByRole("tablist", { name: "Ribbon tabs" }))
      .getAllByRole("tab")
      .map((t) => t.textContent);
    expect(tabs[0]).toBe("Project Info");
    await userEvent.click(screen.getByRole("tab", { name: "Project Info" }));
    const browser = await screen.findByRole("complementary", { name: "Project Info Browser" });
    expect(screen.queryByRole("complementary", { name: "Project browser" })).toBeNull();
    for (const s of ["Overview", "Location", "Client & Owner", "Consultants", "Budget"]) {
      expect(within(browser).getByRole("button", { name: new RegExp(s) })).toBeVisible();
    }
    // Facts from the model.
    expect((await screen.findAllByText("2,000 SF")).length).toBeGreaterThan(0);
  });

  it("saves typed fields after a pause, with the name and number", async () => {
    await openProjectInfo();
    const name = await screen.findByPlaceholderText("Needed to save");
    await userEvent.clear(name);
    await userEvent.type(name, "Oak House");
    await userEvent.type(screen.getByLabelText("Project Type"), "Office");
    await waitFor(() => expect(lastSet()?.name).toBe("Oak House"), { timeout: 2000 });
    expect((lastSet()!.details.overview as { project_type: string }).project_type).toBe("Office");
    expect(fake.projectInfo.identity.name).toBe("Oak House");
    await waitFor(() =>
      expect(screen.getByText("Saved", { selector: ".statusbar span" })).toBeInTheDocument(),
    );
  });

  it("doesn't save without a name", async () => {
    await openProjectInfo();
    const name = await screen.findByPlaceholderText("Needed to save");
    await userEvent.clear(name);
    expect(await screen.findByText("The project needs a name to save.")).toBeVisible();
    await new Promise((r) => setTimeout(r, 800));
    expect(lastSet()).toBeUndefined();
  });

  it("adds a consultant and fills in their firm", async () => {
    const browser = await openProjectInfo();
    await userEvent.click(within(browser).getByRole("button", { name: /Consultants/ }));
    await userEvent.selectOptions(
      await screen.findByRole("combobox", { name: "Add consultant" }),
      "Lighting Designer",
    );
    // The new consultant opens for details.
    const company = await screen.findByPlaceholderText("Consultant company");
    await userEvent.type(company, "Lumen Studio");
    await waitFor(
      () => {
        const team = lastSet()?.details.team as {
          discipline: string;
          contact: { company: string };
        }[];
        expect(team?.[2]).toMatchObject({
          discipline: "Lighting Designer",
          contact: { company: "Lumen Studio" },
        });
      },
      { timeout: 2000 },
    );
    expect(within(browser).getByText("1/3")).toBeInTheDocument();
  });

  it("budgets lines and shows the totals from the backend", async () => {
    const browser = await openProjectInfo();
    await userEvent.click(within(browser).getByRole("button", { name: /Budget/ }));
    await userEvent.type(await screen.findByLabelText("Building construction amount"), "1,000,000");
    await userEvent.click(screen.getByLabelText("Building construction amount").closest("div")!);
    // 1,000,000 + 10% contingency; 1,000,000 / 2,000 SF.
    expect(
      (await screen.findAllByText("$1,100,000", {}, { timeout: 2000 })).length,
    ).toBeGreaterThan(0);
    expect(await screen.findByText("$500/SF")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "+ Add line" }));
    await waitFor(
      () => expect((lastSet()?.details.budget as { lines: unknown[] }).lines).toHaveLength(3),
      { timeout: 2000 },
    );
  });

  it("writes the directory for an email", () => {
    const info = fake.projectInfo;
    const d = {
      name: "Oak House",
      number: "2601",
      details: structuredClone(info.details),
    };
    d.details.client.company = "Oak Holdings";
    d.details.team[1]!.contact = {
      ...d.details.team[1]!.contact,
      company: "Beam & Co",
      email: "a@b.co",
    };
    expect(directory(d)).toBe(
      "Oak House (2601)\n\nClient — Oak Holdings\nStructural Engineer — Beam & Co — a@b.co",
    );
  });
});
