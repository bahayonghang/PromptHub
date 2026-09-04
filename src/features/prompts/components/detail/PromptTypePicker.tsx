import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { CheckIcon, LoaderCircleIcon, PlusIcon, XIcon } from "lucide-react";
import {
  PROMPT_TYPES,
  type CreatePromptTypeInput,
  type PromptType,
  type PromptTypeDefinition,
} from "../../types";

import { IconButton, Select} from "../../../../components/ui";

export interface PromptTypePickerProps {
  definitions: PromptTypeDefinition[];
  baseKind: PromptType;
  definitionId: string | null;
  onChange: (baseKind: PromptType, definitionId: string | null) => void;
  onCreate: (
    input: CreatePromptTypeInput,
  ) => Promise<PromptTypeDefinition | null>;
  disabled?: boolean;
}

export function PromptTypePicker({
  definitions,
  baseKind,
  definitionId,
  onChange,
  onCreate,
  disabled = false,
}: PromptTypePickerProps) {
  const { t } = useTranslation();
  const selectRef = useRef<HTMLSelectElement>(null);
  const createButtonRef = useRef<HTMLButtonElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const [creating, setCreating] = useState(false);
  const [name, setName] = useState("");
  const [newBaseKind, setNewBaseKind] = useState<PromptType>("text");
  const [busy, setBusy] = useState(false);
  const [validationError, setValidationError] = useState<string | null>(null);

  useEffect(() => {
    if (creating) inputRef.current?.focus();
  }, [creating]);

  const cancel = () => {
    if (busy) return;
    setCreating(false);
    setName("");
    setValidationError(null);
    createButtonRef.current?.focus();
  };

  const submit = async () => {
    if (busy) return;
    const trimmedName = name.trim();
    if (trimmedName === "") {
      setValidationError("promptsView.editor.typeNameRequired");
      return;
    }
    if ([...trimmedName].length > 100) {
      setValidationError("promptsView.editor.typeNameTooLong");
      return;
    }
    setValidationError(null);
    setBusy(true);
    const definition = await onCreate({
      name: trimmedName,
      baseKind: newBaseKind,
    });
    setBusy(false);
    if (!definition) return;
    onChange(definition.baseKind, definition.id);
    setCreating(false);
    setName("");
    selectRef.current?.focus();
  };

  const inputClass =
    "min-w-0 w-full rounded-md border border-input bg-background px-3 py-2 text-body text-foreground outline-none";
  const value = definitionId ? `custom:${definitionId}` : `base:${baseKind}`;

  return (
    <div className="flex flex-col gap-1.5">
      <label
        className="text-label font-medium text-muted-foreground"
        htmlFor="prompt-type"
      >
        {t("promptsView.editor.type")}
      </label>
      <div className="grid grid-cols-[minmax(0,1fr)_2.25rem] gap-2">
        <Select
          ref={selectRef}
          id="prompt-type"
          value={value}
          disabled={disabled}
          onChange={(event) => {
            const selected = event.target.value;
            if (selected.startsWith("base:")) {
              onChange(selected.slice(5) as PromptType, null);
              return;
            }
            const definition = definitions.find(
              (item) => `custom:${item.id}` === selected,
            );
            if (definition) onChange(definition.baseKind, definition.id);
          }}
          wrapperClassName={inputClass}
        >
          <optgroup label={t("promptsView.editor.builtInTypes")}>
            {PROMPT_TYPES.map((type) => (
              <option key={type} value={`base:${type}`}>
                {t(
                  `promptsView.editor.type${type[0].toUpperCase()}${type.slice(1)}`,
                )}
              </option>
            ))}
          </optgroup>
          {definitions.length > 0 && (
            <optgroup label={t("promptsView.editor.customTypes")}>
              {definitions.map((definition) => (
                <option key={definition.id} value={`custom:${definition.id}`}>
                  {definition.name}
                </option>
              ))}
            </optgroup>
          )}
        </Select>
        <IconButton
          label={t("promptsView.editor.newType")}
          icon={<PlusIcon className="h-4 w-4" aria-hidden="true" />}
          size="lg"
          variant="bordered"
          disabled={disabled}
          onClick={() => {
            setCreating(true);
            setNewBaseKind(baseKind);
            setValidationError(null);
          }}
          aria-expanded={creating}
          ref={createButtonRef}
        />
      </div>
      {creating && (
        <div className="flex flex-col gap-1">
          <div className="grid grid-cols-[minmax(0,1fr)_minmax(7rem,0.6fr)_2.25rem_2.25rem] gap-2">
            <input
              ref={inputRef}
              value={name}
              aria-label={t("promptsView.editor.typeName")}
              aria-busy={busy}
              aria-invalid={validationError != null}
              aria-describedby={
                validationError ? "prompt-type-name-error" : undefined
              }
              placeholder={t("promptsView.editor.typeNamePlaceholder")}
              disabled={busy}
              onChange={(event) => {
                setName(event.target.value);
                setValidationError(null);
              }}
              onKeyDown={(event) => {
                if (event.key === "Enter") {
                  event.preventDefault();
                  void submit();
                } else if (event.key === "Escape") {
                  event.preventDefault();
                  cancel();
                }
              }}
              className={inputClass}
            />
            <Select
              value={newBaseKind}
              aria-label={t("promptsView.editor.baseType")}
              disabled={busy}
              onChange={(event) =>
                setNewBaseKind(event.target.value as PromptType)
              }
              wrapperClassName={inputClass}
            >
              {PROMPT_TYPES.map((type) => (
                <option key={type} value={type}>
                  {t(
                    `promptsView.editor.type${type[0].toUpperCase()}${type.slice(1)}`,
                  )}
                </option>
              ))}
            </Select>
            <IconButton
              label={
                busy
                  ? t("promptsView.editor.creatingType")
                  : t("promptsView.editor.createType")
              }
              icon={busy ? (
                <LoaderCircleIcon
                  className="h-4 w-4 animate-spin"
                  aria-hidden="true"
                />
              ) : (
                <CheckIcon className="h-4 w-4" aria-hidden="true" />
              )}
              size="lg"
              variant="bordered"
              disabled={busy}
              onClick={() => void submit()}
            />
            <IconButton
              label={t("promptsView.editor.cancelTypeCreate")}
              icon={<XIcon className="h-4 w-4" aria-hidden="true" />}
              size="lg"
              variant="bordered"
              disabled={busy}
              onClick={cancel}
            />
          </div>
          {validationError && (
            <span
              id="prompt-type-name-error"
              role="alert"
              className="text-label text-destructive"
            >
              {t(validationError)}
            </span>
          )}
        </div>
      )}
    </div>
  );
}
