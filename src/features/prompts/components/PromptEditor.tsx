import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { PlusIcon, SaveIcon, XIcon } from "lucide-react";
import {
  PROMPT_TYPES,
  type CreatePromptInput,
  type Folder,
  type Prompt,
  type PromptType,
  type UpdatePromptInput,
  type Variable,
} from "../types";
import { substituteVariables, syncVariables } from "../promptText";
import { MediaRefList } from "./MediaRefList";
import { VariableEditor } from "./VariableEditor";

/** The editable draft backing the form, independent of the saved prompt. */
interface Draft {
  title: string;
  description: string;
  promptType: PromptType;
  folderId: string | null;
  systemPrompt: string;
  userPrompt: string;
  variables: Variable[];
  tags: string[];
  images: string[];
  videos: string[];
  source: string;
  notes: string;
}

/** Builds a fresh draft from a saved prompt, or a blank draft for creation. */
function toDraft(prompt: Prompt | null): Draft {
  return {
    title: prompt?.title ?? "",
    description: prompt?.description ?? "",
    promptType: prompt?.promptType ?? "text",
    folderId: prompt?.folderId ?? null,
    systemPrompt: prompt?.systemPrompt ?? "",
    userPrompt: prompt?.userPrompt ?? "",
    variables: prompt?.variables ?? [],
    tags: prompt?.tags ?? [],
    images: prompt?.images ?? [],
    videos: prompt?.videos ?? [],
    source: prompt?.source ?? "",
    notes: prompt?.notes ?? "",
  };
}

interface PromptEditorProps {
  /** The prompt being edited, or `null` when composing a new prompt. */
  prompt: Prompt | null;
  /** Whether the editor is in create mode (no prompt yet). */
  creating: boolean;
  folders: Folder[];
  knownTags: string[];
  onCreate: (input: CreatePromptInput) => void;
  onSave: (id: string, patch: UpdatePromptInput) => void;
  onCancelCreate: () => void;
}

/**
 * The prompt editor form (Req 6.1, 6.4, 6.7). Edits title, description, type,
 * folder, system/user prompt, variables, tags, media references, source, and
 * notes against a local draft, then creates (Req 6.1) or applies a partial
 * update (Req 6.4). Variables stay reconciled with the `{{name}}` placeholders
 * in the prompt text, and a live preview substitutes sample values (Req 6.11).
 */
