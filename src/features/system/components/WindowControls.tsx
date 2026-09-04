import { useTranslation } from "react-i18next";
import {
  CopyIcon,
  MaximizeIcon,
  MinimizeIcon,
  MinusIcon,
  ShrinkIcon,
  XIcon,
} from "lucide-react";
import { useSystemStore } from "../systemStore";

/**
 * Native window controls for the custom title bar (Req 20.1, 20.2): minimize,
 * maximize/restore, fullscreen toggle, and close. Each routes through the system
 * store, which calls the Window_Manager via the Runtime_Bridge (Req 3.1). When
 * window controls are unavailable in the current runtime (Req 3.7) the group
 * renders nothing so the layout degrades gracefully. Every icon is from the
 * Lucide set (Req 22.4) and every label resolves through i18n (Req 21.3).
 */
export function WindowControls() {
  const { t } = useTranslation();

  const isMaximized = useSystemStore((s) => s.isMaximized);
  const isFullscreen = useSystemStore((s) => s.isFullscreen);
  const unavailable = useSystemStore((s) => s.windowControlsUnavailable);

  const minimize = useSystemStore((s) => s.minimize);
  const toggleMaximize = useSystemStore((s) => s.toggleMaximize);
  const toggleFullscreen = useSystemStore((s) => s.toggleFullscreen);
  const close = useSystemStore((s) => s.close);

  if (unavailable) return null;

  const buttonClass =
    "flex h-control-md w-control-md items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-accent hover:text-foreground";

  const maximizeLabel = isMaximized
    ? t("systemView.window.restore")
    : t("systemView.window.maximize");
  const fullscreenLabel = isFullscreen
    ? t("systemView.window.exitFullscreen")
    : t("systemView.window.enterFullscreen");

  return (
    <div className="flex items-center gap-1">
      <button
        type="button"
        onClick={() => void toggleFullscreen()}
        title={fullscreenLabel}
        aria-label={fullscreenLabel}
        className={buttonClass}
      >
        {isFullscreen ? (
          <ShrinkIcon className="h-4 w-4" aria-hidden="true" />
        ) : (
          <MaximizeIcon className="h-4 w-4" aria-hidden="true" />
        )}
      </button>
      <button
        type="button"
        onClick={() => void minimize()}
        title={t("systemView.window.minimize")}
        aria-label={t("systemView.window.minimize")}
        className={buttonClass}
      >
        <MinusIcon className="h-4 w-4" aria-hidden="true" />
      </button>
      <button
        type="button"
        onClick={() => void toggleMaximize()}
        title={maximizeLabel}
        aria-label={maximizeLabel}
        className={buttonClass}
      >
        {isMaximized ? (
          <CopyIcon className="h-4 w-4" aria-hidden="true" />
        ) : (
          <MinimizeIcon className="h-4 w-4" aria-hidden="true" />
        )}
      </button>
      <button
        type="button"
        onClick={() => void close()}
        title={t("systemView.window.close")}
        aria-label={t("systemView.window.close")}
        className="flex h-control-md w-control-md items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-destructive hover:text-destructive-foreground"
      >
        <XIcon className="h-4 w-4" aria-hidden="true" />
      </button>
    </div>
  );
}
