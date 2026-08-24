import { useState } from "react";
import { useTranslation } from "react-i18next";
import { CheckCheckIcon, FolderInputIcon, TagIcon, Trash2Icon, XIcon } from "lucide-react";
import type { Folder } from "../types";

interface BatchToolbarProps {
  selectedCount: number;
  folders: Folder[];
  onSelectPage: () => void;
  onClear: () => void;
  onMove: (folderId: string | null) => void;
  onTag: (tags: string[]) => void;
  onDelete: () => void;
  onExit: () => void;
}

export function BatchToolbar({
  selectedCount,
  folders,
  onSelectPage,
  onClear,
  onMove,
  onTag,
  onDelete,
  onExit,
}: BatchToolbarProps) {
  const { t } = useTranslation();
  const [folderId, setFolderId] = useState("");
  const [tag, setTag] = useState("");

  return (
    <div className="flex flex-col gap-2 border-b border-border bg-muted/40 p-2">
      <div className="flex items-center gap-1 text-xs text-muted-foreground">
        <span className="min-w-0 flex-1">
          {t("promptsView.batch.selected", { count: selectedCount })}
        </span>
        <button
          type="button"
          title={t("promptsView.batch.selectPage")}
          aria-label={t("promptsView.batch.selectPage")}
          onClick={onSelectPage}
          className="rounded p-1 hover:bg-accent hover:text-foreground"
        >
          <CheckCheckIcon className="h-3.5 w-3.5" aria-hidden="true" />
        </button>
        <button
          type="button"
          title={t("promptsView.batch.clear")}
          aria-label={t("promptsView.batch.clear")}
          onClick={onClear}
          className="rounded p-1 hover:bg-accent hover:text-foreground"
        >
          <XIcon className="h-3.5 w-3.5" aria-hidden="true" />
        </button>
        <button
          type="button"
          title={t("promptsView.chrome.batchExit")}
          aria-label={t("promptsView.chrome.batchExit")}
          onClick={onExit}
          className="rounded px-1.5 py-1 text-xs text-muted-foreground hover:bg-accent hover:text-foreground"
        >
          {t("promptsView.chrome.batchExit")}
        </button>
      </div>
      <div className="flex items-center gap-1">
        <select
          value={folderId}
          onChange={(event) => setFolderId(event.target.value)}
          aria-label={t("promptsView.batch.folder")}
          className="min-w-0 flex-1 rounded border border-input bg-background px-2 py-1 text-xs text-foreground"
        >
          <option value="">{t("promptsView.editor.noFolder")}</option>
          {folders.map((folder) => (
            <option key={folder.id} value={folder.id}>
              {folder.name}
            </option>
          ))}
        </select>
        <button
          type="button"
          title={t("promptsView.batch.move")}
          aria-label={t("promptsView.batch.move")}
          onClick={() => onMove(folderId || null)}
          className="rounded p-1.5 text-muted-foreground hover:bg-accent hover:text-foreground"
        >
          <FolderInputIcon className="h-4 w-4" aria-hidden="true" />
        </button>
      </div>
      <div className="flex items-center gap-1">
        <input
          value={tag}
          onChange={(event) => setTag(event.target.value)}
          placeholder={t("promptsView.batch.tagPlaceholder")}
          className="min-w-0 flex-1 rounded border border-input bg-background px-2 py-1 text-xs text-foreground"
        />
        <button
          type="button"
          title={t("promptsView.batch.addTag")}
          aria-label={t("promptsView.batch.addTag")}
          disabled={tag.trim() === ""}
          onClick={() => {
            onTag([tag.trim()]);
            setTag("");
          }}
          className="rounded p-1.5 text-muted-foreground hover:bg-accent hover:text-foreground disabled:opacity-40"
        >
          <TagIcon className="h-4 w-4" aria-hidden="true" />
        </button>
        <button
          type="button"
          title={t("promptsView.batch.delete")}
          aria-label={t("promptsView.batch.delete")}
          onClick={onDelete}
          className="rounded p-1.5 text-muted-foreground hover:bg-destructive/15 hover:text-destructive"
        >
          <Trash2Icon className="h-4 w-4" aria-hidden="true" />
        </button>
      </div>
    </div>
  );
}
