import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { TerrainBar } from "./components/TerrainBar";

describe("terrain toolbar (ADR-045)", () => {
  it("toggles earth depth, contours and labels, and sets the interval", async () => {
    const onSolid = vi.fn();
    const onContours = vi.fn();
    const onLabels = vi.fn();
    const onInterval = vi.fn();
    const { rerender } = render(
      <TerrainBar
        solid
        contours
        labels={false}
        interval={304.8}
        onSolid={onSolid}
        onContours={onContours}
        onLabels={onLabels}
        onInterval={onInterval}
      />,
    );
    const bar = screen.getByRole("toolbar", { name: "Terrain" });
    expect(bar).toBeTruthy();
    expect(screen.getByRole("button", { name: "Earth depth" }).getAttribute("aria-pressed")).toBe(
      "true",
    );
    await userEvent.click(screen.getByRole("button", { name: "Earth depth" }));
    expect(onSolid).toHaveBeenCalledWith(false);
    await userEvent.click(screen.getByRole("button", { name: "Contour labels" }));
    expect(onLabels).toHaveBeenCalledWith(true);
    // The interval shows as 1' and changes to 5'.
    const select = screen.getByRole("combobox", { name: "Contour interval" }) as HTMLSelectElement;
    expect(select.selectedOptions[0]!.textContent).toBe("1'");
    await userEvent.selectOptions(select, "1524");
    expect(onInterval).toHaveBeenCalledWith(1524);
    // With contours off, labels and interval wait for them.
    rerender(
      <TerrainBar
        solid
        contours={false}
        labels
        interval={304.8}
        onSolid={onSolid}
        onContours={onContours}
        onLabels={onLabels}
        onInterval={onInterval}
      />,
    );
    expect(
      (screen.getByRole("button", { name: "Contour labels" }) as HTMLButtonElement).disabled,
    ).toBe(true);
    expect(select.disabled).toBe(true);
  });
});
