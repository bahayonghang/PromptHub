// @vitest-environment jsdom
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import "../../../runtime/i18n";
import { SpecimenCard } from "./SpecimenCard";

afterEach(cleanup);

describe("SpecimenCard", () => {
  it("renders one framed multilingual interface specimen", () => {
    render(<SpecimenCard />);

    expect(screen.getByTestId("specimen-display").textContent?.length).toBeGreaterThan(0);
    expect(screen.getByTestId("specimen-body").textContent?.length).toBeGreaterThan(0);
    expect(screen.getByText(/简体中文/)).toBeTruthy();
    expect(screen.getByTestId("specimen-action").className).toContain("bg-primary");
    expect(screen.getByTestId("specimen-card").className).toContain("border-border");
  });
});
