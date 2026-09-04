import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { useAppStore } from "./store/appStore";
import { AppShell } from "./components/layout/AppShell";

function App() {
  const { t } = useTranslation();
  const initError = useAppStore((state) => state.initError);
  const initialize = useAppStore((state) => state.initialize);

  useEffect(() => {
    // Bootstrap app-level state (readiness + fatal init failures) through the
    // Runtime_Bridge (Req 3.1 / 23.3). The store owns all backend access.
    let cancelled = false;
    let unsubscribe: (() => void) | undefined;
    void initialize().then((fn) => {
      if (cancelled) {
        fn();
      } else {
        unsubscribe = fn;
      }
    });

    return () => {
      cancelled = true;
      unsubscribe?.();
    };
  }, [initialize]);

  if (initError) {
    return (
      <main className="flex h-full flex-col items-center justify-center gap-4 bg-background p-8 text-foreground">
        <h1 className="text-2xl font-semibold text-destructive">{t("shell.startupFailedTitle")}</h1>
        <p className="max-w-lg text-center text-sm text-muted-foreground">{initError}</p>
      </main>
    );
  }

  return <AppShell />;
}

export default App;
