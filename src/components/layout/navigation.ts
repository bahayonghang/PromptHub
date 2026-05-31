import { BookOpenIcon, CommandIcon, SettingsIcon, type LucideIcon } from "lucide-react";
import { APP_VIEWS, type AppView } from "../../store/appStore";

/**
 * A single navigation entry in the application shell sidebar. `labelKey` is an
 * i18n key resolved with `t()` so no user-facing text is hard-coded (Req 21.3).
 */
export interface NavEntry {
  /** The view this entry activates. */
  view: AppView;
  /** Translation key for the entry's visible label. */
  labelKey: string;
  /** Lucide icon component for the entry (Req 22.4). */
  icon: LucideIcon;
}

/**
 * The ordered navigation entries shown in the sidebar, one per major view
 * (Req 22.3). Settings is conventionally pinned to the bottom of the sidebar;
 * {@link PRIMARY_NAV} and {@link FOOTER_NAV} split the entries accordingly.
 */
export const NAV_ENTRIES: readonly NavEntry[] = [
  { view: "prompts", labelKey: "common.prompts", icon: CommandIcon },
  { view: "skills", labelKey: "common.skills", icon: BookOpenIcon },
  { view: "settings", labelKey: "settings.title", icon: SettingsIcon },
];

/** Primary navigation entries rendered in the main sidebar group. */
export const PRIMARY_NAV: readonly NavEntry[] = NAV_ENTRIES.filter(
  (entry) => entry.view !== "settings",
);

/** Footer navigation entries (Settings) pinned to the bottom of the sidebar. */
export const FOOTER_NAV: readonly NavEntry[] = NAV_ENTRIES.filter(
  (entry) => entry.view === "settings",
);

// Invariant: every view has exactly one navigation entry. Kept as a runtime
// guard so adding a view without a nav entry (or vice versa) fails fast.
if (NAV_ENTRIES.length !== APP_VIEWS.length) {
  throw new Error("NAV_ENTRIES must contain exactly one entry per AppView");
}
