import { useState } from "react";
import { useTranslation } from "react-i18next";
import { PlusIcon, Trash2Icon } from "lucide-react";
import { MAX_SHORTCUTS, type Shortcut, type ShortcutMode } from "../types";
import { useSystemStore } from "../systemStore";

/** A blank shortcut row used when adding a new entry. */
const EMPTY_SHORTCUT: Shortcut = { action: "", accelerator: "", mode: "global" };

/**
 * Keyboard-shortcut configuration (Req 20.6, 20.11). Edits a draft list of up to
 * {@link MAX_SHORTCUTS} shortcuts — each with an action id, an accelerator, and a
 * global/local mode — then registers the full set through the system store, which
 * calls the Window_Manager via the Runtime_Bridge (Req 3.1). A conflicting set is
 * rejected by the backend and the previously registered set is left unchanged
 * (Req 20.11); the rejection surfaces through the store error. The last triggered
 * shortcut (from `shortcut:triggered`) is shown for confirmation. All text
 * resolves through i18n (Req 21.3) and icons are from Lucide (Req 22.4).
 */
export function ShortcutsPanel() {
  const { t } = useTranslation();

  const registered = useSystemStore((s) => s.shortcuts);
  const lastTriggered = useSystemStore((s) => s.lastTriggeredAction);
  const registerShortcuts = useSystemStore((s) => s.registerShortcuts);

  const [draft, setDraft] = useState<Shortcut[]>(registered);

  const labelClass = "text-sm font-medium text-foreground";
  const hintClass = "text-xs text-muted-foreground";
  const inputClass =
    "rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground outline-none focus:ring-1 focus:ring-ring";

  const updateRow = (index: number, patch: Partial<Shortcut>) => {
    setDraft((rows) => rows.map((row, i) => (i === index ? { ...row, ...patch } : row)));
  };

  const removeRow = (index: number) => {
    setDraft((rows) => rows.filter((_, i) => i !== index));
  };

  const addRow = () => {
    setDraft((rows) => (rows.length < MAX_SHORTCUTS ? [...rows, { ...EMPTY_SHORTCUT }] : rows));
  };

  const save = () => {
    // Drop incomplete rows so empty drafts never reach the backend.
    const complete = draft.filter(
      (row) => row.action.trim() !== "" && row.accelerator.trim() !== "",
    );
    void registerShortcuts(complete);
  };

  return (
    <section className="flex flex-col gap-3">
      <div className="flex flex-col gap-0.5">
        <h3 className={labelClass}>{t("systemView.shortcuts.title")}</h3>
        <p className={hintClass}>
          {t("systemView.shortcuts.hint", { max: MAX_SHORTCUTS })}
        </p>
      </div>

      <div className="flex flex-col gap-2">
        {draft.length === 0 && (
          <p className={hintClass}>{t("systemView.shortcuts.empty")}</p>
        )}
        {draft.map((row, index) => (
          <div key={index} className="flex flex-wrap items-center gap-2">
            <input
              type="text"
              value={row.action}
              onChange={(e) => updateRow(index, { action: e.target.value })}
              placeholder={t("systemView.shortcuts.actionPlaceholder")}
              aria-label={t("systemView.shortcuts.action")}
              className={`${inputClass} min-w-0 flex-1`}
            />
            <input
              type="text"
              value={row.accelerator}
              onChange={(e) => updateRow(index, { accelerator: e.target.value })}
              placeholder={t("systemView.shortcuts.acceleratorPlaceholder")}
              aria-label={t("systemView.shortcuts.accelerator")}
              className={`${inputClass} min-w-0 flex-1`}
            />
            <select
              value={row.mode}
              onChange={(e) => updateRow(index, { mode: e.target.value as ShortcutMode })}
              aria-label={t("systemView.shortcuts.mode")}
              className={inputClass}
            >
              <option value="global">{t("systemView.shortcuts.modeGlobal")}</option>
              <option value="local">{t("systemView.shortcuts.modeLocal")}</option>
            </select>
            <button
              type="button"
              onClick={() => removeRow(index)}
              title={t("systemView.shortcuts.remove")}
              aria-label={t("systemView.shortcuts.remove")}
              className="flex h-9 w-9 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-destructive hover:text-destructive-foreground"
            >
              <Trash2Icon className="h-4 w-4" aria-hidden="true" />
            </button>
          </div>
        ))}
      </div>

      <div className="flex flex-wrap items-center gap-2">
        <button
          type="button"
          onClick={addRow}
          disabled={draft.length >= MAX_SHORTCUTS}
          className="inline-flex items-center gap-2 rounded-md border border-input px-3 py-2 text-sm text-foreground transition-colors hover:bg-accent disabled:cursor-not-allowed disabled:opacity-50"
        >
          <PlusIcon className="h-4 w-4" aria-hidden="true" />
          {t("systemView.shortcuts.add")}
        </button>
        <button
          type="button"
          onClick={save}
          className="rounded-md bg-primary px-3 py-2 text-sm text-primary-foreground transition-colors hover:bg-primary/90"
        >
          {t("systemView.shortcuts.save")}
        </button>
      </div>

      {lastTriggered && (
        <p className={hintClass} role="status">
          {t("systemView.shortcuts.lastTriggered", { action: lastTriggered })}
        </p>
      )}
    </section>
  );
}
