import { useEffect } from "react";
import { isCloseAction } from "../types";
import { useSystemStore } from "../systemStore";
import { WindowBehaviorPanel } from "./WindowBehaviorPanel";
import { ShortcutsPanel } from "./ShortcutsPanel";
import { NotificationsPanel } from "./NotificationsPanel";
import { UpdaterPanel } from "./UpdaterPanel";
import { RuntimeInfoPanel } from "./RuntimeInfoPanel";

interface SystemPanelProps {
  /** The persisted launch-at-startup preference, used to seed the toggle (Req 20.5). */
  launchAtStartup?: boolean | null;
  /** The persisted close action, used to seed the three-way control (Req 20.4). */
  closeAction?: string | null;
}

/**
 * The system settings section (Req 20, 24). Composes the window-behavior
 * (close action + auto-launch), keyboard-shortcut, notification, updater, and
 * runtime-info / cache panels. On mount it loads version / platform / runtime
 * paths / cache size through the system store (Req 3.1) and seeds the
 * auto-launch toggle and close action from persisted settings so the controls
 * reflect saved state before the user changes them (Req 20.4, 20.5).
 */
export function SystemPanel({ launchAtStartup, closeAction }: SystemPanelProps) {
  const loadInfo = useSystemStore((s) => s.loadInfo);
  const setAutoLaunchLocal = useSystemStore((s) => s.setAutoLaunchLocal);
  const setCloseActionLocal = useSystemStore((s) => s.setCloseActionLocal);
  const error = useSystemStore((s) => s.error);

  useEffect(() => {
    void loadInfo();
  }, [loadInfo]);

  useEffect(() => {
    if (launchAtStartup != null) setAutoLaunchLocal(launchAtStartup);
  }, [launchAtStartup, setAutoLaunchLocal]);

  useEffect(() => {
    if (isCloseAction(closeAction)) setCloseActionLocal(closeAction);
  }, [closeAction, setCloseActionLocal]);

  return (
    <div className="flex flex-col gap-8">
      {error && (
        <p className="text-sm text-destructive" role="alert">
          {error}
        </p>
      )}
      <WindowBehaviorPanel />
      <ShortcutsPanel />
      <NotificationsPanel />
      <UpdaterPanel />
      <RuntimeInfoPanel />
    </div>
  );
}
