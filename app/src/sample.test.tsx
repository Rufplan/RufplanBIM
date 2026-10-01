import { beforeEach, describe, expect, it } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { App } from "./App";
import { useAppStore } from "./store";
import { installFakeBackend, type FakeBackend } from "./test/fakeBackend";

let fake: FakeBackend;

beforeEach(() => {
  useAppStore.setState({ app: null, error: null, openViews: [], activeView: null });
  fake = installFakeBackend();
});

describe("Sample projects (ADR-093)", () => {
  it("the start screen opens the Modern House or the basic sample", async () => {
    render(<App />);
    await userEvent.click(screen.getByRole("button", { name: "Sample: Modern House" }));
    await waitFor(() =>
      expect(fake.calls.find((c) => c.cmd === "project_sample")?.args).toMatchObject({
        kind: "modern",
      }),
    );
    useAppStore.setState({ app: null });
    await userEvent.click(await screen.findByRole("button", { name: "Sample: Basic House" }));
    await waitFor(() =>
      expect(fake.calls.filter((c) => c.cmd === "project_sample").at(-1)?.args).toMatchObject({
        kind: "basic",
      }),
    );
  });
});
