import { useTranslation } from "react-i18next";
import { VARIABLE_TYPES, type Variable, type VariableType } from "../types";

import { EmptyHint, Select } from "../../../components/ui";

interface VariableEditorProps {
  /** Variables derived from the prompt text's `{{name}}` placeholders. */
  variables: Variable[];
  onChange: (variables: Variable[]) => void;
}

/**
 * Edits the metadata of a prompt's variables (Req 6.7). The variable set itself
 * is derived from the `{{name}}` placeholders in the prompt text (see
 * {@link syncVariables}); this control only edits each variable's type, label,
 * default value, and required flag.
 */
export function VariableEditor({ variables, onChange }: VariableEditorProps) {
  const { t } = useTranslation();

  const patch = (name: string, change: Partial<Variable>) =>
    onChange(variables.map((v) => (v.name === name ? { ...v, ...change } : v)));

  const labelClass = "text-label font-medium text-muted-foreground";
  const cellClass =
    "rounded-md border border-input bg-background px-2 py-1 text-label text-foreground outline-none";

  return (
    <div className="flex flex-col gap-1.5">
      <span className={labelClass}>{t("promptsView.editor.variables")}</span>
      {variables.length === 0 ? (
        <EmptyHint>{t("promptsView.editor.noVariables")}</EmptyHint>
      ) : (
        <ul className="flex flex-col gap-2">
          {variables.map((variable) => (
            <li
              key={variable.name}
              className="flex flex-wrap items-center gap-2 rounded-md border border-border bg-card p-2"
            >
              <code className="rounded-sm bg-muted px-1.5 py-0.5 text-label text-foreground">
                {`{{${variable.name}}}`}
              </code>
              <Select
                aria-label={t("promptsView.editor.variableType")}
                value={variable.type}
                onChange={(e) =>
                  patch(variable.name, { type: e.target.value as VariableType })
                }
                wrapperClassName={cellClass}
              >
                {VARIABLE_TYPES.map((type) => (
                  <option key={type} value={type}>
                    {type}
                  </option>
                ))}
              </Select>
              <input
                aria-label={t("promptsView.editor.variableLabel")}
                value={variable.label ?? ""}
                placeholder={t("promptsView.editor.variableLabelPlaceholder")}
                onChange={(e) => patch(variable.name, { label: e.target.value })}
                className={`${cellClass} min-w-0 flex-1`}
              />
              <input
                aria-label={t("promptsView.editor.variableDefault")}
                value={variable.defaultValue ?? ""}
                placeholder={t("promptsView.editor.variableDefaultPlaceholder")}
                onChange={(e) =>
                  patch(variable.name, { defaultValue: e.target.value })
                }
                className={`${cellClass} min-w-0 flex-1`}
              />
              <label className="flex items-center gap-1 text-label text-foreground">
                <input
                  type="checkbox"
                  checked={variable.required}
                  onChange={(e) =>
                    patch(variable.name, { required: e.target.checked })
                  }
                  className="h-3.5 w-3.5 accent-[hsl(var(--primary))]"
                />
                {t("promptsView.editor.variableRequired")}
              </label>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
