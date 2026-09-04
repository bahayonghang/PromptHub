// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import closeDialogSrc from "./CloseDialog.tsx?raw";
import { CloseDialog } from "./CloseDialog";
import { useSystemStore } from "../systemStore";
import i18n, { ensureBundle } from "../../../runtime/i18n";

const initialSystem = useSystemStore.getState();

afterEach(() => {
  cleanup();
  useSystemStore.setState(initialSystem, true);
});

describe("CloseDialog (Req 20.4)", () => {
  it("does not import @tauri-apps/api", () => {
    expect(closeDialogSrc).not.toContain("@tauri-apps/api");
  });

  it("renders nothing until a close is requested", async () => {
    await ensureBundle("en");
    await i18n.changeLanguage("en");
    useSystemStore.setState({ closeDialogOpen: false });
    const { container } = render(<CloseDialog />);
    expect(container.firstChild).toBeNull();
  });

  it("offers keep running, minimize to tray, and exit", async () => {
    await ensureBundle("en");
    await i18n.changeLanguage("en");
    const dismiss = vi.fn();
    const hideToTray = vi.fn(async () => undefined);
    const confirmClose = vi.fn(async () => undefined);
    useSystemStore.setState({
      closeDialogOpen: true,
      dismissCloseDialog: dismiss,
      hideToTray,
      confirmClose,
    });
    render(<CloseDialog />);

    fireEvent.click(screen.getByRole("button", { name: "Keep running" }));
    expect(dismiss).toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: "Minimize to tray" }));
    expect(hideToTray).toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: "Exit" }));
    expect(confirmClose).toHaveBeenCalled();
  });
});
