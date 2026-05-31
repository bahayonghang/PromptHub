import { useTranslation } from "react-i18next";
import { PanelLeftCloseIcon, PanelLeftOpenIcon } from "lucide-react";
import { useAppStore, type AppView } from "../../store/appStore";
import { FOOTER_NAV, PRIMARY_NAV, type NavEntry } from "./navigation";

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
      className={`flex items-center rounded-lg transition-colors ${
        collapsed ? "h-10 w-10 justify-center" : "w-full justify-start gap-3 px-3 py-2"
      } ${
        active
          ? "bg-primary text-primary-foreground shadow-sm"
          : "text-sidebar-foreground/70 hover:bg-sidebar-accent hover:text-sidebar-foreground"
      }`}
    >
      <Icon className="h-5 w-5 shrink-0" aria-hidden="true" />
      {!collapsed && <span className="min-w-0 flex-1 truncate text-left text-sm">{label}</span>}
    </button>
  );
}

/**
 * The application navigation sidebar (Req 22.3). Lists the major views, lets the
 * user switch the active view, and can collapse to an icon rail. Settings is
 * pinned to the footer. All text comes from i18n keys (Req 21.3) and every icon
 * is from the Lucide set (Req 22.4).
 */
export function Sidebar() {
  const { t } = useTranslation();
  const activeView = useAppStore((state) => state.activeView);
  const setActiveView = useAppStore((state) => state.setActiveView);
  const collapsed = useAppStore((state) => state.sidebarCollapsed);
  const toggleSidebar = useAppStore((state) => state.toggleSidebar);

  const toggleLabel = collapsed ? t("shell.expandSidebar") : t("shell.collapseSidebar");

  return (
    <aside
      className={`flex shrink-0 flex-col border-r border-sidebar-border bg-sidebar text-sidebar-foreground transition-[width] duration-200 ${
        collapsed ? "w-16" : "w-60"
      }`}
    >
      <div
        className={`flex h-14 shrink-0 items-center border-b border-sidebar-border ${
          collapsed ? "justify-center px-2" : "justify-between px-4"
        }`}
      >
        {!collapsed && (
          <span className="truncate text-sm font-semibold">{t("app.name")}</span>
        )}
        <button
          type="button"
          onClick={toggleSidebar}
          title={toggleLabel}
          aria-label={toggleLabel}
          className="flex h-8 w-8 items-center justify-center rounded-lg text-sidebar-foreground/70 transition-colors hover:bg-sidebar-accent hover:text-sidebar-foreground"
        >
          {collapsed ? (
            <PanelLeftOpenIcon className="h-5 w-5" aria-hidden="true" />
          ) : (
            <PanelLeftCloseIcon className="h-5 w-5" aria-hidden="true" />
          )}
        </button>
      </div>

      <nav className="flex flex-1 flex-col gap-1 overflow-y-auto p-2" aria-label={t("shell.primaryNav")}>
        {PRIMARY_NAV.map((entry) => (
          <NavButton
            key={entry.view}
            entry={entry}
            active={activeView === entry.view}
            collapsed={collapsed}
            onSelect={setActiveView}
          />
        ))}
      </nav>

      <div className="mt-auto flex flex-col gap-1 border-t border-sidebar-border p-2">
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
