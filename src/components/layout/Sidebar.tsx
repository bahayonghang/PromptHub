import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import {
  MoonIcon,
  PanelLeftCloseIcon,
  PanelLeftOpenIcon,
  SearchIcon,
  SunIcon,
} from "lucide-react";
import { useAppStore, type AppView } from "../../store/appStore";
import { useSettingsStore } from "../../features/settings/settingsStore";
import { FOOTER_NAV, type NavEntry } from "./navigation";
import { formatBinding, SHORTCUT_BINDINGS } from "../../shortcuts/bindings";
import { platformModifier } from "../../shortcuts/platform";

interface NavButtonProps {
  entry: NavEntry;
  active: boolean;
  collapsed: boolean;
  onSelect: (view: AppView) => void;
}

/** A single sidebar navigation button. Collapses to an icon-only rail. */
function NavButton({ entry, active, collapsed, onSelect }: NavButtonProps) {
  const { t } = useTranslation();
  const label = t(entry.labelKey);
  const Icon = entry.icon;

  return (
    <button
      type="button"
      onClick={() => onSelect(entry.view)}
      title={label}
      aria-label={label}
      aria-current={active ? "page" : undefined}
      className={`flex items-center rounded-lg transition-colors duration-fast ease-out ${
        collapsed ? "h-10 w-10 justify-center" : "w-full justify-start gap-3 px-3 py-2"
      } ${
        active
          ? "bg-primary text-primary-foreground shadow-sm"
          : "text-sidebar-foreground/70 hover:bg-sidebar-accent hover:text-sidebar-foreground"
      }`}
    >
      <Icon className="h-5 w-5 shrink-0" aria-hidden="true" />
      {!collapsed && <span className="min-w-0 flex-1 truncate text-left text-body">{label}</span>}
    </button>
  );
}

function paintedModeIsDark(): boolean {
  return typeof document !== "undefined" && document.documentElement.classList.contains("dark");
}

/**
 * The application navigation sidebar (Req 22.3). Holds the product mark, a
 * library slot, collapse control, theme toggle, and settings. All text comes
 * from i18n keys (Req 21.3) and every icon is from the Lucide set (Req 22.4).
 */
export function Sidebar({
  children,
  onOpenCommandPalette,
}: {
  children?: ReactNode;
  onOpenCommandPalette?: () => void;
}) {
  const { t } = useTranslation();
  const activeView = useAppStore((state) => state.activeView);
  const setActiveView = useAppStore((state) => state.setActiveView);
  const collapsed = useAppStore((state) => state.sidebarCollapsed);
  const toggleSidebar = useAppStore((state) => state.toggleSidebar);
  const theme = useSettingsStore((state) => state.settings?.theme);
  const setPreference = useSettingsStore((state) => state.setPreference);

  const toggleLabel = collapsed ? t("shell.expandSidebar") : t("shell.collapseSidebar");
  const isDark = theme === "system" ? paintedModeIsDark() : theme !== "light";
  const themeLabel = isDark
    ? t("promptsView.library.themeToLight")
    : t("promptsView.library.themeToDark");

  const toggleTheme = () => {
    const next = isDark ? "light" : "dark";
    void setPreference("theme", next);
  };

  return (
    <aside
      className={`flex shrink-0 flex-col border-r border-sidebar-border bg-sidebar text-sidebar-foreground transition-[width] duration-base ease-spring ${
        collapsed ? "w-16" : "w-[264px]"
      }`}
    >
      <div
        className={`flex h-14 shrink-0 items-center border-b border-sidebar-border ${
          collapsed ? "justify-center px-2" : "justify-between gap-2 px-4"
        }`}
      >
        <div
          aria-hidden={collapsed}
          className={`min-w-0 overflow-hidden transition-opacity duration-fast ease-out ${
            collapsed ? "pointer-events-none w-0 opacity-0" : "opacity-100"
          }`}
        >
          <p className="truncate text-body font-semibold">{t("app.name")}</p>
          <p className="truncate font-mono text-meta text-muted-foreground-subtle">
            {t("promptsView.library.productVersion")}
          </p>
        </div>
        <button
          type="button"
          onClick={toggleSidebar}
          title={toggleLabel}
          aria-label={toggleLabel}
          className="flex h-control-md w-control-md items-center justify-center rounded-lg text-sidebar-foreground/70 transition-colors duration-fast ease-out hover:bg-sidebar-accent hover:text-sidebar-foreground"
        >
          {collapsed ? (
            <PanelLeftOpenIcon className="h-5 w-5" aria-hidden="true" />
          ) : (
            <PanelLeftCloseIcon className="h-5 w-5" aria-hidden="true" />
          )}
        </button>
      </div>

      <div className={`shrink-0 p-2 ${collapsed ? "flex justify-center" : ""}`}>
        <button
          type="button"
          onClick={onOpenCommandPalette}
          title={t("promptsView.library.commandPalette")}
          aria-label={t("promptsView.library.commandPalette")}
          className={`flex items-center rounded-lg border border-border text-muted-foreground hover:bg-sidebar-accent hover:text-sidebar-foreground ${
            collapsed ? "h-10 w-10 justify-center" : "h-9 w-full justify-between px-3 text-body"
          }`}
        >
          <span className="flex items-center gap-2">
            <SearchIcon className="h-4 w-4 shrink-0" aria-hidden="true" />
            {!collapsed && t("promptsView.library.commandPalette")}
          </span>
          {!collapsed && (
            <kbd className="font-mono text-meta text-muted-foreground-subtle">
              {formatBinding(
                SHORTCUT_BINDINGS.find((item) => item.id === "togglePalette")!,
                platformModifier().symbol,
              )}
            </kbd>
          )}
        </button>
      </div>

      {children}

      <div className="mt-auto flex flex-col gap-1 border-t border-sidebar-border p-2">
        <button
          type="button"
          onClick={toggleTheme}
          title={themeLabel}
          aria-label={themeLabel}
          className={`flex items-center rounded-lg text-sidebar-foreground/70 transition-colors duration-fast ease-out hover:bg-sidebar-accent hover:text-sidebar-foreground ${
            collapsed ? "h-10 w-10 justify-center" : "h-9 w-full justify-start gap-3 px-3"
          }`}
        >
          {isDark ? (
            <SunIcon className="h-5 w-5 shrink-0" aria-hidden="true" />
          ) : (
            <MoonIcon className="h-5 w-5 shrink-0" aria-hidden="true" />
          )}
          {!collapsed && <span className="text-body">{themeLabel}</span>}
        </button>
        {FOOTER_NAV.map((entry) => (
          <NavButton
            key={entry.view}
            entry={entry}
            active={activeView === entry.view}
            collapsed={collapsed}
            onSelect={setActiveView}
          />
        ))}
      </div>
    </aside>
  );
}
