import type { ReactElement } from "react";
import { useAppStore, type AppView } from "../../store/appStore";
import { Sidebar } from "./Sidebar";
import { Header } from "./Header";
import { PromptsView } from "../views/PromptsView";
import { SkillsView } from "../views/SkillsView";
import { SettingsView } from "../views/SettingsView";
import { TitleBar } from "../../features/system/components/TitleBar";
import { CloseDialog } from "../../features/system/components/CloseDialog";

/** Maps each major view to the component rendered in the content region. */
const VIEW_COMPONENTS: Record<AppView, () => ReactElement> = {
  prompts: PromptsView,
  skills: SkillsView,
  settings: SettingsView,
};

/**
 * The application shell layout (Req 22.3): a custom title bar with native window
 * controls (Req 20.1, 20.2) above a navigation sidebar, a header, and a content
 * area that renders the active view. The flex layout with `min-h-0` / `min-w-0`
 * and an overflow-scrolled content region keeps every element visible and
 * operable without clipping or overlap across platforms (Req 23.6). The
 * close-confirmation dialog overlays the shell when an `ask`-action close is
 * requested (Req 20.4).
 */
export function AppShell() {
  const activeView = useAppStore((state) => state.activeView);
  const ActiveView = VIEW_COMPONENTS[activeView];

  return (
    <div className="flex h-full w-full flex-col overflow-hidden bg-background text-foreground">
      <TitleBar />
      <div className="flex min-h-0 flex-1 overflow-hidden">
        <Sidebar />
        <div className="flex min-w-0 flex-1 flex-col">
          <Header />
          <main className="min-h-0 flex-1 overflow-auto">
            <ActiveView />
          </main>
        </div>
      </div>
      <CloseDialog />
    </div>
  );
}
