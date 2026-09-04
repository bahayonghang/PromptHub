import { useTranslation } from "react-i18next";
import { SparklesIcon } from "lucide-react";

/**
 * Live preview specimen (Req 8). Renders a display-level text sample (using
 * `--font-display`), a body-level text sample (using `--font-body`), and an
 * accent-using interactive control. The active base/flavor/accent/fonts/density
 * are scoped to this card's own container via a dedicated controller, so the
 * preview is correct even if mounted outside the document root.
 */
export function SpecimenCard() {
  const { t } = useTranslation();

  return (
    <div
      data-testid="specimen-card"
      className="density-gap flex flex-col rounded-md border border-border bg-card density-p text-card-foreground"
    >
      <p data-testid="specimen-display" className="text-lg font-semibold text-foreground">
        {t("settingsView.appearance.specimenDisplay")}
      </p>
      <p data-testid="specimen-body" className="text-body text-muted-foreground">
        {t("settingsView.appearance.specimenBody")}
      </p>
      <p className="text-body text-muted-foreground">
        {t("settingsView.appearance.specimenMultilingual")}
      </p>
      <button
        type="button"
        data-testid="specimen-action"
        className="inline-flex w-fit items-center gap-1.5 rounded-md bg-primary px-3 py-2 text-body font-medium text-primary-foreground"
      >
        <SparklesIcon className="h-4 w-4" aria-hidden="true" />
        {t("settingsView.appearance.specimenAction")}
      </button>
    </div>
  );
}
