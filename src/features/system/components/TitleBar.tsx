import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { useSystemStore } from "../systemStore";
import { WindowControls } from "./WindowControls";

/**
 * The custom application title bar (Req 20.1, 20.2). Provides a draggable region
 * (via the Tauri `data-tauri-drag-region` attribute) carrying the app name and,
 * on the trailing edge, the native window controls. The bar wires up the
 * Window_Manager / Updater event subscriptions on mount through the system store
 * (Req 3.1, 3.6), so live fullscreen / visibility / close-requested / shortcut /
 * updater events stay reflected in app state while the shell is mounted.
 */
export function TitleBar() {
  const { t } = useTranslation();
  const initialize = useSystemStore((s) => s.initialize);

  useEffect(() => {
    // Subscribe to Window_Manager + Updater events; detach on unmount (Req 3.6).
    const unsubscribe = initialize();
    return unsubscribe;
  }, [initialize]);

  return (
    <div
      data-tauri-drag-region
      className="flex h-9 shrink-0 items-center justify-between border-b border-border bg-background px-3 select-none"
    >
      <span
        data-tauri-drag-region
        className="truncate text-xs font-medium text-muted-foreground"
      >
        {t("app.name")}
      </span>
      <WindowControls />
    </div>
  );
}
