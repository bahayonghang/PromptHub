import { useTranslation } from "react-i18next";
import { Modal } from "./Modal";
import { Button } from "./Button";

export interface ConfirmDialogProps {
  open: boolean;
  title: string;
  /** Body copy explaining the consequence of confirming. */
  message: string;
  /** Defaults to the shared `common.confirm` label. */
  confirmLabel?: string;
  cancelLabel?: string;
  /** Renders the confirm action as destructive. */
  destructive?: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}

/**
 * Themed replacement for `window.confirm` (design plan §5).
 *
 * The native dialog is drawn by the OS, ignores the app's theme entirely, and
 * inside a Tauri window looks like a bug. This reuses `Modal`, so it inherits
 * the existing focus trap, Escape handling, focus restoration, and the `inert`
 * treatment of `#app-content`.
 */
export function ConfirmDialog({
  open,
  title,
  message,
  confirmLabel,
  cancelLabel,
  destructive = false,
  onConfirm,
  onCancel,
}: ConfirmDialogProps) {
  const { t } = useTranslation();

  return (
    <Modal open={open} title={title} onClose={onCancel} className="max-h-none w-full max-w-md">
      <div className="p-5">
        <h2 className="text-title text-foreground">{title}</h2>
        <p className="mt-2 text-body text-muted-foreground">{message}</p>
        <div className="mt-5 flex flex-col gap-2 sm:flex-row sm:justify-end">
          <Button variant="ghost" onClick={onCancel}>
            {cancelLabel ?? t("common.cancel")}
          </Button>
          <Button variant={destructive ? "danger" : "primary"} onClick={onConfirm}>
            {confirmLabel ?? t("common.confirm")}
          </Button>
        </div>
      </div>
    </Modal>
  );
}
