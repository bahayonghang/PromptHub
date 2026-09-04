import { useTranslation } from "react-i18next";
import { CLOSE_ACTIONS, type CloseAction } from "../types";
import { useSystemStore } from "../systemStore";

/**
 * Window-behavior preferences (Req 20.4, 20.5): the close-window action
 * (ask / minimize / exit) and the launch-at-startup toggle. Both route through
 * the system store to the Window_Manager via the Runtime_Bridge (Req 3.1). The
 * close action and auto-launch state are tracked optimistically once the backend
 * accepts the change. All text resolves through i18n (Req 21.3).
 */
export function WindowBehaviorPanel() {
  const { t } = useTranslation();

  const closeAction = useSystemStore((s) => s.closeAction);
  const autoLaunch = useSystemStore((s) => s.autoLaunch);
  const setCloseAction = useSystemStore((s) => s.setCloseAction);
  const setAutoLaunch = useSystemStore((s) => s.setAutoLaunch);

  const labelClass = "text-body font-medium text-foreground";
  const hintClass = "text-label text-muted-foreground";

  return (
    <div className="flex flex-col gap-6">
      {/* Close action (Req 20.4) */}
      <section className="flex flex-col gap-3">
        <div className="flex flex-col gap-0.5">
          <h3 className={labelClass}>{t("systemView.window.closeActionTitle")}</h3>
          <p className={hintClass}>{t("systemView.window.closeActionHint")}</p>
        </div>
        <div
          className="flex flex-wrap gap-2"
          role="group"
          aria-label={t("systemView.window.closeActionTitle")}
        >
          {CLOSE_ACTIONS.map((action: CloseAction) => (
            <button
              key={action}
              type="button"
              aria-pressed={closeAction === action}
              onClick={() => void setCloseAction(action)}
              className={`rounded-md border px-4 py-2 text-body transition-colors ${
                closeAction === action
                  ? "border-primary bg-primary/15 text-foreground"
                  : "border-input text-muted-foreground hover:bg-accent hover:text-foreground"
              }`}
            >
              {t(`systemView.window.closeAction.${action}`)}
            </button>
          ))}
        </div>
      </section>

      {/* Auto-launch (Req 20.5) */}
      <section className="flex items-center justify-between gap-4">
        <div className="flex flex-col gap-0.5">
          <h3 className={labelClass}>{t("systemView.window.autoLaunch")}</h3>
          <p className={hintClass}>{t("systemView.window.autoLaunchHint")}</p>
        </div>
        <button
          type="button"
          role="switch"
          aria-checked={autoLaunch}
          aria-label={t("systemView.window.autoLaunch")}
          onClick={() => void setAutoLaunch(!autoLaunch)}
          className={`relative h-6 w-11 shrink-0 rounded-full transition-colors ${
            autoLaunch ? "bg-primary" : "bg-input"
          }`}
        >
          <span
            className={`absolute top-0.5 left-0.5 h-5 w-5 rounded-full bg-white shadow transition-transform ${
              autoLaunch ? "translate-x-5" : "translate-x-0"
            }`}
          />
        </button>
      </section>
    </div>
  );
}