export function PromptEditor({
  prompt,
  creating,
  folders,
  knownTags,
  onCreate,
  onSave,
  onCancelCreate,
}: PromptEditorProps) {
  const { t } = useTranslation();
  const [draft, setDraft] = useState<Draft>(() => toDraft(prompt));
  const [tagInput, setTagInput] = useState("");
  const [previewValues, setPreviewValues] = useState<Record<string, string>>({});

  // Reset the draft whenever the selected prompt (or create mode) changes.
  const resetKey = creating ? "__new__" : prompt?.id ?? "__none__";
  useEffect(() => {
    setDraft(toDraft(prompt));
    setTagInput("");
    setPreviewValues({});
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [resetKey]);

  const update = <K extends keyof Draft>(key: K, value: Draft[K]) =>
    setDraft((d) => ({ ...d, [key]: value }));

  /** Keeps the variable list in sync with placeholders when prompt text changes. */
  const updateText = (key: "systemPrompt" | "userPrompt", value: string) => {
    setDraft((d) => {
      const next = { ...d, [key]: value };
      next.variables = syncVariables(d.variables, next.systemPrompt, next.userPrompt);
      return next;
    });
  };

  const titleValid = draft.title.trim() !== "";
  const userPromptValid = draft.userPrompt.trim() !== "";
  const canSubmit = titleValid && userPromptValid;

  const addTag = (raw: string) => {
    const tag = raw.trim();
    if (tag !== "" && !draft.tags.includes(tag)) {
      update("tags", [...draft.tags, tag]);
    }
    setTagInput("");
  };

  const previewText = useMemo(
    () => substituteVariables(draft.userPrompt, previewValues),
    [draft.userPrompt, previewValues],
  );

  const submit = () => {
    if (!canSubmit) return;
    if (creating) {
      const input: CreatePromptInput = {
        title: draft.title.trim(),
        userPrompt: draft.userPrompt,
        promptType: draft.promptType,
        description: draft.description || undefined,
        systemPrompt: draft.systemPrompt || undefined,
        variables: draft.variables,
        tags: draft.tags,
        folderId: draft.folderId,
        images: draft.images,
        videos: draft.videos,
        source: draft.source || undefined,
        notes: draft.notes || undefined,
      };
      onCreate(input);
    } else if (prompt) {
      const patch: UpdatePromptInput = {
        title: draft.title.trim(),
        description: draft.description,
        promptType: draft.promptType,
        systemPrompt: draft.systemPrompt,
        userPrompt: draft.userPrompt,
        variables: draft.variables,
        tags: draft.tags,
        folderId: draft.folderId,
        images: draft.images,
        videos: draft.videos,
        source: draft.source,
        notes: draft.notes,
      };
      onSave(prompt.id, patch);
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
        {/* Title */}
        <div className="flex flex-col gap-1">
          <label className={labelClass} htmlFor="prompt-title">
            {t("promptsView.editor.title")}
          </label>
          <input
            id="prompt-title"
            value={draft.title}
            placeholder={t("promptsView.editor.titlePlaceholder")}
            onChange={(e) => update("title", e.target.value)}
            className={inputClass}
          />
          {!titleValid && (
            <span className="text-xs text-destructive">
              {t("promptsView.editor.titleRequired")}
            </span>
          )}
        </div>

        {/* Type + Folder */}
        <div className="grid grid-cols-2 gap-3">
          <div className="flex flex-col gap-1">
            <label className={labelClass} htmlFor="prompt-type">
              {t("promptsView.editor.type")}
            </label>
            <select
              id="prompt-type"
              value={draft.promptType}
              onChange={(e) => update("promptType", e.target.value as PromptType)}
              className={inputClass}
            >
              {PROMPT_TYPES.map((type) => (
                <option key={type} value={type}>
                  {t(`promptsView.editor.type${type[0].toUpperCase()}${type.slice(1)}`)}
                </option>
              ))}
            </select>
          </div>
          <div className="flex flex-col gap-1">
            <label className={labelClass} htmlFor="prompt-folder">
              {t("promptsView.editor.folder")}
            </label>
            <select
              id="prompt-folder"
              value={draft.folderId ?? ""}
              onChange={(e) => update("folderId", e.target.value || null)}
              className={inputClass}
            >
              <option value="">{t("promptsView.editor.noFolder")}</option>
              {folders.map((folder) => (
                <option key={folder.id} value={folder.id}>
                  {folder.name}
                </option>
              ))}
            </select>
          </div>
        </div>

        {/* Description */}
        <div className="flex flex-col gap-1">
          <label className={labelClass} htmlFor="prompt-description">
            {t("promptsView.editor.description")}
          </label>
          <input
            id="prompt-description"
            value={draft.description}
            placeholder={t("promptsView.editor.descriptionPlaceholder")}
            onChange={(e) => update("description", e.target.value)}
            className={inputClass}
          />
        </div>

        {/* System prompt */}
        <div className="flex flex-col gap-1">
          <label className={labelClass} htmlFor="prompt-system">
            {t("promptsView.editor.systemPrompt")}
          </label>
          <textarea
            id="prompt-system"
            value={draft.systemPrompt}
            placeholder={t("promptsView.editor.systemPromptPlaceholder")}
            onChange={(e) => updateText("systemPrompt", e.target.value)}
            rows={3}
            className={`${inputClass} resize-y`}
          />
        </div>

        {/* User prompt */}
        <div className="flex flex-col gap-1">
          <label className={labelClass} htmlFor="prompt-user">
            {t("promptsView.editor.userPrompt")}
          </label>
          <textarea
            id="prompt-user"
            value={draft.userPrompt}
            placeholder={t("promptsView.editor.userPromptPlaceholder")}
            onChange={(e) => updateText("userPrompt", e.target.value)}
            rows={6}
            className={`${inputClass} resize-y font-mono`}
          />
          {!userPromptValid && (
            <span className="text-xs text-destructive">
              {t("promptsView.editor.userPromptRequired")}
            </span>
          )}
        </div>

        {/* Variables */}
        <VariableEditor
          variables={draft.variables}
          onChange={(variables) => update("variables", variables)}
        />

        {/* Preview */}
        {draft.variables.length > 0 && (
          <div className="flex flex-col gap-2 rounded-lg border border-border bg-card p-3">
            <span className={labelClass}>{t("promptsView.editor.preview")}</span>
            <div className="grid grid-cols-2 gap-2">
              {draft.variables.map((variable) => (
                <input
                  key={variable.name}
                  value={previewValues[variable.name] ?? ""}
                  placeholder={variable.label || variable.name}
                  onChange={(e) =>
                    setPreviewValues((v) => ({
                      ...v,
                      [variable.name]: e.target.value,
                    }))
                  }
                  className="rounded-md border border-input bg-background px-2 py-1 text-xs text-foreground outline-none focus:ring-1 focus:ring-ring"
                />
              ))}
            </div>
            <pre className="whitespace-pre-wrap rounded-md bg-muted p-2 text-xs text-foreground">
              {previewText}
            </pre>
          </div>
        )}

        {/* Tags */}
        <div className="flex flex-col gap-1.5">
          <label className={labelClass}>{t("promptsView.editor.tags")}</label>
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
              list="known-tags"
              placeholder={t("promptsView.editor.addTagPlaceholder")}
              onChange={(e) => setTagInput(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") {
                  e.preventDefault();
                  addTag(tagInput);
                }
              }}
              className={inputClass}
            />
            <datalist id="known-tags">
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
              {t("promptsView.editor.addTag")}
            </button>
          </div>
        </div>

        {/* Media references */}
        <MediaRefList
          label={t("promptsView.editor.images")}
          kind="image"
          refs={draft.images}
          onChange={(images) => update("images", images)}
        />
        <MediaRefList
          label={t("promptsView.editor.videos")}
          kind="video"
          refs={draft.videos}
          onChange={(videos) => update("videos", videos)}
        />

        {/* Source + Notes */}
        <div className="flex flex-col gap-1">
          <label className={labelClass} htmlFor="prompt-source">
            {t("promptsView.editor.source")}
          </label>
          <input
            id="prompt-source"
            value={draft.source}
            placeholder={t("promptsView.editor.sourcePlaceholder")}
            onChange={(e) => update("source", e.target.value)}
            className={inputClass}
          />
        </div>
        <div className="flex flex-col gap-1">
          <label className={labelClass} htmlFor="prompt-notes">
            {t("promptsView.editor.notes")}
          </label>
          <textarea
            id="prompt-notes"
            value={draft.notes}
            placeholder={t("promptsView.editor.notesPlaceholder")}
            onChange={(e) => update("notes", e.target.value)}
            rows={2}
            className={`${inputClass} resize-y`}
          />
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
            {t("promptsView.editor.cancel")}
          </button>
        )}
        <button
          type="submit"
          disabled={!canSubmit}
          className="flex items-center gap-2 rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground transition-opacity disabled:opacity-50"
        >
          <SaveIcon className="h-4 w-4" aria-hidden="true" />
          {creating ? t("promptsView.editor.create") : t("promptsView.editor.save")}
        </button>
      </div>
    </form>
  );
}
