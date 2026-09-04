// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from "vitest";
import { useState } from "react";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { useConfirm } from "./useConfirm";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => (key === "common.confirm" ? "Confirm" : "Cancel"),
  }),
}));

// No global setupFiles in this project, so RTL's auto-cleanup is not installed.
afterEach(cleanup);

/**
 * Harness mirroring a real call site: an action guarded by `await confirm(...)`
 * exactly where `if (window.confirm(...))` used to sit.
 */
function Harness({ onAct }: { onAct: () => void }) {
  const { confirm, confirmDialog } = useConfirm();
  const [settled, setSettled] = useState<string>("pending");

  return (
    <div>
      <button
        type="button"
        onClick={() => {
          void (async () => {
            const ok = await confirm({
              title: "Delete prompt",
              message: "This cannot be undone.",
              destructive: true,
            });
            setSettled(String(ok));
            if (ok) onAct();
          })();
        }}
      >
        Delete
      </button>
      <span data-testid="settled">{settled}</span>
      {confirmDialog}
    </div>
  );
}

describe("useConfirm", () => {
  it("does not render a dialog until a confirmation is requested", () => {
    render(<Harness onAct={vi.fn()} />);
    expect(screen.queryByText("This cannot be undone.")).toBeNull();
  });

  it("runs the guarded action only after the user confirms", async () => {
    const onAct = vi.fn();
    render(<Harness onAct={onAct} />);

    fireEvent.click(screen.getByRole("button", { name: "Delete" }));
    expect(await screen.findByText("This cannot be undone.")).toBeTruthy();
    // Still gated at this point: the promise has not resolved.
    expect(onAct).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: "Confirm" }));

    await waitFor(() => expect(onAct).toHaveBeenCalledTimes(1));
    expect(screen.getByTestId("settled").textContent).toBe("true");
    // Dialog closes once resolved.
    await waitFor(() =>
      expect(screen.queryByText("This cannot be undone.")).toBeNull(),
    );
  });

  it("resolves false and skips the action when cancelled", async () => {
    const onAct = vi.fn();
    render(<Harness onAct={onAct} />);

    fireEvent.click(screen.getByRole("button", { name: "Delete" }));
    expect(await screen.findByText("This cannot be undone.")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));

    await waitFor(() =>
      expect(screen.getByTestId("settled").textContent).toBe("false"),
    );
    expect(onAct).not.toHaveBeenCalled();
  });

  it("resolves false when the dialog is dismissed with Escape", async () => {
    const onAct = vi.fn();
    render(<Harness onAct={onAct} />);

    fireEvent.click(screen.getByRole("button", { name: "Delete" }));
    expect(await screen.findByText("This cannot be undone.")).toBeTruthy();
    // Modal traps Escape via an onKeyDown handler on the dialog itself, so the
    // event has to originate inside it rather than on document.
    fireEvent.keyDown(screen.getByRole("button", { name: "Cancel" }), {
      key: "Escape",
    });

    await waitFor(() =>
      expect(screen.getByTestId("settled").textContent).toBe("false"),
    );
    expect(onAct).not.toHaveBeenCalled();
  });

  it("never strands a promise when a second request interrupts the first", async () => {
    // Two confirmations racing would previously leave the first `await`
    // pending forever, silently wedging whatever action it guarded.
    const settled: boolean[] = [];
    function Racer() {
      const { confirm, confirmDialog } = useConfirm();
      return (
        <div>
          <button
            type="button"
            onClick={() => {
              void confirm({ title: "A", message: "first" }).then((v) =>
                settled.push(v),
              );
            }}
          >
            ask
          </button>
          {confirmDialog}
        </div>
      );
    }
    render(<Racer />);
    const ask = screen.getByRole("button", { name: "ask" });
    fireEvent.click(ask);
    fireEvent.click(ask);

    await waitFor(() => expect(settled).toEqual([false]));
  });
});
