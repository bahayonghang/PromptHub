import { useState } from "react";
import { useTranslation } from "react-i18next";
import { ImageIcon, PlusIcon, VideoIcon, XIcon } from "lucide-react";

import { IconButton } from "../../../components/ui";

interface MediaRefListProps {
  label: string;
  kind: "image" | "video";
  /** Stored media file-name references (Req 6.7). */
  refs: string[];
  onChange: (refs: string[]) => void;
}

/**
 * Edits a prompt's image or video file-name references (Req 6.7). References are
 * plain strings (the names the Media_Service stores); this control adds and
 * removes them without uploading. Media upload is handled elsewhere.
 */
export function MediaRefList({ label, kind, refs, onChange }: MediaRefListProps) {
  const { t } = useTranslation();
  const [value, setValue] = useState("");
  const Icon = kind === "image" ? ImageIcon : VideoIcon;

  const add = () => {
    const ref = value.trim();
    if (ref !== "" && !refs.includes(ref)) onChange([...refs, ref]);
    setValue("");
  };

  return (
    <div className="flex flex-col gap-1.5">
      <span className="flex items-center gap-1.5 text-label font-medium text-muted-foreground">
        <Icon className="h-3.5 w-3.5" aria-hidden="true" />
        {label}
      </span>
      {refs.length > 0 && (
        <ul className="flex flex-col gap-1">
          {refs.map((ref) => (
            <li
              key={ref}
              className="flex items-center gap-2 rounded-md border border-border bg-card px-2 py-1 text-label text-foreground"
            >
              <span className="min-w-0 flex-1 truncate">{ref}</span>
              <IconButton
                label={t("promptsView.editor.removeMediaRef")}
                icon={<XIcon className="h-3.5 w-3.5" aria-hidden="true" />}
                variant="danger"
                onClick={() => onChange(refs.filter((x) => x !== ref))}
              />
            </li>
          ))}
        </ul>
      )}
      <div className="flex items-center gap-2">
        <input
          value={value}
          placeholder={t("promptsView.editor.mediaRefPlaceholder")}
          onChange={(e) => setValue(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              e.preventDefault();
              add();
            }
          }}
          className="w-full rounded-md border border-input bg-background px-2 py-1 text-label text-foreground outline-none"
        />
        <IconButton
          label={kind === "image" ? t("promptsView.editor.addImage") : t("promptsView.editor.addVideo")}
          icon={<PlusIcon className="h-3.5 w-3.5" aria-hidden="true" />}
          variant="bordered"
          onClick={add}
        />
      </div>
    </div>
  );
}
