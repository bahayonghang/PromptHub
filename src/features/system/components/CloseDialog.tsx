import { useTranslation } from "react-i18next";
import { useSystemStore } from "../systemStore";

/**
 * The close-confirmation dialog shown when the close action is `ask` and a
 * window-close is attempted (Req 20.4). The Window_Manager emits
 * `window:close-requested` and keeps the application running; this dialog lets
 * the user keep running, hide to the tray, or exit via `window.quit`. It
 * renders nothing until a close is requested.
 */
export function CloseDialog() {
  const { t } = useTranslation();
  const open = useSystemStore((s) => s.closeDialogOpen);
  const dismiss = useSystemStore((s) => s.dismissCloseDialog);
  const hideToTray = useSystemStore((s) => s.hideToTray);
  const confirm = useSystemStore((s) => s.confirmClose);

  if (!open) return null;

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-label={t("systemView.close.title")}
      className="fixed inset-0 z-50 flex items-center justify-center bg-background/70 p-4"
    >
      <div className="w-full max-w-sm rounded-lg border border-border bg-card p-5 text-card-foreground shadow-lg">
        <h2 className="text-title font-semibold">{t("systemView.close.title")}</h2>
        <p className="mt-2 text-body text-muted-foreground">
          {t("systemView.close.message")}
        </p>
        <div className="mt-5 flex flex-wrap justify-end gap-2">
          <button
            type="button"
            onClick={dismiss}
            className="rounded-md border border-input px-4 py-2 text-body text-foreground transition-colors duration-fast ease-out hover:bg-accent"
          >
            {t("systemView.close.cancel")}
          </button>
          <button
            type="button"
            onClick={() => void hideToTray()}
            className="rounded-md border border-input px-4 py-2 text-body text-foreground transition-colors duration-fast ease-out hover:bg-accent"
          >
            {t("systemView.close.minimize")}
          </button>
          <button
            type="button"
            onClick={() => void confirm()}
            className="rounded-md bg-destructive px-4 py-2 text-body text-destructive-foreground transition-colors duration-fast ease-out hover:bg-destructive/90"
          >
            {t("systemView.close.confirm")}
          </button>
        </div>
      </div>
    </div>
  );
}
