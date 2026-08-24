import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { CheckIcon, FolderPlusIcon, LoaderCircleIcon, XIcon } from "lucide-react";
import type { CreateFolderInput, Folder } from "../../types";

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

  const inputClass =
    "min-w-0 w-full rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground outline-none focus-visible:ring-2 focus-visible:ring-ring";
  const iconButtonClass =
    "flex h-9 w-9 shrink-0 items-center justify-center rounded-md border border-input text-muted-foreground hover:bg-accent hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50";

  return (
    <div className="flex flex-col gap-1.5">
      <label
        className="text-xs font-medium text-muted-foreground"
        htmlFor="prompt-folder"
      >
        {t("promptsView.editor.folder")}
      </label>
      <div className="grid grid-cols-[minmax(0,1fr)_2.25rem] gap-2">
        <select
          ref={selectRef}
          id="prompt-folder"
          value={value ?? ""}
          disabled={disabled}
          onChange={(event) => onChange(event.target.value || null)}
          className={inputClass}
        >
          <option value="">{t("promptsView.editor.noFolder")}</option>
          {folders.map((folder) => (
            <option key={folder.id} value={folder.id}>
              {folder.name}
            </option>
          ))}
        </select>
        <button
          ref={createButtonRef}
          type="button"
          title={t("promptsView.newFolder")}
          aria-label={t("promptsView.newFolder")}
          aria-expanded={creating}
          disabled={disabled}
          onClick={() => {
            setCreating(true);
            setValidationError(null);
          }}
          className={iconButtonClass}
        >
          <FolderPlusIcon className="h-4 w-4" aria-hidden="true" />
        </button>
      </div>
      {creating && (
        <div className="flex flex-col gap-1">
          <div className="grid grid-cols-[minmax(0,1fr)_2.25rem_2.25rem] gap-2">
            <input
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
              className={inputClass}
            />
            <button
              type="button"
              title={
                busy
                  ? t("promptsView.editor.creatingFolder")
                  : t("promptsView.editor.createFolder")
              }
              aria-label={
                busy
                  ? t("promptsView.editor.creatingFolder")
                  : t("promptsView.editor.createFolder")
              }
              disabled={busy}
              onClick={() => void submit()}
              className={iconButtonClass}
            >
              {busy ? (
                <LoaderCircleIcon
                  className="h-4 w-4 animate-spin"
                  aria-hidden="true"
                />
              ) : (
                <CheckIcon className="h-4 w-4" aria-hidden="true" />
              )}
            </button>
            <button
              type="button"
              title={t("promptsView.editor.cancelFolderCreate")}
              aria-label={t("promptsView.editor.cancelFolderCreate")}
              disabled={busy}
              onClick={cancel}
              className={iconButtonClass}
            >
              <XIcon className="h-4 w-4" aria-hidden="true" />
            </button>
          </div>
          {validationError && (
            <span
              id="prompt-folder-name-error"
              role="alert"
              className="text-xs text-destructive"
            >
              {t(validationError)}
            </span>
          )}
        </div>
      )}
    </div>
  );
}
