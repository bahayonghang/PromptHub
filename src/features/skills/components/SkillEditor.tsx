import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { PlusIcon, SaveIcon, XIcon } from "lucide-react";
import type {
  CreateSkillInput,
  Skill,
  UpdateSkillInput,
} from "../types";

/** The editable draft backing the form, independent of the saved skill. */
interface Draft {
  name: string;
  description: string;
  version: string;
  author: string;
  content: string;
  tags: string[];
}

/** Builds a fresh draft from a saved skill, or a blank draft for creation. */
function toDraft(skill: Skill | null): Draft {
  return {
    name: skill?.name ?? "",
    description: skill?.description ?? "",
    version: skill?.version ?? "",
    author: skill?.author ?? "",
    content: skill?.content ?? "",
    tags: skill?.tags ?? [],
  };
}

interface SkillEditorProps {
  /** The skill being edited, or `null` when composing a new skill. */
  skill: Skill | null;
  /** Whether the editor is in create mode (no skill yet). */
  creating: boolean;
  knownTags: string[];
  onCreate: (input: CreateSkillInput) => void;
  onSave: (id: string, patch: UpdateSkillInput) => void;
  onCancelCreate: () => void;
}

/**
 * The skill editor form (Req 9.1, 9.4). Edits name, description, version label,
 * author, SKILL.md content, and tags against a local draft, then creates
 * (Req 9.1) or applies a partial update (Req 9.4). A non-empty name is required
 * (Req 9.11).
 */
