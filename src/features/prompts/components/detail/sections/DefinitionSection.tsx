import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import {
  ArrowDownIcon,
  ArrowUpIcon,
  MessageSquareIcon,
  PlusIcon,
  TextIcon,
  Trash2Icon,
} from "lucide-react";
import type { Prompt, PromptMessage, PromptMessageRole } from "../../../types";
import { substituteVariables } from "../../../promptText";
import { CopyPromptButton } from "../../CopyPromptButton";
import { VariableEditor } from "../../VariableEditor";
import type { PromptDraft } from "../promptDraft";

export interface DefinitionSectionProps {
  draft: PromptDraft;
  prompt: Prompt | null;
  chatMode: boolean;
  userPromptValid: boolean;
  readOnly: boolean;
  previewValues: Record<string, string>;
  writeText?: (text: string) => Promise<void>;
  onSetChatMode: (enabled: boolean) => void;
  onUpdateText: (key: "systemPrompt" | "userPrompt", value: string) => void;
  onUpdateMessages: (messages: PromptMessage[]) => void;
  onChange: (patch: Partial<PromptDraft>) => void;
  onPreviewValuesChange: (values: Record<string, string>) => void;
}

const labelClass = "text-xs font-medium text-muted-foreground";
const inputClass =
  "w-full rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-60";

export function DefinitionSection({
  draft,
  prompt,
  chatMode,
  userPromptValid,
  readOnly,
  previewValues,
  writeText,
  onSetChatMode,
  onUpdateText,
  onUpdateMessages,
  onChange,
  onPreviewValuesChange,
}: DefinitionSectionProps) {
  const { t } = useTranslation();

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

  return (
    <section
      aria-labelledby="prompt-editor-definition"
      className="border-t border-border pt-5"
    >
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex min-w-0 items-center gap-2">
          <CopyPromptButton
            source={{
              systemPrompt: draft.systemPrompt,
              userPrompt: draft.userPrompt,
              messages: draft.messages,
              variables: draft.variables,
            }}
            promptId={prompt?.id}
            name={draft.title || prompt?.title}
            locked={readOnly && prompt?.isLocked}
            writeText={writeText}
          />
          <h3
            id="prompt-editor-definition"
            className="text-sm font-semibold text-foreground"
          >
            {t("promptsView.editor.sections.definition")}
          </h3>
        </div>
        <div
          role="group"
          aria-label={t("evaluation.definitionMode")}
          className="flex rounded-md border border-input p-0.5"
        >
            <button
              type="button"
              aria-pressed={!chatMode}
              disabled={readOnly}
              onClick={() => onSetChatMode(false)}
              className={`flex min-h-8 items-center gap-1.5 rounded px-2 text-xs focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-40 ${
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
              disabled={readOnly}
              onClick={() => onSetChatMode(true)}
              className={`flex min-h-8 items-center gap-1.5 rounded px-2 text-xs focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-40 ${
                chatMode
                  ? "bg-accent text-foreground"
                  : "text-muted-foreground"
              }`}
            >
              <MessageSquareIcon className="h-3.5 w-3.5" aria-hidden="true" />
              {t("evaluation.chatMode")}
            </button>
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
                  disabled={readOnly}
                  onChange={(event) => {
                    const next = [...draft.messages];
                    next[index] = {
                      ...message,
                      role: event.target.value as PromptMessageRole,
                    };
                    onUpdateMessages(next);
                  }}
                  className="prompt-editor__message-role w-full min-w-[6.5rem] rounded-md border border-input bg-background px-2 py-2 text-xs text-foreground"
                >
                  <option value="system">{t("evaluation.roleSystem")}</option>
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
                  disabled={readOnly}
                  onChange={(event) => {
                    const next = [...draft.messages];
                    next[index] = {
                      ...message,
                      content: event.target.value,
                    };
                    onUpdateMessages(next);
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
                          readOnly ||
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
                            const target = action === 0 ? index - 1 : index + 1;
                            [next[index], next[target]] = [
                              next[target],
                              next[index],
                            ];
                          }
                          onUpdateMessages(next);
                        }}
                        className="flex h-8 w-8 items-center justify-center rounded-md text-muted-foreground hover:bg-accent hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-30"
                      >
                        <Icon className="h-3.5 w-3.5" aria-hidden="true" />
                      </button>
                    ),
                  )}
                </div>
              </div>
            ))}
            <button
              type="button"
              disabled={readOnly}
              onClick={() =>
                onUpdateMessages([
                  ...draft.messages,
                  { role: "user", content: "" },
                ])
              }
              className="flex min-h-8 w-fit items-center gap-1.5 rounded-md border border-input px-2.5 text-xs text-muted-foreground hover:bg-accent hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-40"
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
                placeholder={t("promptsView.editor.systemPromptPlaceholder")}
                disabled={readOnly}
                onChange={(e) => onUpdateText("systemPrompt", e.target.value)}
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
                placeholder={t("promptsView.editor.userPromptPlaceholder")}
                disabled={readOnly}
                onChange={(e) => onUpdateText("userPrompt", e.target.value)}
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
          onChange={(variables) => onChange({ variables })}
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
                  disabled={readOnly}
                  onChange={(e) =>
                    onPreviewValuesChange({
                      ...previewValues,
                      [variable.name]: e.target.value,
                    })
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
  );
}
