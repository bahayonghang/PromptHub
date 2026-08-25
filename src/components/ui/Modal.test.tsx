// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { Modal } from "./Modal";
import { CloseDialog } from "../../features/system/components/CloseDialog";
import { useSystemStore } from "../../features/system/systemStore";
import i18n, { ensureBundle } from "../../runtime/i18n";

const initialSystem = useSystemStore.getState();

afterEach(() => {
  cleanup();
  useSystemStore.setState(initialSystem, true);
});

describe("Modal", () => {
  it("moves focus in, traps tab, and restores on close", async () => {
    await ensureBundle("en");
    await i18n.changeLanguage("en");
    const outside = document.createElement("button");
    outside.textContent = "Outside";
    document.body.appendChild(outside);
    outside.focus();
    const onClose = vi.fn();
    const { rerender } = render(
      <Modal open title="Dialog" onClose={onClose}>
        <button type="button">First</button>
        <button type="button">Last</button>
      </Modal>,
    );
    const first = await screen.findByRole("button", { name: "First" });
    expect(document.activeElement).toBe(first);

    fireEvent.keyDown(first, { key: "Tab", shiftKey: true });
    expect(document.activeElement).toBe(
      screen.getByRole("button", { name: "Last" }),
    );

    fireEvent.keyDown(screen.getByRole("button", { name: "Last" }), {
      key: "Tab",
    });
    expect(document.activeElement).toBe(first);

    rerender(
      <Modal open={false} title="Dialog" onClose={onClose}>
        <button type="button">First</button>
      </Modal>,
    );
    expect(document.activeElement).toBe(outside);
    outside.remove();
  });

  it("Escape closes only the top stacked modal", async () => {
    const closeBottom = vi.fn();
    const closeTop = vi.fn();
    render(
      <>
        <Modal open title="Bottom" onClose={closeBottom}>
          <button type="button">Bottom button</button>
        </Modal>
        <Modal open title="Top" onClose={closeTop}>
          <button type="button">Top button</button>
        </Modal>
      </>,
    );
    fireEvent.keyDown(screen.getByRole("button", { name: "Top button" }), {
      key: "Escape",
    });
    expect(closeTop).toHaveBeenCalledOnce();
    expect(closeBottom).not.toHaveBeenCalled();
  });

  it("keeps CloseDialog focusable while the overlay is open", async () => {
    await ensureBundle("en");
    await i18n.changeLanguage("en");
    useSystemStore.setState({
      closeDialogOpen: true,
      confirmClose: vi.fn(),
      dismissCloseDialog: vi.fn(),
    });
    render(
      <div>
        <div id="app-content">
          <button type="button">App</button>
        </div>
        <CloseDialog />
        <Modal open title="Overlay" onClose={vi.fn()}>
          <button type="button">Inside</button>
        </Modal>
      </div>,
    );
    const region = document.getElementById("app-content");
    expect(region?.getAttribute("aria-hidden")).toBe("true");
    const cancel = screen.getByRole("button", { name: "Keep running" });
    const confirm = screen.getByRole("button", { name: "Exit" });
    expect(cancel).toBeTruthy();
    expect(confirm).toBeTruthy();
    fireEvent.click(confirm);
    expect(useSystemStore.getState().confirmClose).toHaveBeenCalled();
  });
});
