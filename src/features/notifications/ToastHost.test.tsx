// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import i18n, { ensureBundle } from "../../runtime/i18n";
import { ToastHost } from "./ToastHost";
import { useToastStore } from "./toastStore";

afterEach(() => {
  cleanup();
  useToastStore.setState({ toasts: [] });
});

describe("ToastHost", () => {
  it("announces a toast and dismisses it from the keyboard", async () => {
    await ensureBundle("en");
    await i18n.changeLanguage("en");
    render(<ToastHost />);
    act(() => {
      useToastStore.getState().push({ message: "Exported", tone: "success" });
    });
    expect(screen.getByRole("status").textContent).toContain("Exported");
    const dismiss = screen.getByRole("button", { name: "Dismiss notification" });
    expect(document.activeElement).not.toBe(dismiss);
    fireEvent.click(dismiss);
    expect(screen.queryByText("Exported")).toBeNull();
  });
});
