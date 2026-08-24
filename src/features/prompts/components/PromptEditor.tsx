import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  ArrowDownIcon,
  ArrowUpIcon,
  CheckIcon,
  FolderPlusIcon,
  LoaderCircleIcon,
  MessageSquareIcon,
  PlusIcon,
  SaveIcon,
  TextIcon,
  Trash2Icon,
  XIcon,
} from "lucide-react";
import {
  PROMPT_TYPES,
  type CreateFolderInput,
  type CreatePromptInput,
  type CreatePromptTypeInput,
  type Folder,
  type Prompt,
  type PromptMessage,
  type PromptMessageRole,
  type PromptType,
  type PromptTypeDefinition,
  type UpdatePromptInput,
  type Variable,
} from "../types";
import {
  preferredChatMode,
  setPreferredChatMode,
} from "../definitionMode";
import {
  deriveTextFieldsFromMessages,
  seedChatMessages,
  substituteVariables,
  syncVariables,
} from "../promptText";
import { CopyPromptButton } from "./CopyPromptButton";
import { MediaRefList } from "./MediaRefList";
import { VariableEditor } from "./VariableEditor";

/** The editable draft backing the form, independent of the saved prompt. */
interface Draft {
  title: string;
  description: string;
  promptType: PromptType;
  typeDefinitionId: string | null;
  folderId: string | null;
  systemPrompt: string;
  userPrompt: string;
  messages: PromptMessage[];
  variables: Variable[];
  tags: string[];
  images: string[];
  videos: string[];
  source: string;
  notes: string;
  isPrivate: boolean;
}

/** Builds a fresh draft from a saved prompt, or a blank draft for creation. */
function toDraft(prompt: Prompt | null): Draft {
  return {
    title: prompt?.title ?? "",
    description: prompt?.description ?? "",
    promptType: prompt?.promptType ?? "text",
    typeDefinitionId: prompt?.typeDefinitionId ?? null,
    folderId: prompt?.folderId ?? null,
    systemPrompt: prompt?.systemPrompt ?? "",
    userPrompt: prompt?.userPrompt ?? "",
    messages:
      prompt != null && prompt.messages.length > 0
        ? prompt.messages
        : preferredChatMode()
          ? seedChatMessages(prompt?.systemPrompt, prompt?.userPrompt ?? "")
          : [],
    variables: prompt?.variables ?? [],
    tags: prompt?.tags ?? [],
    images: prompt?.images ?? [],
    videos: prompt?.videos ?? [],
    source: prompt?.source ?? "",
    notes: prompt?.notes ?? "",
    isPrivate: prompt?.isPrivate ?? false,
  };
}

interface PromptEditorProps {
  /** The prompt being edited, or `null` when composing a new prompt. */
  prompt: Prompt | null;
  /** Whether the editor is in create mode (no prompt yet). */
  creating: boolean;
  folders: Folder[];
  promptTypeDefinitions: PromptTypeDefinition[];
  knownTags: string[];
  onCreate: (input: CreatePromptInput) => void;
  onSave: (id: string, patch: UpdatePromptInput) => void;
  onCancelCreate: () => void;
  onCreateFolder: (input: CreateFolderInput) => Promise<Folder | null>;
  onCreatePromptType: (
    input: CreatePromptTypeInput,
  ) => Promise<PromptTypeDefinition | null>;
  writeText?: (text: string) => Promise<void>;
}

interface FolderPickerProps {
  folders: Folder[];
  value: string | null;
  onChange: (folderId: string | null) => void;
  onCreateFolder: (input: CreateFolderInput) => Promise<Folder | null>;
}

