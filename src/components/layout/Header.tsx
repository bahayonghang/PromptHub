import { useTranslation } from "react-i18next";
import { useAppStore } from "../../store/appStore";
import { NAV_ENTRIES } from "./navigation";

/**
 * The application header bar (Req 22.3). Shows the title of the active view so
 * the user always has a visible heading for the current content region. Text is
 * resolved through i18n (Req 21.3).
 */
export function Header() {
  const { t } = useTranslation();
  const activeView = useAppStore((state) => state.activeView);
  const entry = NAV_ENTRIES.find((nav) => nav.view === activeView);
  const title = entry ? t(entry.labelKey) : t("app.name");

  return (
    <header className="flex h-14 shrink-0 items-center border-b border-border bg-background px-6">
      <h1 className="truncate text-lg font-semibold text-foreground">{title}</h1>
    </header>
  );
}
