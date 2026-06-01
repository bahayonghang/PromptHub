import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import {
  DropletIcon,
  LanguagesIcon,
  type LucideIcon,
  PaletteIcon,
  PilcrowIcon,
  RowsIcon,
  TypeIcon,
  ZoomInIcon,
} from "lucide-react";
import {
  ACCENT_COLORS,
  ACCENT_PALETTE,
  type AccentColor,
  type Appearance,
  type AppearanceController,
  DENSITIES,
  type Density,
  FLAVOR_BASE,
  FLAVOR_OVERRIDES,
  FLAVORS,
  type Flavor,
  FONT_CATALOG,
  FONT_SCALES,
  type FontFamilyName,
  type FontScale,
  normalizeAppearance,
  setAccentColor,
  setBodyFont,
  setDensity,
  setDisplayFont,
  setFlavor,
  setFontScale,
} from "../../../appearance";
import {
  changeLocale,
  DEFAULT_LOCALE,
  isSupportedLocale,
  SUPPORTED,
  type SupportedLocale,
} from "../../../runtime/i18n";
import { runtime, type RuntimeBridge } from "../../../runtime";
import { useSettingsStore } from "../settingsStore";
import type { Settings } from "../types";
import { SpecimenCard } from "./SpecimenCard";
import { SummaryStrip } from "./SummaryStrip";

/** The `invoke` slice of the Runtime_Bridge the set* entry points persist through. */
type Invoke = RuntimeBridge["invoke"];

/** A `BridgeError`-shaped message extractor, mirroring the settings store. */
function errorMessage(err: unknown): string {
  if (err && typeof err === "object" && "message" in err) {
    return String((err as { message: unknown }).message);
  }
  return String(err);
}

/** CSS color for a flavor's surface swatch (HSL channels -> `hsl(...)`). */
function flavorSwatch(flavor: Flavor): string {
  return `hsl(${FLAVOR_OVERRIDES[flavor]["--background"]})`;
}

/** CSS color for an accent's primary swatch under the active flavor's base. */
function accentSwatch(flavor: Flavor, accent: AccentColor): string {
  return `hsl(${ACCENT_PALETTE[FLAVOR_BASE[flavor]][accent]["--primary"]})`;
}

export interface AppearancePanelProps {
  /** Bridge invoke forwarded to the set* entry points; injectable for tests. */
  invoke?: Invoke;
  /** Appearance controller forwarded to the set* entry points; injectable for tests. */
  controller?: AppearanceController;
  /** Locale switcher; injectable for tests, defaults to the I18n_Layer. */
  changeLocaleFn?: (locale: SupportedLocale) => Promise<void>;
}

const labelClass = "text-sm font-medium text-foreground";
const selectClass =
  "w-full max-w-xs rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground outline-none focus:ring-1 focus:ring-ring";

/** A control section with a Lucide icon header (Req 1.3, 1.4). */
function Section({
  icon: Icon,
  label,
  children,
}: {
  icon: LucideIcon;
  label: string;
  children: ReactNode;
}) {
  return (
    <section className="flex flex-col gap-2">
      <h3 className={`flex items-center gap-2 ${labelClass}`}>
        <Icon className="h-4 w-4 shrink-0 text-muted-foreground" aria-hidden="true" />
        {label}
      </h3>
      {children}
    </section>
  );
}

/**
 * Appearance section panel (Req 1-9). Composes the seven controls, the live
 * Specimen_Card, and the Summary_Strip. Every selection applies instantly and
 * persists through the appearance `set*` entry points (routed via the Runtime
 * Bridge, never the raw Tauri API, Req 1.5); a persistence rejection sets the
 * settings-store error so the existing banner surfaces the "not saved"
 * indication (Req 3.8, 4.7, 5.6, 6.6, 7.6). The applied value is kept regardless.
 */
