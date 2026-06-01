// @vitest-environment jsdom
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import "../../../runtime/i18n";
import { SpecimenCard } from "./SpecimenCard";
import { DEFAULT_APPEARANCE, FONT_STACK, type Appearance } from "../../../appearance";

afterEach(cleanup);


describe("SpecimenCard (Req 8)", () => {
  it("renders display text, body text, and an accent control bound to the token variables", () => {
    const appearance: Appearance = {
      ...DEFAULT_APPEARANCE,
      displayFont: "Inter",
      bodyFont: "JetBrains Mono",
    };
    render(<SpecimenCard appearance={appearance} />);

    expect(screen.getByTestId("specimen-display").textContent?.length).toBeGreaterThan(0);
    expect(screen.getByTestId("specimen-body").textContent?.length).toBeGreaterThan(0);

    const action = screen.getByTestId("specimen-action");
    expect(action.className).toContain("bg-primary");

    // The active appearance is scoped to the card's own container.
    const card = screen.getByTestId("specimen-card");
    expect(card.style.getPropertyValue("--font-display")).toBe(FONT_STACK.Inter);
    expect(card.style.getPropertyValue("--font-body")).toBe(FONT_STACK["JetBrains Mono"]);
    expect(card.style.getPropertyValue("--primary").length).toBeGreaterThan(0);
  });

  it("updates the bound token variables when the appearance prop changes", () => {
    const { rerender } = render(
      <SpecimenCard appearance={{ ...DEFAULT_APPEARANCE, displayFont: "Inter" }} />,
    );
    expect(screen.getByTestId("specimen-card").style.getPropertyValue("--font-display")).toBe(
      FONT_STACK.Inter,
    );

    rerender(<SpecimenCard appearance={{ ...DEFAULT_APPEARANCE, displayFont: "Space Grotesk" }} />);
    expect(screen.getByTestId("specimen-card").style.getPropertyValue("--font-display")).toBe(
      FONT_STACK["Space Grotesk"],
    );
  });
});
