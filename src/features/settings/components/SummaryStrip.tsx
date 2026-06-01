import { useTranslation } from "react-i18next";
import {
  normalizeAccent,
  normalizeFlavor,
  normalizeFont,
  normalizeFontScale,
  type Appearance,
} from "../../../appearance";
import { DEFAULT_LOCALE, isSupportedLocale } from "../../../runtime/i18n";

/** The six categories the Summary_Strip reports (Req 9.1). */
export type SummaryCategory =
  | "flavor"
  | "language"
  | "accentColor"
  | "fontScale"
  | "displayFont"
  | "bodyFont";

/** A raw, possibly-partial appearance slice the summary resolves from. */
export type SummaryInput = Partial<Record<keyof Appearance, unknown>>;

/** The six categories in display order. */
export const SUMMARY_CATEGORIES: readonly SummaryCategory[] = [
  "flavor",
  "language",
  "accentColor",
  "fontScale",
  "displayFont",
  "bodyFont",
];

/** i18n key prefix for each category's option-value label. */
const VALUE_KEY_PREFIX: Record<SummaryCategory, string> = {
  flavor: "settingsView.appearance.flavorOption",
  language: "settingsView.appearance.locale",
  accentColor: "settingsView.appearance.accentOption",
  fontScale: "settingsView.appearance.fontScaleOption",
  displayFont: "settingsView.appearance.fontOption",
  bodyFont: "settingsView.appearance.fontOption",
};

/**
 * Pure view-model derivation (Property 12): resolves each category to its active
 * value, falling back to that field's documented default when unset/invalid so
 * no category is ever blank (Req 9.4). Changing one input field changes only its
 * own resolved value, leaving the other five untouched (Req 9.3).
 */
export function deriveSummary(
  appearance: SummaryInput,
  locale: string | null | undefined,
): Record<SummaryCategory, string> {
  return {
    flavor: normalizeFlavor(appearance.flavor),
    language: isSupportedLocale(locale) ? locale : DEFAULT_LOCALE,
    accentColor: normalizeAccent(appearance.accentColor),
    fontScale: normalizeFontScale(appearance.fontScale),
    displayFont: normalizeFont(appearance.displayFont),
    bodyFont: normalizeFont(appearance.bodyFont),
  };
}

export interface SummaryStripProps {
  /** The currently applied appearance. */
  appearance: Appearance;
  /** The active locale id (for the language category). */
  locale: string;
}

/** Labeled-chip strip of the six effective appearance selections (Req 9.1, 9.2). */
export function SummaryStrip({ appearance, locale }: SummaryStripProps) {
  const { t } = useTranslation();
  const summary = deriveSummary(appearance, locale);

  return (
    <div
      role="list"
      aria-label={t("settingsView.appearance.summary")}
      className="flex flex-wrap gap-2"
    >
      {SUMMARY_CATEGORIES.map((category) => (
        <div
          key={category}
          role="listitem"
          className="flex items-center gap-1.5 rounded-md border border-border bg-muted px-2.5 py-1 text-xs"
        >
          <span className="text-muted-foreground">{t(`settingsView.appearance.${category}`)}</span>
          <span className="font-medium text-foreground">
            {t(`${VALUE_KEY_PREFIX[category]}.${summary[category]}`)}
          </span>
        </div>
      ))}
    </div>
  );
}