export function AppearancePanel({ invoke, controller, changeLocaleFn = changeLocale }: AppearancePanelProps) {
  const { t, i18n } = useTranslation();
  const settings = useSettingsStore((s) => s.settings);
  const mergeLocalSettings = useSettingsStore((s) => s.mergeLocalSettings);

  // The applied appearance: persisted values normalized, defaulting when unset.
  const applied = normalizeAppearance(
    {
      flavor: settings?.flavor,
      accentColor: settings?.accentColor,
      displayFont: settings?.displayFont,
      bodyFont: settings?.bodyFont,
      fontScale: settings?.fontScale,
      density: settings?.density,
    },
    settings?.theme,
  );
  const activeLocale: SupportedLocale = isSupportedLocale(i18n.language) ? i18n.language : DEFAULT_LOCALE;

  // Wrap the bridge invoke so a persistence rejection surfaces through the store
  // error channel; the set* entry points still swallow it so nothing escapes.
  const baseInvoke: Invoke = invoke ?? runtime.invoke.bind(runtime);
  function persistInvoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
    return baseInvoke<T>(command, args).catch((err) => {
      useSettingsStore.setState({ error: errorMessage(err) });
      throw err;
    });
  }

  /** Applies + persists a field via its set* entry point, then syncs the store. */
  function apply<K extends keyof Appearance>(
    field: K,
    value: Appearance[K],
    setter: (v: Appearance[K], inv?: Invoke, ctrl?: AppearanceController) => Promise<void>,
  ): void {
    void setter(value, persistInvoke, controller);
    mergeLocalSettings({ [field]: value } as Partial<Settings>);
  }

  const handleLanguage = (locale: SupportedLocale): void => {
    void changeLocaleFn(locale).catch((err) => useSettingsStore.setState({ error: errorMessage(err) }));
    mergeLocalSettings({ language: locale });
  };

  return (
    <div className="flex flex-col gap-6">
      {/* Flavor */}
      <Section icon={PaletteIcon} label={t("settingsView.appearance.flavor")}>
        <div className="flex flex-wrap gap-2" role="group" aria-label={t("settingsView.appearance.flavor")}>
          {FLAVORS.map((flavor) => (
            <button
              key={flavor}
              type="button"
              aria-pressed={applied.flavor === flavor}
              onClick={() => apply("flavor", flavor, setFlavor)}
              className={`flex items-center gap-2 rounded-md border px-3 py-2 text-sm transition-colors ${
                applied.flavor === flavor
                  ? "border-primary bg-primary/15 text-foreground"
                  : "border-input text-muted-foreground hover:bg-accent hover:text-foreground"
              }`}
            >
              <span
                className="h-4 w-4 shrink-0 rounded-full border border-border"
                style={{ backgroundColor: flavorSwatch(flavor) }}
                aria-hidden="true"
              />
              {t(`settingsView.appearance.flavorOption.${flavor}`)}
            </button>
          ))}
        </div>
      </Section>

      {/* Language */}
      <Section icon={LanguagesIcon} label={t("settingsView.appearance.language")}>
        <select
          value={activeLocale}
          aria-label={t("settingsView.appearance.language")}
          onChange={(e) => handleLanguage(e.target.value as SupportedLocale)}
          className={selectClass}
        >
          {SUPPORTED.map((locale) => (
            <option key={locale} value={locale}>
              {t(`settingsView.appearance.locale.${locale}`)}
            </option>
          ))}
        </select>
      </Section>

      {/* Accent color */}
      <Section icon={DropletIcon} label={t("settingsView.appearance.accentColor")}>
        <div className="flex flex-wrap gap-2" role="group" aria-label={t("settingsView.appearance.accentColor")}>
          {ACCENT_COLORS.map((accent) => (
            <button
              key={accent}
              type="button"
              aria-pressed={applied.accentColor === accent}
              title={t(`settingsView.appearance.accentOption.${accent}`)}
              aria-label={t(`settingsView.appearance.accentOption.${accent}`)}
              onClick={() => apply("accentColor", accent, setAccentColor)}
              className={`h-7 w-7 rounded-full border-2 transition-colors ${
                applied.accentColor === accent ? "border-foreground" : "border-transparent"
              }`}
              style={{ backgroundColor: accentSwatch(applied.flavor, accent) }}
            />
          ))}
        </div>
      </Section>

      {/* Display font */}
      <Section icon={TypeIcon} label={t("settingsView.appearance.displayFont")}>
        <select
          value={applied.displayFont}
          aria-label={t("settingsView.appearance.displayFont")}
          onChange={(e) => apply("displayFont", e.target.value as FontFamilyName, setDisplayFont)}
          className={selectClass}
        >
          {FONT_CATALOG.map((font) => (
            <option key={font} value={font}>
              {t(`settingsView.appearance.fontOption.${font}`)}
            </option>
          ))}
        </select>
      </Section>

      {/* Body font */}
      <Section icon={PilcrowIcon} label={t("settingsView.appearance.bodyFont")}>
        <select
          value={applied.bodyFont}
          aria-label={t("settingsView.appearance.bodyFont")}
          onChange={(e) => apply("bodyFont", e.target.value as FontFamilyName, setBodyFont)}
          className={selectClass}
        >
          {FONT_CATALOG.map((font) => (
            <option key={font} value={font}>
              {t(`settingsView.appearance.fontOption.${font}`)}
            </option>
          ))}
        </select>
      </Section>

      {/* Font scale */}
      <Section icon={ZoomInIcon} label={t("settingsView.appearance.fontScale")}>
        <div className="flex flex-wrap gap-2" role="group" aria-label={t("settingsView.appearance.fontScale")}>
          {FONT_SCALES.map((scale) => (
            <button
              key={scale}
              type="button"
              aria-pressed={applied.fontScale === scale}
              onClick={() => apply("fontScale", scale as FontScale, setFontScale)}
              className={`rounded-md border px-3 py-2 text-sm transition-colors ${
                applied.fontScale === scale
                  ? "border-primary bg-primary/15 text-foreground"
                  : "border-input text-muted-foreground hover:bg-accent hover:text-foreground"
              }`}
            >
              {t(`settingsView.appearance.fontScaleOption.${scale}`)}
            </button>
          ))}
        </div>
      </Section>

      {/* Density */}
      <Section icon={RowsIcon} label={t("settingsView.appearance.density")}>
        <div className="flex flex-wrap gap-2" role="group" aria-label={t("settingsView.appearance.density")}>
          {DENSITIES.map((density) => (
            <button
              key={density}
              type="button"
              aria-pressed={applied.density === density}
              onClick={() => apply("density", density as Density, setDensity)}
              className={`rounded-md border px-3 py-2 text-sm transition-colors ${
                applied.density === density
                  ? "border-primary bg-primary/15 text-foreground"
                  : "border-input text-muted-foreground hover:bg-accent hover:text-foreground"
              }`}
            >
              {t(`settingsView.appearance.densityOption.${density}`)}
            </button>
          ))}
        </div>
      </Section>

      {/* Live preview */}
      <Section icon={PaletteIcon} label={t("settingsView.appearance.preview")}>
        <SpecimenCard appearance={applied} />
      </Section>

      {/* Selection summary */}
      <Section icon={RowsIcon} label={t("settingsView.appearance.summary")}>
        <SummaryStrip appearance={applied} locale={activeLocale} />
      </Section>
    </div>
  );
}
