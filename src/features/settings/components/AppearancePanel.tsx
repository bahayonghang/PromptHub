import { useEffect, useMemo, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import {
  ChevronDownIcon,
  ChevronUpIcon,
  DropletIcon,
  MonitorIcon,
  MoonIcon,
  PaletteIcon,
  PlusIcon,
  RowsIcon,
  SunIcon,
  TypeIcon,
  XIcon,
  ZoomInIcon,
  type LucideIcon,
} from "lucide-react";
import {
  ACCENT_COLORS,
  ACCENT_PALETTE,
  DENSITIES,
  FLAVOR_BASE,
  FLAVOR_OVERRIDES,
  FONT_CATALOG,
  FONT_SCALES,
} from "../../../appearance";
import {
  CATPPUCCIN_DARK_VARIANTS,
  COLOR_MODES,
  MAX_INTERFACE_FONT_FAMILIES,
  THEME_FAMILIES,
  normalizeAppearancePreferences,
  resolveThemeVariant,
  type AppearancePreferences,
} from "../../../appearance/preferences";
import { useSettingsStore, type PreferenceKey, type PreferenceValue } from "../settingsStore";
import { PreferenceStatus } from "./PreferenceStatus";
import { SpecimenCard } from "./SpecimenCard";

const labelClass = "text-sm font-medium text-foreground";
const selectClass =
  "w-full rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50";
const optionClass =
  "rounded-md border px-3 py-2 text-sm transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50";
const iconButtonClass =
  "flex h-8 w-8 shrink-0 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-accent hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-35";

function Section({
  icon: Icon,
  label,
  hint,
  children,
}: {
  icon: LucideIcon;
  label: string;
  hint?: string;
  children: ReactNode;
}) {
  return (
    <section className="flex flex-col gap-2.5">
      <div className="flex flex-col gap-0.5">
        <h3 className={`flex items-center gap-2 ${labelClass}`}>
          <Icon className="h-4 w-4 shrink-0 text-muted-foreground" aria-hidden="true" />
          {label}
        </h3>
        {hint && <p className="pl-6 text-xs text-muted-foreground">{hint}</p>}
      </div>
      {children}
    </section>
  );
}

function swatch(flavor: keyof typeof FLAVOR_OVERRIDES, token: "--background" | "--muted"): string {
  return `hsl(${FLAVOR_OVERRIDES[flavor][token]})`;
}

function effectiveVariant(preferences: AppearancePreferences) {
  const systemDark =
    typeof document !== "undefined" && document.documentElement.classList.contains("dark");
  return resolveThemeVariant(preferences, systemDark);
}

export function AppearancePanel() {
  const { t } = useTranslation();
  const settings = useSettingsStore((state) => state.settings);
  const preferenceStatus = useSettingsStore((state) => state.preferenceStatus);
  const preferenceErrors = useSettingsStore((state) => state.preferenceErrors);
  const setPreference = useSettingsStore((state) => state.setPreference);
  const retryPreference = useSettingsStore((state) => state.retryPreference);
  const systemFonts = useSettingsStore((state) => state.systemFonts);
  const systemFontsStatus = useSettingsStore((state) => state.systemFontsStatus);
  const loadSystemFonts = useSettingsStore((state) => state.loadSystemFonts);
  const retrySystemFonts = useSettingsStore((state) => state.retrySystemFonts);
  const applied = normalizeAppearancePreferences(settings ?? {});
  const variant = effectiveVariant(applied);
  const fontStack = applied.interfaceFontStack;
  const availableFonts = useMemo(
    () => [...new Set([...FONT_CATALOG, ...systemFonts])],
    [systemFonts],
  );

  useEffect(() => {
    void loadSystemFonts();
  }, [loadSystemFonts]);

  const save = (key: PreferenceKey, value: PreferenceValue) => {
    void setPreference(key, value);
  };
  const status = (key: PreferenceKey) => (
    <PreferenceStatus
      status={preferenceStatus[key]}
      errorKey={preferenceErrors[key]}
      onRetry={() => void retryPreference(key)}
    />
  );
  const saving = (key: PreferenceKey) => preferenceStatus[key] === "saving";
  const updateFont = (index: number, family: string) => {
    const next = [...fontStack];
    next[index] = family;
    save("interfaceFontStack", next);
  };
  const moveFont = (index: number, direction: -1 | 1) => {
    const target = index + direction;
    if (target < 0 || target >= fontStack.length) return;
    const next = [...fontStack];
    [next[index], next[target]] = [next[target], next[index]];
    save("interfaceFontStack", next);
  };
  const removeFont = (index: number) => {
    if (fontStack.length === 1) return;
    save("interfaceFontStack", fontStack.filter((_, itemIndex) => itemIndex !== index));
  };
  const nextUnusedFont = availableFonts.find(
    (family) => !fontStack.some((selected) => selected.toLocaleLowerCase() === family.toLocaleLowerCase()),
  );

  return (
    <div className="flex flex-col gap-7">
      <Section
        icon={PaletteIcon}
        label={t("settingsView.appearance.themeFamily")}
        hint={t("settingsView.appearance.themeFamilyHint")}
      >
        <div
          className="grid grid-cols-1 gap-2 sm:grid-cols-2"
          role="group"
          aria-label={t("settingsView.appearance.themeFamily")}
        >
          {THEME_FAMILIES.map((family) => {
            const selected = applied.themeFamily === family;
            const variants = family === "catppuccin" ? (["Latte", "Mocha"] as const) : (["Claude Light", "Claude Dark"] as const);
            return (
              <button
                key={family}
                type="button"
                aria-pressed={selected}
                disabled={saving("themeFamily")}
                onClick={() => save("themeFamily", family)}
                className={`flex min-h-14 items-center gap-3 rounded-md border px-3 py-2.5 text-left transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring ${
                  selected
                    ? "border-primary bg-primary/10 text-foreground"
                    : "border-input text-muted-foreground hover:bg-accent hover:text-foreground"
                }`}
              >
                <span className="flex shrink-0 overflow-hidden rounded border border-border" aria-hidden="true">
                  {variants.map((item) => (
                    <span key={item} className="h-7 w-5" style={{ backgroundColor: swatch(item, "--background") }} />
                  ))}
                </span>
                <span className="min-w-0 break-words">
                  <span className="block text-sm font-medium">
                    {t(`settingsView.appearance.familyOption.${family}`)}
                  </span>
                  <span className="block text-xs text-muted-foreground">
                    {t(`settingsView.appearance.familyDescription.${family}`)}
                  </span>
                </span>
              </button>
            );
          })}
        </div>
        {status("themeFamily")}
      </Section>

      <Section icon={MonitorIcon} label={t("settingsView.appearance.colorMode")}>
        <div
          className="inline-flex w-fit max-w-full rounded-md border border-input bg-muted/40 p-1"
          role="group"
          aria-label={t("settingsView.appearance.colorMode")}
        >
          {COLOR_MODES.map((mode) => {
            const Icon = mode === "light" ? SunIcon : mode === "dark" ? MoonIcon : MonitorIcon;
            const selected = applied.theme === mode;
            return (
              <button
                key={mode}
                type="button"
                aria-pressed={selected}
                disabled={saving("theme")}
                onClick={() => save("theme", mode)}
                className={`inline-flex min-h-9 items-center gap-1.5 rounded px-3 text-sm transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring ${
                  selected ? "bg-background font-medium text-foreground shadow-sm" : "text-muted-foreground hover:text-foreground"
                }`}
              >
                <Icon className="h-4 w-4" aria-hidden="true" />
                {t(`settingsView.general.themeMode.${mode}`)}
              </button>
            );
          })}
        </div>
        {status("theme")}
      </Section>

      {applied.themeFamily === "catppuccin" && (
        <Section icon={MoonIcon} label={t("settingsView.appearance.catppuccinDarkVariant")}>
          <select
            value={applied.catppuccinDarkVariant}
            aria-label={t("settingsView.appearance.catppuccinDarkVariant")}
            disabled={saving("catppuccinDarkVariant")}
            onChange={(event) => save("catppuccinDarkVariant", event.target.value)}
            className={`${selectClass} max-w-sm`}
          >
            {CATPPUCCIN_DARK_VARIANTS.map((item) => (
              <option key={item} value={item}>
                {t(`settingsView.appearance.darkVariantOption.${item}`)}
              </option>
            ))}
          </select>
          {status("catppuccinDarkVariant")}
        </Section>
      )}

      <Section icon={DropletIcon} label={t("settingsView.appearance.accentColor")}>
        <div className="flex flex-wrap gap-2" role="group" aria-label={t("settingsView.appearance.accentColor")}>
          {ACCENT_COLORS.map((accent) => (
            <button
              key={accent}
              type="button"
              aria-pressed={applied.accentColor === accent}
              aria-label={t(`settingsView.appearance.accentOption.${accent}`)}
              title={t(`settingsView.appearance.accentOption.${accent}`)}
              disabled={saving("accentColor")}
              onClick={() => save("accentColor", accent)}
              className={`h-8 w-8 rounded-full border-2 transition-transform hover:scale-105 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background disabled:cursor-not-allowed disabled:opacity-50 ${
                applied.accentColor === accent ? "border-foreground" : "border-transparent"
              }`}
              style={{ backgroundColor: `hsl(${ACCENT_PALETTE[FLAVOR_BASE[variant]][accent]["--primary"]})` }}
            />
          ))}
        </div>
        {status("accentColor")}
      </Section>

      <Section
        icon={TypeIcon}
        label={t("settingsView.appearance.interfaceFonts")}
        hint={t("settingsView.appearance.interfaceFontsHint")}
      >
        <ol className="flex max-w-xl flex-col gap-2">
          {fontStack.map((font, index) => {
            const options = availableFonts.filter(
              (family) =>
                family === font ||
                !fontStack.some((selected) => selected.toLocaleLowerCase() === family.toLocaleLowerCase()),
            );
            return (
              <li key={`${font}-${index}`} className="flex items-center gap-2">
                <label className="min-w-0 flex-1">
                  <span className="mb-1 block text-xs text-muted-foreground">
                    {index === 0
                      ? t("settingsView.appearance.primaryFont")
                      : t("settingsView.appearance.fallbackFont", { index })}
                  </span>
                  <select
                    value={font}
                    disabled={saving("interfaceFontStack")}
                    onChange={(event) => updateFont(index, event.target.value)}
                    className={selectClass}
                  >
                    {options.map((family) => (
                      <option key={family} value={family}>{family}</option>
                    ))}
                  </select>
                </label>
                <div className="mt-5 flex items-center gap-0.5">
                  <button type="button" aria-label={t("settingsView.appearance.moveFontUp")} title={t("settingsView.appearance.moveFontUp")} disabled={index === 0 || saving("interfaceFontStack")} onClick={() => moveFont(index, -1)} className={iconButtonClass}>
                    <ChevronUpIcon className="h-4 w-4" aria-hidden="true" />
                  </button>
                  <button type="button" aria-label={t("settingsView.appearance.moveFontDown")} title={t("settingsView.appearance.moveFontDown")} disabled={index === fontStack.length - 1 || saving("interfaceFontStack")} onClick={() => moveFont(index, 1)} className={iconButtonClass}>
                    <ChevronDownIcon className="h-4 w-4" aria-hidden="true" />
                  </button>
                  <button type="button" aria-label={t("settingsView.appearance.removeFont")} title={t("settingsView.appearance.removeFont")} disabled={fontStack.length === 1 || saving("interfaceFontStack")} onClick={() => removeFont(index)} className={iconButtonClass}>
                    <XIcon className="h-4 w-4" aria-hidden="true" />
                  </button>
                </div>
              </li>
            );
          })}
        </ol>
        <button
          type="button"
          disabled={fontStack.length >= MAX_INTERFACE_FONT_FAMILIES || nextUnusedFont == null || saving("interfaceFontStack")}
          onClick={() => nextUnusedFont && save("interfaceFontStack", [...fontStack, nextUnusedFont])}
          className="inline-flex w-fit items-center gap-1.5 rounded-md border border-input px-3 py-2 text-sm text-foreground transition-colors hover:bg-accent focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50"
        >
          <PlusIcon className="h-4 w-4" aria-hidden="true" />
          {t("settingsView.appearance.addFallback")}
        </button>
        {systemFontsStatus === "loading" && (
          <div role="status" className="h-4 w-40 animate-pulse rounded bg-muted" aria-label={t("settingsView.appearance.systemFontsLoading")} />
        )}
        {systemFontsStatus === "empty" && <p className="text-xs text-muted-foreground">{t("settingsView.appearance.systemFontsEmpty")}</p>}
        {systemFontsStatus === "error" && (
          <div role="alert" className="flex flex-wrap items-center gap-2 text-xs text-destructive">
            <span>{t("settingsView.appearance.systemFontsError")}</span>
            <button type="button" onClick={() => void retrySystemFonts()} className="font-medium underline underline-offset-2 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring">
              {t("settingsView.preferences.retry")}
            </button>
          </div>
        )}
        {status("interfaceFontStack")}
      </Section>

      <Section icon={ZoomInIcon} label={t("settingsView.appearance.fontScale")}>
        <div className="flex flex-wrap gap-2" role="group" aria-label={t("settingsView.appearance.fontScale")}>
          {FONT_SCALES.map((scale) => (
            <button key={scale} type="button" aria-pressed={applied.fontScale === scale} disabled={saving("fontScale")} onClick={() => save("fontScale", scale)} className={`${optionClass} ${applied.fontScale === scale ? "border-primary bg-primary/10 text-foreground" : "border-input text-muted-foreground hover:bg-accent hover:text-foreground"}`}>
              {t(`settingsView.appearance.fontScaleOption.${scale}`)}
            </button>
          ))}
        </div>
        {status("fontScale")}
      </Section>

      <Section icon={RowsIcon} label={t("settingsView.appearance.density")}>
        <div className="flex flex-wrap gap-2" role="group" aria-label={t("settingsView.appearance.density")}>
          {DENSITIES.map((density) => (
            <button key={density} type="button" aria-pressed={applied.density === density} disabled={saving("density")} onClick={() => save("density", density)} className={`${optionClass} ${applied.density === density ? "border-primary bg-primary/10 text-foreground" : "border-input text-muted-foreground hover:bg-accent hover:text-foreground"}`}>
              {t(`settingsView.appearance.densityOption.${density}`)}
            </button>
          ))}
        </div>
        {status("density")}
      </Section>

      <Section icon={PaletteIcon} label={t("settingsView.appearance.preview")}>
        <SpecimenCard />
      </Section>
    </div>
  );
}
