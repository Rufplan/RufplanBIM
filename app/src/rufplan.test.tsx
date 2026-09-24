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
    cloud: null,
    openViews: [],
    activeView: null,
    selection: [],
    tool: "select",
  });
  fake = installFakeBackend();
});

async function openRufplanTab() {
  render(<App />);
  await userEvent.click(screen.getByRole("button", { name: "New Project" }));
  await userEvent.click(await screen.findByRole("tab", { name: "Rufplan" }));
}

describe("Rufplan integration", () => {
  it("signs in with email and password", async () => {
    await openRufplanTab();
    await userEvent.click(await screen.findByRole("button", { name: /Sign In/ }));
    const dialog = await screen.findByRole("dialog", { name: "Rufplan Account" });
    await userEvent.type(screen.getByLabelText("Email"), "ada@example.com");
    await userEvent.type(screen.getByLabelText("Password"), "secret");
    await userEvent.click(within(dialog).getByRole("button", { name: "Sign In" }));
    expect(await screen.findByText("Signed in as ada@example.com")).toBeInTheDocument();
    expect(dialog).toBeInTheDocument();
    const call = fake.calls.find((c) => c.cmd === "cloud_sign_in");
    expect(call?.args).toEqual({ email: "ada@example.com", password: "secret" });
  });

  it("links a project, then publishes the stage set", async () => {
    await openRufplanTab();
    useAppStore.getState().setCloud({
      configured: true,
      signedIn: true,
      email: "ada@example.com",
      name: "Ada Arch",
    });
    expect(screen.getByRole("button", { name: "Publish" })).toBeDisabled();

    await userEvent.click(screen.getByRole("button", { name: /Link Project/ }));
    await userEvent.click(await screen.findByRole("radio", { name: /Studio Loft/ }));
    await userEvent.click(screen.getByRole("button", { name: "Link" }));
    const link = fake.calls.find((c) => c.cmd === "link_rufplan");
    expect(link?.args).toEqual({ link: { id: "p2", name: "Studio Loft", slug: "studio-loft" } });
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();

    await userEvent.click(screen.getByRole("button", { name: "Publish" }));
    const dialog = await screen.findByRole("dialog", { name: "Publish" });
    await within(dialog).findByText(/4 sheets as a PDF/);
    expect(screen.getByLabelText("Issue name")).toHaveValue("SD Set");
    await userEvent.selectOptions(screen.getByLabelText("Deliverable"), "sd60");
    await userEvent.click(within(dialog).getByRole("button", { name: "Publish" }));
    expect(await screen.findByText("Published to Studio Loft")).toBeInTheDocument();
    expect(screen.getByText(/not supported/)).toBeInTheDocument();
    const pub = fake.calls.find((c) => c.cmd === "publish_to_rufplan");
    expect(pub?.args).toEqual({ name: "SD Set", deliverable: "sd60" });
  });
});
