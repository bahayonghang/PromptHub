import { useTranslation } from "react-i18next";
import { CLOSE_ACTIONS, type CloseAction } from "../types";
import { useSystemStore } from "../systemStore";
import { SettingRow, Switch } from "../../../components/ui";

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


  return (
    <div className="flex flex-col gap-6">
      {/* Close action (Req 20.4) */}
      <SettingRow
        title={t("systemView.window.closeActionTitle")}
        hint={t("systemView.window.closeActionHint")}
        layout="stacked"
      >
        {({ titleId }) => (
          <div className="flex flex-wrap gap-2" role="group" aria-labelledby={titleId}>
            {CLOSE_ACTIONS.map((action: CloseAction) => (
              <button
                key={action}
                type="button"
                aria-pressed={closeAction === action}
                onClick={() => void setCloseAction(action)}
                className={`rounded-md border px-4 py-2 text-body transition-colors duration-fast ease-out ${
                  closeAction === action
                    ? "border-primary bg-state-selected text-foreground"
                    : "border-input text-muted-foreground hover:bg-accent hover:text-foreground"
                }`}
              >
                {t(`systemView.window.closeAction.${action}`)}
              </button>
            ))}
          </div>
        )}
      </SettingRow>

      {/* Auto-launch (Req 20.5) */}
      <SettingRow
        title={t("systemView.window.autoLaunch")}
        hint={t("systemView.window.autoLaunchHint")}
      >
        {({ titleId, hintId }) => (
          <Switch
            checked={autoLaunch}
            onChange={(next) => void setAutoLaunch(next)}
            labelledBy={titleId}
            describedBy={hintId}
          />
        )}
      </SettingRow>
    </div>
  );
}