export function SkillEditor({
  skill,
  creating,
  knownTags,
  onCreate,
  onSave,
  onCancelCreate,
}: SkillEditorProps) {
  const { t } = useTranslation();
  const [draft, setDraft] = useState<Draft>(() => toDraft(skill));
  const [tagInput, setTagInput] = useState("");

  // Reset the draft whenever the selected skill (or create mode) changes.
  const resetKey = creating ? "__new__" : skill?.id ?? "__none__";
  useEffect(() => {
    setDraft(toDraft(skill));
    setTagInput("");
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [resetKey]);

  const update = <K extends keyof Draft>(key: K, value: Draft[K]) =>
    setDraft((d) => ({ ...d, [key]: value }));

  const nameValid = draft.name.trim() !== "";

  const addTag = (raw: string) => {
    const tag = raw.trim();
    if (tag !== "" && !draft.tags.includes(tag)) {
      update("tags", [...draft.tags, tag]);
    }
    setTagInput("");
  };

  const submit = () => {
    if (!nameValid) return;
    if (creating) {
      const input: CreateSkillInput = {
        name: draft.name.trim(),
        description: draft.description || undefined,
        version: draft.version || undefined,
        author: draft.author || undefined,
        content: draft.content || undefined,
        tags: draft.tags,
      };
      onCreate(input);
    } else if (skill) {
      const patch: UpdateSkillInput = {
        name: draft.name.trim(),
        description: draft.description,
        version: draft.version,
        author: draft.author,
        content: draft.content,
        tags: draft.tags,
      };
      onSave(skill.id, patch);
    }
  };

  const labelClass = "text-xs font-medium text-muted-foreground";
  const inputClass =
    "w-full rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground outline-none focus:ring-1 focus:ring-ring";

  return (
    <form
      className="flex h-full flex-col"
      onSubmit={(e) => {
        e.preventDefault();
        submit();
      }}
    >
      <div className="flex flex-1 flex-col gap-4 overflow-y-auto p-4">
        {/* Name */}
        <div className="flex flex-col gap-1">
          <label className={labelClass} htmlFor="skill-name">
            {t("skillsView.editor.name")}
          </label>
          <input
            id="skill-name"
            value={draft.name}
            placeholder={t("skillsView.editor.namePlaceholder")}
            onChange={(e) => update("name", e.target.value)}
            className={inputClass}
          />
          {!nameValid && (
            <span className="text-xs text-destructive">
              {t("skillsView.editor.nameRequired")}
            </span>
          )}
        </div>

        {/* Version + Author */}
        <div className="grid grid-cols-2 gap-3">
          <div className="flex flex-col gap-1">
            <label className={labelClass} htmlFor="skill-version">
              {t("skillsView.editor.version")}
            </label>
            <input
              id="skill-version"
              value={draft.version}
              placeholder={t("skillsView.editor.versionPlaceholder")}
              onChange={(e) => update("version", e.target.value)}
              className={inputClass}
            />
          </div>
          <div className="flex flex-col gap-1">
            <label className={labelClass} htmlFor="skill-author">
              {t("skillsView.editor.author")}
            </label>
            <input
              id="skill-author"
              value={draft.author}
              placeholder={t("skillsView.editor.authorPlaceholder")}
              onChange={(e) => update("author", e.target.value)}
              className={inputClass}
            />
          </div>
        </div>

        {/* Description */}
        <div className="flex flex-col gap-1">
          <label className={labelClass} htmlFor="skill-description">
            {t("skillsView.editor.description")}
          </label>
          <input
            id="skill-description"
            value={draft.description}
            placeholder={t("skillsView.editor.descriptionPlaceholder")}
            onChange={(e) => update("description", e.target.value)}
            className={inputClass}
          />
        </div>

        {/* SKILL.md content */}
        <div className="flex flex-col gap-1">
          <label className={labelClass} htmlFor="skill-content">
            {t("skillsView.editor.content")}
          </label>
          <textarea
            id="skill-content"
            value={draft.content}
            placeholder={t("skillsView.editor.contentPlaceholder")}
            onChange={(e) => update("content", e.target.value)}
            rows={12}
            className={`${inputClass} resize-y font-mono`}
          />
        </div>

        {/* Tags */}
        <div className="flex flex-col gap-1.5">
          <label className={labelClass}>{t("skillsView.editor.tags")}</label>
          {draft.tags.length > 0 && (
            <div className="flex flex-wrap gap-1.5">
              {draft.tags.map((tag) => (
                <span
                  key={tag}
                  className="flex items-center gap-1 rounded-full bg-muted px-2 py-0.5 text-xs text-foreground"
                >
                  {tag}
                  <button
                    type="button"
                    aria-label={t("common.cancel")}
                    onClick={() =>
                      update(
                        "tags",
                        draft.tags.filter((x) => x !== tag),
                      )
                    }
                    className="text-muted-foreground hover:text-foreground"
                  >
                    <XIcon className="h-3 w-3" aria-hidden="true" />
                  </button>
                </span>
              ))}
            </div>
          )}
          <div className="flex items-center gap-2">
            <input
              value={tagInput}
              list="known-skill-tags"
              placeholder={t("skillsView.editor.addTagPlaceholder")}
              onChange={(e) => setTagInput(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") {
                  e.preventDefault();
                  addTag(tagInput);
                }
              }}
              className={inputClass}
            />
            <datalist id="known-skill-tags">
              {knownTags.map((tag) => (
                <option key={tag} value={tag} />
              ))}
            </datalist>
            <button
              type="button"
              onClick={() => addTag(tagInput)}
              className="flex shrink-0 items-center gap-1 rounded-md border border-input px-3 py-2 text-sm text-muted-foreground hover:bg-accent hover:text-foreground"
            >
              <PlusIcon className="h-4 w-4" aria-hidden="true" />
              {t("skillsView.editor.addTag")}
            </button>
          </div>
        </div>
      </div>

      {/* Footer actions */}
      <div className="flex shrink-0 items-center justify-end gap-2 border-t border-border px-4 py-3">
        {creating && (
          <button
            type="button"
            onClick={onCancelCreate}
            className="rounded-md border border-input px-4 py-2 text-sm text-muted-foreground hover:bg-accent hover:text-foreground"
          >
            {t("skillsView.editor.cancel")}
          </button>
        )}
        <button
          type="submit"
          disabled={!nameValid}
          className="flex items-center gap-2 rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground transition-opacity disabled:opacity-50"
        >
          <SaveIcon className="h-4 w-4" aria-hidden="true" />
          {creating ? t("skillsView.editor.create") : t("skillsView.editor.save")}
        </button>
      </div>
    </form>
  );
}
