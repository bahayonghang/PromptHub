import { useState } from "react";
import { useTranslation } from "react-i18next";
import { CheckIcon, PencilIcon, TagsIcon, Trash2Icon, XIcon } from "lucide-react";

import { IconButton } from "../../../components/ui";

interface TagManagerProps {
  tags: string[];
  onRename: (old: string, next: string) => void;
  onDelete: (tag: string) => void;
}

export function TagManager({ tags, onRename, onDelete }: TagManagerProps) {
  const { t } = useTranslation();
  const [editing, setEditing] = useState<string | null>(null);
  const [value, setValue] = useState("");

  return (
    <details className="border-b border-border px-3 py-2">
      <summary className="flex cursor-pointer list-none items-center gap-1.5 text-label text-muted-foreground hover:text-foreground">
        <TagsIcon className="h-3.5 w-3.5" aria-hidden="true" />
        {t("promptsView.tags.manage")}
      </summary>
      <ul className="mt-2 max-h-40 space-y-1 overflow-y-auto">
        {tags.map((tag) => (
          <li key={tag} className="flex h-7 items-center gap-1">
            {editing === tag ? (
              <>
                <input
                  value={value}
                  onChange={(event) => setValue(event.target.value)}
                  aria-label={t("promptsView.tags.renameValue")}
                  className="min-w-0 flex-1 rounded-sm border border-input bg-background px-2 py-1 text-label text-foreground"
                />
                <IconButton
                  label={t("common.save")}
                  icon={<CheckIcon className="h-3.5 w-3.5" aria-hidden="true" />}
                  disabled={value.trim() === ""}
                  onClick={() => {
                    onRename(tag, value.trim());
                    setEditing(null);
                  }}
                />
                <IconButton
                  label={t("common.cancel")}
                  icon={<XIcon className="h-3.5 w-3.5" aria-hidden="true" />}
                  onClick={() => setEditing(null)}
                />
              </>
            ) : (
              <>
                <span className="min-w-0 flex-1 truncate text-label text-foreground">{tag}</span>
                <IconButton
                  label={t("promptsView.tags.rename")}
                  icon={<PencilIcon className="h-3.5 w-3.5" aria-hidden="true" />}
                  onClick={() => {
                    setEditing(tag);
                    setValue(tag);
                  }}
                />
                <IconButton
                  label={t("promptsView.tags.delete")}
                  icon={<Trash2Icon className="h-3.5 w-3.5" aria-hidden="true" />}
                  variant="danger"
                  onClick={() => onDelete(tag)}
                />
              </>
            )}
          </li>
        ))}
      </ul>
    </details>
  );
}
