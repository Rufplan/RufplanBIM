import { describe, expect, it } from "vitest";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { TitleBlockDialog } from "./components/TitleBlockDialog";
import { installFakeBackend } from "./test/fakeBackend";

// Double-clicking a sheet's title block opens its fields (ADR-111).
describe("title block dialog", () => {
  it("edits the firm, the license line and who drew the sheet", async () => {
    const fake = installFakeBackend();
    let closed = false;
    render(<TitleBlockDialog sheet="s1" onClose={() => (closed = true)} />);
    const dialog = await screen.findByRole("dialog", { name: "Title Block" });
    const name = await within(dialog).findByLabelText("Project Name");
    expect((name as HTMLInputElement).value).toBe("Modern House");
    await userEvent.type(within(dialog).getByLabelText("Architect Firm"), "Kerr Architects");
    await userEvent.type(within(dialog).getByLabelText("License No."), "C-12345");
    await userEvent.type(within(dialog).getByLabelText("Drawn By"), "RK");
    await userEvent.click(within(dialog).getByRole("button", { name: "OK" }));
    expect(fake.titleBlock).toMatchObject({
      firm: "Kerr Architects",
      license: { number: "C-12345" },
      signed: { drawn: "RK" },
      projectName: "Modern House",
    });
    expect(closed).toBe(true);
  });
});
