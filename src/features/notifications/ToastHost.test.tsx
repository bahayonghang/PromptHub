// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import i18n, { ensureBundle } from "../../runtime/i18n";
import { ToastHost } from "./ToastHost";
import { useToastStore } from "./toastStore";

afterEach(() => {
  cleanup();
  for (const toast of useToastStore.getState().toasts) {
    useToastStore.getState().dismiss(toast.id);
  }
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

  it("paints a success toast with the copied message", async () => {
    await ensureBundle("en");
    await i18n.changeLanguage("en");
    render(<ToastHost />);
    act(() => {
      useToastStore.getState().push({
        message: "Copied Steelman",
        tone: "success",
        replaceGroup: "prompt-copy",
      });
    });
    expect(screen.getByRole("status").textContent).toContain("Copied Steelman");
    const item = document.querySelector(".toast-item");
    expect(item?.querySelector(".bg-success")).toBeTruthy();
    expect(item?.querySelector(".text-success")).toBeTruthy();
  });

  it("replaceGroup replaces an existing toast in that group", () => {
    act(() => {
      useToastStore.getState().push({
        message: "Copied A",
        tone: "success",
        replaceGroup: "prompt-copy",
      });
      useToastStore.getState().push({
        message: "Copied B",
        tone: "success",
        replaceGroup: "prompt-copy",
      });
    });
    const copyToasts = useToastStore
      .getState()
      .toasts.filter((toast) => toast.replaceGroup === "prompt-copy");
    expect(copyToasts).toHaveLength(1);
    expect(copyToasts[0]?.message).toBe("Copied B");
  });

  it("keeps toasts from different groups", () => {
    act(() => {
      useToastStore.getState().push({
        message: "Copied",
        tone: "success",
        replaceGroup: "prompt-copy",
      });
      useToastStore.getState().push({
        message: "Saved",
        tone: "success",
        replaceGroup: "prompt-save",
      });
    });
    expect(useToastStore.getState().toasts.map((toast) => toast.message)).toEqual([
      "Copied",
      "Saved",
    ]);
  });

  it("does not drop ungrouped save toasts", () => {
    act(() => {
      useToastStore.getState().push({
        message: "Saved",
        tone: "success",
      });
      useToastStore.getState().push({
        message: "Copied",
        tone: "success",
        replaceGroup: "prompt-copy",
      });
    });
    expect(useToastStore.getState().toasts.map((toast) => toast.message)).toEqual([
      "Saved",
      "Copied",
    ]);
  });
});
