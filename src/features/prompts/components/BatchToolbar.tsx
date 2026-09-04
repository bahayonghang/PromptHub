import { useState } from "react";
import { useTranslation } from "react-i18next";
import { CheckCheckIcon, FolderInputIcon, TagIcon, Trash2Icon, XIcon } from "lucide-react";
import type { Folder } from "../types";

import { IconButton, Select} from "../../../components/ui";

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
      <div className="flex items-center gap-1 text-label text-muted-foreground">
        <span className="min-w-0 flex-1">
          {t("promptsView.batch.selected", { count: selectedCount })}
        </span>
        <IconButton
          label={t("promptsView.batch.selectPage")}
          icon={<CheckCheckIcon className="h-3.5 w-3.5" aria-hidden="true" />}
          onClick={onSelectPage}
        />
        <IconButton
          label={t("promptsView.batch.clear")}
          icon={<XIcon className="h-3.5 w-3.5" aria-hidden="true" />}
          onClick={onClear}
        />
        <button
          type="button"
          title={t("promptsView.chrome.batchExit")}
          aria-label={t("promptsView.chrome.batchExit")}
          onClick={onExit}
          className="rounded-sm px-1.5 py-1 text-label text-muted-foreground hover:bg-accent hover:text-foreground"
        >
          {t("promptsView.chrome.batchExit")}
        </button>
      </div>
      <div className="flex items-center gap-1">
        <Select
          value={folderId}
          onChange={(event) => setFolderId(event.target.value)}
          aria-label={t("promptsView.batch.folder")}
          wrapperClassName="min-w-0 flex-1"
        >
          <option value="">{t("promptsView.editor.noFolder")}</option>
          {folders.map((folder) => (
            <option key={folder.id} value={folder.id}>
              {folder.name}
            </option>
          ))}
        </Select>
        <IconButton
          label={t("promptsView.batch.move")}
          icon={<FolderInputIcon className="h-4 w-4" aria-hidden="true" />}
          onClick={() => onMove(folderId || null)}
        />
      </div>
      <div className="flex items-center gap-1">
        <input
          value={tag}
          onChange={(event) => setTag(event.target.value)}
          placeholder={t("promptsView.batch.tagPlaceholder")}
          className="min-w-0 flex-1 rounded-sm border border-input bg-background px-2 py-1 text-label text-foreground"
        />
        <IconButton
          label={t("promptsView.batch.addTag")}
          icon={<TagIcon className="h-4 w-4" aria-hidden="true" />}
          disabled={tag.trim() === ""}
          onClick={() => {
            onTag([tag.trim()]);
            setTag("");
          }}
        />
        <IconButton
          label={t("promptsView.batch.delete")}
          icon={<Trash2Icon className="h-4 w-4" aria-hidden="true" />}
          variant="danger"
          onClick={onDelete}
        />
      </div>
    </div>
  );
}
