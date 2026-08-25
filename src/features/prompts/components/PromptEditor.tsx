import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { SaveIcon, XIcon } from "lucide-react";
import type {
  CreateFolderInput,
  CreatePromptInput,
  CreatePromptTypeInput,
  Folder,
  Prompt,
  PromptMessage,
  PromptTypeDefinition,
  UpdatePromptInput,
} from "../types";
import { setPreferredChatMode } from "../definitionMode";
import { syncVariables } from "../promptText";
import {
  canSubmitDraft,
  isChatMode,
  titleIsValid,
  toCreateInput,
  toDraft,
  toUpdatePatch,
  userPromptIsValid,
  type PromptDraft,
} from "./detail/promptDraft";
import { IdentitySection } from "./detail/sections/IdentitySection";
import { DefinitionSection } from "./detail/sections/DefinitionSection";
import { OrganizationSection } from "./detail/sections/OrganizationSection";
import { MediaSection } from "./detail/sections/MediaSection";

interface PromptEditorProps {
  prompt: Prompt | null;
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
  readOnly?: boolean;
}

/**
 * The prompt editor form (Req 6.1, 6.4, 6.7). Draft ownership for this
 * component is transitional: PromptDetailModal hoists the same draft.
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
  readOnly = false,
}: PromptEditorProps) {
  const { t } = useTranslation();
  const [draft, setDraft] = useState<PromptDraft>(() => toDraft(prompt));
  const [tagInput, setTagInput] = useState("");
  const [previewValues, setPreviewValues] = useState<Record<string, string>>({});

  const resetKey = creating ? "__new__" : prompt?.id ?? "__none__";
  useEffect(() => {
    setDraft(toDraft(prompt));
    setTagInput("");
    setPreviewValues({});
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [resetKey]);

  const onChange = (patch: Partial<PromptDraft>) =>
    setDraft((current) => ({ ...current, ...patch }));

  const updateText = (key: "systemPrompt" | "userPrompt", value: string) => {
    setDraft((current) => {
      const next = { ...current, [key]: value };
      next.variables = syncVariables(
        current.variables,
        next.systemPrompt,
        next.userPrompt,
      );
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

  const chatMode = isChatMode(draft);

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

  const addTag = (raw: string) => {
    const tag = raw.trim();
    if (tag !== "" && !draft.tags.includes(tag)) {
      onChange({ tags: [...draft.tags, tag] });
    }
    setTagInput("");
  };

  const submit = () => {
    if (!canSubmitDraft(draft)) return;
    if (creating) {
      onCreate(toCreateInput(draft));
    } else if (prompt) {
      onSave(prompt.id, toUpdatePatch(draft));
    }
  };

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
          <IdentitySection
            draft={draft}
            titleValid={titleIsValid(draft)}
            readOnly={readOnly}
            promptTypeDefinitions={promptTypeDefinitions}
            onChange={onChange}
            onCreatePromptType={onCreatePromptType}
          />
          <DefinitionSection
            draft={draft}
            prompt={prompt}
            chatMode={chatMode}
            userPromptValid={userPromptIsValid(draft)}
            readOnly={readOnly}
            previewValues={previewValues}
            writeText={writeText}
            onSetChatMode={setChatMode}
            onUpdateText={updateText}
            onUpdateMessages={updateMessages}
            onChange={onChange}
            onPreviewValuesChange={setPreviewValues}
          />
          <OrganizationSection
            draft={draft}
            folders={folders}
            knownTags={knownTags}
            tagInput={tagInput}
            resetKey={resetKey}
            readOnly={readOnly}
            onChange={onChange}
            onTagInputChange={setTagInput}
            onAddTag={addTag}
            onCreateFolder={onCreateFolder}
          />
          <MediaSection
            draft={draft}
            readOnly={readOnly}
            onChange={onChange}
          />
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
          disabled={!canSubmitDraft(draft) || readOnly}
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
