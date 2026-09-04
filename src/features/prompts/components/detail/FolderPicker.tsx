import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { CheckIcon, FolderPlusIcon, LoaderCircleIcon, XIcon } from "lucide-react";
import type { CreateFolderInput, Folder } from "../../types";

import { IconButton, Input, Select } from "../../../../components/ui";

export interface FolderPickerProps {
  folders: Folder[];
  value: string | null;
  onChange: (folderId: string | null) => void;
  onCreateFolder: (input: CreateFolderInput) => Promise<Folder | null>;
  disabled?: boolean;
}

export function FolderPicker({
  folders,
  value,
  onChange,
  onCreateFolder,
  disabled = false,
}: FolderPickerProps) {
  const { t } = useTranslation();
  const selectRef = useRef<HTMLSelectElement>(null);
  const createButtonRef = useRef<HTMLButtonElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const [creating, setCreating] = useState(false);
  const [name, setName] = useState("");
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
      setValidationError("promptsView.editor.folderNameRequired");
      return;
    }
    if (trimmedName.length > 255) {
      setValidationError("promptsView.editor.folderNameTooLong");
      return;
    }

    setValidationError(null);
    setBusy(true);
    const folder = await onCreateFolder({ name: trimmedName, parentId: null });
    setBusy(false);
    if (!folder) return;

    onChange(folder.id);
    setCreating(false);
    setName("");
    selectRef.current?.focus();
  };

  return (
    <div className="flex flex-col gap-1.5">
      <label
        className="text-label font-medium text-muted-foreground"
        htmlFor="prompt-folder"
      >
        {t("promptsView.editor.folder")}
      </label>
      <div className="grid grid-cols-[minmax(0,1fr)_2.25rem] gap-2">
        <Select
          ref={selectRef}
          id="prompt-folder"
          value={value ?? ""}
          disabled={disabled}
          onChange={(event) => onChange(event.target.value || null)}
          block
        >
          <option value="">{t("promptsView.editor.noFolder")}</option>
          {folders.map((folder) => (
            <option key={folder.id} value={folder.id}>
              {folder.name}
            </option>
          ))}
        </Select>
        <IconButton
          label={t("promptsView.newFolder")}
          icon={<FolderPlusIcon className="h-4 w-4" aria-hidden="true" />}
          size="lg"
          variant="bordered"
          disabled={disabled}
          onClick={() => {
            setCreating(true);
            setValidationError(null);
          }}
          aria-expanded={creating}
          ref={createButtonRef}
        />
      </div>
      {creating && (
        <div className="flex flex-col gap-1">
          <div className="grid grid-cols-[minmax(0,1fr)_2.25rem_2.25rem] gap-2">
            <Input
              ref={inputRef}
              value={name}
              aria-label={t("promptsView.editor.folderName")}
              aria-busy={busy}
              aria-invalid={validationError != null}
              aria-describedby={
                validationError ? "prompt-folder-name-error" : undefined
              }
              placeholder={t("promptsView.folderNamePlaceholder")}
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
              size="lg"
            />
            <IconButton
              label={
                busy
                  ? t("promptsView.editor.creatingFolder")
                  : t("promptsView.editor.createFolder")
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
              label={t("promptsView.editor.cancelFolderCreate")}
              icon={<XIcon className="h-4 w-4" aria-hidden="true" />}
              size="lg"
              variant="bordered"
              disabled={busy}
              onClick={cancel}
            />
          </div>
          {validationError && (
            <span
              id="prompt-folder-name-error"
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