function FolderPicker({
  folders,
  value,
  onChange,
  onCreateFolder,
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
              aria-describedby={validationError ? "prompt-folder-name-error" : undefined}
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

interface PromptTypePickerProps {
  definitions: PromptTypeDefinition[];
  baseKind: PromptType;
  definitionId: string | null;
  onChange: (baseKind: PromptType, definitionId: string | null) => void;
  onCreate: (
    input: CreatePromptTypeInput,
  ) => Promise<PromptTypeDefinition | null>;
}

function PromptTypePicker({
  definitions,
  baseKind,
  definitionId,
  onChange,
  onCreate,
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
    "min-w-0 w-full rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground outline-none focus-visible:ring-2 focus-visible:ring-ring";
  const iconButtonClass =
    "flex h-9 w-9 shrink-0 items-center justify-center rounded-md border border-input text-muted-foreground hover:bg-accent hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50";
  const value = definitionId ? `custom:${definitionId}` : `base:${baseKind}`;

  return (
    <div className="flex flex-col gap-1.5">
      <label className="text-xs font-medium text-muted-foreground" htmlFor="prompt-type">
        {t("promptsView.editor.type")}
      </label>
      <div className="grid grid-cols-[minmax(0,1fr)_2.25rem] gap-2">
        <select
          ref={selectRef}
          id="prompt-type"
          value={value}
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
          className={inputClass}
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
        </select>
        <button
          ref={createButtonRef}
          type="button"
          title={t("promptsView.editor.newType")}
          aria-label={t("promptsView.editor.newType")}
          aria-expanded={creating}
          onClick={() => {
            setCreating(true);
            setNewBaseKind(baseKind);
            setValidationError(null);
          }}
          className={iconButtonClass}
        >
          <PlusIcon className="h-4 w-4" aria-hidden="true" />
        </button>
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
              aria-describedby={validationError ? "prompt-type-name-error" : undefined}
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
            <select
              value={newBaseKind}
              aria-label={t("promptsView.editor.baseType")}
              disabled={busy}
              onChange={(event) => setNewBaseKind(event.target.value as PromptType)}
              className={inputClass}
            >
              {PROMPT_TYPES.map((type) => (
                <option key={type} value={type}>
                  {t(
                    `promptsView.editor.type${type[0].toUpperCase()}${type.slice(1)}`,
                  )}
                </option>
              ))}
            </select>
            <button
              type="button"
              title={busy ? t("promptsView.editor.creatingType") : t("promptsView.editor.createType")}
              aria-label={busy ? t("promptsView.editor.creatingType") : t("promptsView.editor.createType")}
              disabled={busy}
              onClick={() => void submit()}
              className={iconButtonClass}
            >
              {busy ? (
                <LoaderCircleIcon className="h-4 w-4 animate-spin" aria-hidden="true" />
              ) : (
                <CheckIcon className="h-4 w-4" aria-hidden="true" />
              )}
            </button>
            <button
              type="button"
              title={t("promptsView.editor.cancelTypeCreate")}
              aria-label={t("promptsView.editor.cancelTypeCreate")}
              disabled={busy}
              onClick={cancel}
              className={iconButtonClass}
            >
              <XIcon className="h-4 w-4" aria-hidden="true" />
            </button>
          </div>
          {validationError && (
            <span id="prompt-type-name-error" role="alert" className="text-xs text-destructive">
              {t(validationError)}
            </span>
          )}
        </div>
      )}
    </div>
  );
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
  promptTypeDefinitions,
  knownTags,
  onCreate,
  onSave,
  onCancelCreate,
  onCreateFolder,
  onCreatePromptType,
  writeText,
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

  const updateMessages = (messages: PromptMessage[]) => {
    setDraft((current) => ({
      ...current,
      messages,
      variables: syncVariables(
        current.variables,
        "",
        messages.map((message) => message.content).join("\n"),
      ),
    }));
  };

  const chatMode = draft.messages.length > 0;

  const setChatMode = (enabled: boolean) => {
    setPreferredChatMode(enabled);
    if (enabled) {
      const messages: PromptMessage[] = [];
      if (draft.systemPrompt.trim() !== "") {
        messages.push({ role: "system", content: draft.systemPrompt });
      }
      messages.push({ role: "user", content: draft.userPrompt });
      updateMessages(messages);
      return;
    }
    const system = draft.messages.find((message) => message.role === "system");
    const user = [...draft.messages]
      .reverse()
      .find((message) => message.role === "user");
    setDraft((current) => ({
      ...current,
      systemPrompt: system?.content ?? current.systemPrompt,
      userPrompt: user?.content ?? current.userPrompt,
      messages: [],
      variables: syncVariables(
        current.variables,
        system?.content ?? current.systemPrompt,
        user?.content ?? current.userPrompt,
      ),
    }));
  };

  const titleValid = draft.title.trim() !== "";
  const userPromptValid = chatMode
    ? draft.messages.length > 0 &&
      draft.messages.every((message) => message.content.trim() !== "")
    : draft.userPrompt.trim() !== "";
  const canSubmit = titleValid && userPromptValid;

  const addTag = (raw: string) => {
    const tag = raw.trim();
    if (tag !== "" && !draft.tags.includes(tag)) {
      update("tags", [...draft.tags, tag]);
    }
    setTagInput("");
  };

  const previewText = useMemo(
    () =>
      chatMode
        ? draft.messages
            .map(
              (message) =>
                `${message.role}: ${substituteVariables(message.content, previewValues)}`,
            )
            .join("\n\n")
        : substituteVariables(draft.userPrompt, previewValues),
    [chatMode, draft.messages, draft.userPrompt, previewValues],
  );

  const submit = () => {
    if (!canSubmit) return;
    const textFields = chatMode
      ? deriveTextFieldsFromMessages(draft.messages)
      : {
          systemPrompt: draft.systemPrompt,
          userPrompt: draft.userPrompt,
        };
    if (creating) {
      const input: CreatePromptInput = {
        title: draft.title.trim(),
        userPrompt: textFields.userPrompt,
        promptType: draft.promptType,
        typeDefinitionId: draft.typeDefinitionId,
        description: draft.description || undefined,
        systemPrompt: textFields.systemPrompt || undefined,
        messages: draft.messages,
        variables: draft.variables,
        tags: draft.tags,
        folderId: draft.folderId,
        images: draft.images,
        videos: draft.videos,
        source: draft.source || undefined,
        notes: draft.notes || undefined,
        isPrivate: draft.isPrivate,
      };
      onCreate(input);
    } else if (prompt) {
      const patch: UpdatePromptInput = {
        title: draft.title.trim(),
        description: draft.description,
        promptType: draft.promptType,
        typeDefinitionId: draft.typeDefinitionId,
        systemPrompt: textFields.systemPrompt,
        userPrompt: textFields.userPrompt,
        messages: draft.messages,
        variables: draft.variables,
        tags: draft.tags,
        folderId: draft.folderId,
        images: draft.images,
        videos: draft.videos,
        source: draft.source,
        notes: draft.notes,
        isPrivate: draft.isPrivate,
      };
      onSave(prompt.id, patch);
    }
  };

  const labelClass = "text-xs font-medium text-muted-foreground";
  const inputClass =
    "w-full rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground outline-none focus-visible:ring-2 focus-visible:ring-ring";

  return (
    <form
      className="prompt-editor flex h-full min-h-0 flex-col"
      onSubmit={(e) => {
        e.preventDefault();
        submit();
      }}
    >
      <div className="prompt-editor__body min-h-0 flex-1 overflow-y-auto px-4 py-5">
        <div className="flex w-full flex-col gap-6">
          <section aria-labelledby="prompt-editor-basics">
            <h3
              id="prompt-editor-basics"
              className="text-sm font-semibold text-foreground"
            >
              {t("promptsView.editor.sections.identity")}
            </h3>
            <div className="prompt-editor__two-column mt-3 grid grid-cols-1 gap-4">
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

              <PromptTypePicker
                definitions={promptTypeDefinitions}
                baseKind={draft.promptType}
                definitionId={draft.typeDefinitionId}
                onChange={(promptType, typeDefinitionId) =>
                  setDraft((current) => ({
                    ...current,
                    promptType,
                    typeDefinitionId,
                  }))
                }
                onCreate={onCreatePromptType}
              />

              <label className="flex items-center gap-2 self-end py-2 text-sm text-foreground">
                <input
                  type="checkbox"
                  checked={draft.isPrivate}
                  onChange={(event) =>
                    update("isPrivate", event.target.checked)
                  }
                  className="h-4 w-4 shrink-0 rounded border-input text-primary focus:ring-ring"
                />
                <span>
                  {t("promptsView.editor.privatePrompt")}
                  <span className="block text-xs text-muted-foreground">
                    {t("promptsView.editor.privatePromptHint")}
                  </span>
                </span>
              </label>
            </div>
          </section>

          <section
            aria-labelledby="prompt-editor-definition"
            className="border-t border-border pt-5"
          >
            <div className="flex flex-wrap items-center justify-between gap-3">
              <h3
                id="prompt-editor-definition"
                className="text-sm font-semibold text-foreground"
              >
                {t("promptsView.editor.sections.definition")}
              </h3>
              <div className="flex items-center gap-1">
                <CopyPromptButton
                  source={{
                    systemPrompt: draft.systemPrompt,
                    userPrompt: draft.userPrompt,
                    messages: draft.messages,
                    variables: draft.variables,
                  }}
                  promptId={prompt?.id}
                  writeText={writeText}
                />
                <div
                  role="group"
                  aria-label={t("evaluation.definitionMode")}
                  className="flex rounded-md border border-input p-0.5"
                >
                  <button
                    type="button"
                    aria-pressed={!chatMode}
                    onClick={() => setChatMode(false)}
                    className={`flex min-h-8 items-center gap-1.5 rounded px-2 text-xs focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring ${
                      !chatMode
                        ? "bg-accent text-foreground"
                        : "text-muted-foreground"
                    }`}
                  >
                    <TextIcon className="h-3.5 w-3.5" aria-hidden="true" />
                    {t("evaluation.textMode")}
                  </button>
                  <button
                    type="button"
                    aria-pressed={chatMode}
                    onClick={() => setChatMode(true)}
                    className={`flex min-h-8 items-center gap-1.5 rounded px-2 text-xs focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring ${
                      chatMode
                        ? "bg-accent text-foreground"
                        : "text-muted-foreground"
                    }`}
                  >
                    <MessageSquareIcon
                      className="h-3.5 w-3.5"
                      aria-hidden="true"
                    />
                    {t("evaluation.chatMode")}
                  </button>
                </div>
              </div>
            </div>

            <div className="mt-3 flex flex-col gap-4">
              {chatMode ? (
                <div className="flex flex-col gap-2">
                  {draft.messages.map((message, index) => (
                    <div
                      key={`${index}-${message.role}`}
                      className="prompt-editor__message grid grid-cols-1 items-start gap-2 border-b border-border py-3 last:border-b-0"
                    >
                      <select
                        value={message.role}
                        aria-label={t("evaluation.messageRole", {
                          index: index + 1,
                        })}
                        onChange={(event) => {
                          const next = [...draft.messages];
                          next[index] = {
                            ...message,
                            role: event.target.value as PromptMessageRole,
                          };
                          updateMessages(next);
                        }}
                        className="prompt-editor__message-role w-full min-w-[6.5rem] rounded-md border border-input bg-background px-2 py-2 text-xs text-foreground"
                      >
                        <option value="system">
                          {t("evaluation.roleSystem")}
                        </option>
                        <option value="user">{t("evaluation.roleUser")}</option>
                        <option value="assistant">
                          {t("evaluation.roleAssistant")}
                        </option>
                      </select>
                      <textarea
                        value={message.content}
                        aria-label={t("evaluation.messageContent", {
                          index: index + 1,
                        })}
                        onChange={(event) => {
                          const next = [...draft.messages];
                          next[index] = {
                            ...message,
                            content: event.target.value,
                          };
                          updateMessages(next);
                        }}
                        rows={draft.messages.length > 1 ? 8 : 16}
                        className={`${inputClass} prompt-editor__message-body resize-y font-mono ${
                          draft.messages.length > 1
                            ? "prompt-editor__message-body--compact"
                            : ""
                        }`}
                      />
                      <div className="prompt-editor__message-actions flex gap-1">
                        {[ArrowUpIcon, ArrowDownIcon, Trash2Icon].map(
                          (Icon, action) => (
                            <button
                              key={action}
                              type="button"
                              disabled={
                                (action === 0 && index === 0) ||
                                (action === 1 &&
                                  index === draft.messages.length - 1) ||
                                (action === 2 && draft.messages.length === 1)
                              }
                              title={t(
                                action === 0
                                  ? "evaluation.moveMessageUp"
                                  : action === 1
                                    ? "evaluation.moveMessageDown"
                                    : "evaluation.removeMessage",
                              )}
                              aria-label={t(
                                action === 0
                                  ? "evaluation.moveMessageUp"
                                  : action === 1
                                    ? "evaluation.moveMessageDown"
                                    : "evaluation.removeMessage",
                              )}
                              onClick={() => {
                                const next = [...draft.messages];
                                if (action === 2) next.splice(index, 1);
                                else {
                                  const target =
                                    action === 0 ? index - 1 : index + 1;
                                  [next[index], next[target]] = [
                                    next[target],
                                    next[index],
                                  ];
                                }
                                updateMessages(next);
                              }}
                              className="flex h-8 w-8 items-center justify-center rounded-md text-muted-foreground hover:bg-accent hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-30"
                            >
                              <Icon
                                className="h-3.5 w-3.5"
                                aria-hidden="true"
                              />
                            </button>
                          ),
                        )}
                      </div>
                    </div>
                  ))}
                  <button
                    type="button"
                    onClick={() =>
                      updateMessages([
                        ...draft.messages,
                        { role: "user", content: "" },
                      ])
                    }
                    className="flex min-h-8 w-fit items-center gap-1.5 rounded-md border border-input px-2.5 text-xs text-muted-foreground hover:bg-accent hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                  >
                    <PlusIcon className="h-3.5 w-3.5" aria-hidden="true" />
                    {t("evaluation.addMessage")}
                  </button>
                </div>
              ) : (
                <>
                  <div className="flex flex-col gap-1">
                    <label className={labelClass} htmlFor="prompt-system">
                      {t("promptsView.editor.systemPrompt")}
                    </label>
                    <textarea
                      id="prompt-system"
                      value={draft.systemPrompt}
                      placeholder={t(
                        "promptsView.editor.systemPromptPlaceholder",
                      )}
                      onChange={(e) =>
                        updateText("systemPrompt", e.target.value)
                      }
                      rows={4}
                      className={`${inputClass} resize-y`}
                    />
                  </div>
                  <div className="flex flex-col gap-1">
                    <label className={labelClass} htmlFor="prompt-user">
                      {t("promptsView.editor.userPrompt")}
                    </label>
                    <textarea
                      id="prompt-user"
                      value={draft.userPrompt}
                      placeholder={t(
                        "promptsView.editor.userPromptPlaceholder",
                      )}
                      onChange={(e) => updateText("userPrompt", e.target.value)}
                      rows={8}
                      className={`${inputClass} resize-y font-mono`}
                    />
                  </div>
                </>
              )}
              {!userPromptValid && (
                <span className="text-xs text-destructive">
                  {t("promptsView.editor.userPromptRequired")}
                </span>
              )}

              <VariableEditor
                variables={draft.variables}
                onChange={(variables) => update("variables", variables)}
              />

              {draft.variables.length > 0 && (
                <div className="flex flex-col gap-3 border-y border-border bg-muted/30 py-3">
                  <span className={labelClass}>
                    {t("promptsView.editor.preview")}
                  </span>
                  <div className="prompt-editor__preview-grid grid grid-cols-1 gap-2">
                    {draft.variables.map((variable) => (
                      <input
                        key={variable.name}
                        value={previewValues[variable.name] ?? ""}
                        aria-label={variable.label || variable.name}
                        placeholder={variable.label || variable.name}
                        onChange={(e) =>
                          setPreviewValues((v) => ({
                            ...v,
                            [variable.name]: e.target.value,
                          }))
                        }
                        className="rounded-md border border-input bg-background px-2 py-2 text-xs text-foreground outline-none focus-visible:ring-2 focus-visible:ring-ring"
                      />
                    ))}
                  </div>
                  <pre className="max-w-full whitespace-pre-wrap break-words bg-muted px-3 py-2 text-xs text-foreground">
                    {previewText}
                  </pre>
                </div>
              )}
            </div>
          </section>

          <section
            aria-labelledby="prompt-editor-organization"
            className="border-t border-border pt-5"
          >
            <h3
              id="prompt-editor-organization"
              className="text-sm font-semibold text-foreground"
            >
              {t("promptsView.editor.sections.organization")}
            </h3>
            <div className="prompt-editor__organization mt-3 grid grid-cols-1 gap-4">
              <FolderPicker
                key={resetKey}
                folders={folders}
                value={draft.folderId}
                onChange={(folderId) => update("folderId", folderId)}
                onCreateFolder={onCreateFolder}
              />

              <div className="flex min-w-0 flex-col gap-1.5">
                <label
                  className={labelClass}
                  htmlFor="prompt-tag"
                >
                  {t("promptsView.editor.tags")}
                </label>
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
                          className="rounded text-muted-foreground hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                        >
                          <XIcon className="h-3 w-3" aria-hidden="true" />
                        </button>
                      </span>
                    ))}
                  </div>
                )}
                <div className="prompt-editor__tag-row flex min-w-0 flex-col gap-2">
                  <input
                    id="prompt-tag"
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
                    className="flex min-h-9 shrink-0 items-center justify-center gap-1 rounded-md border border-input px-3 text-sm text-muted-foreground hover:bg-accent hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                  >
                    <PlusIcon className="h-4 w-4" aria-hidden="true" />
                    {t("promptsView.editor.addTag")}
                  </button>
                </div>
              </div>
            </div>
          </section>

          <section
            aria-labelledby="prompt-editor-references"
            className="border-t border-border pt-5"
          >
            <h3
              id="prompt-editor-references"
              className="text-sm font-semibold text-foreground"
            >
              {t("promptsView.editor.sections.references")}
            </h3>
            <div className="prompt-editor__two-column mt-3 grid grid-cols-1 gap-4">
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
                  rows={3}
                  className={`${inputClass} resize-y`}
                />
              </div>
            </div>
          </section>
        </div>
      </div>

      <div className="prompt-editor__footer flex shrink-0 flex-wrap items-center justify-end gap-2 border-t border-border bg-background px-4 py-3">
        {creating && (
          <button
            type="button"
            title={t("promptsView.editor.cancel")}
            aria-label={t("promptsView.editor.cancel")}
            onClick={onCancelCreate}
            className="flex min-h-9 items-center gap-2 rounded-md border border-input px-4 py-2 text-sm text-muted-foreground hover:bg-accent hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
          >
            <XIcon className="h-4 w-4" aria-hidden="true" />
            <span className="prompt-editor__footer-label">
              {t("promptsView.editor.cancel")}
            </span>
          </button>
        )}
        <button
          type="submit"
          title={
            creating
              ? t("promptsView.editor.create")
              : t("promptsView.editor.save")
          }
          aria-label={
            creating
              ? t("promptsView.editor.create")
              : t("promptsView.editor.save")
          }
          disabled={!canSubmit}
          className="flex min-h-9 items-center gap-2 rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground transition-opacity focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-50"
        >
          <SaveIcon className="h-4 w-4" aria-hidden="true" />
          <span className="prompt-editor__footer-label">
            {creating
              ? t("promptsView.editor.create")
              : t("promptsView.editor.save")}
          </span>
        </button>
      </div>
    </form>
  );
}
