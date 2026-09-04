import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type KeyboardEvent as ReactKeyboardEvent,
} from "react";
import { useTranslation } from "react-i18next";
import {
  CopyIcon,
  LockIcon,
  MoreHorizontalIcon,
  PencilIcon,
  PinIcon,
  StarIcon,
  Trash2Icon,
  XIcon,
} from "lucide-react";
import { Modal } from "../../../../components/ui/Modal";
import type {
  CreateFolderInput,
  CreatePromptTypeInput,
  Folder,
  Prompt,
  PromptListItem,
  PromptMessage,
  PromptTypeDefinition,
  PromptVersion,
} from "../../types";
import { setPreferredChatMode } from "../../definitionMode";
import {
  defaultVariableValues,
  formatCopiedPrompt,
  syncVariables,
} from "../../promptText";
import {
  CopyPromptButton,
  pushPromptCopyToast,
  recordCopiedPromptUsage,
} from "../CopyPromptButton";
import {
  canSubmitDraft,
  draftSnapshot,
  isChatMode,
  titleIsValid,
  toCreateInput,
  toDraft,
  toUpdatePatch,
  userPromptIsValid,
  type PromptDraft,
} from "./promptDraft";
import { IdentitySection } from "./sections/IdentitySection";
import { DefinitionSection } from "./sections/DefinitionSection";
import { OrganizationSection } from "./sections/OrganizationSection";
import { MediaSection } from "./sections/MediaSection";
import { VersionTab } from "./VersionTab";
import { RunTab } from "./RunTab";
import { ReferencesTab } from "./ReferencesTab";
import {
  usePromptStore,
  type DetailActions,
  type NavigationGuard,
  type SaveResult,
} from "../../promptStore";
import { platformModifier } from "../../../../shortcuts/platform";
import { useToastStore } from "../../../notifications/toastStore";

import { Button, IconButton, Kbd } from "../../../../components/ui";
import { cn } from "../../../../components/ui/cn";

export type DetailTab = "content" | "versions" | "run" | "references";

export interface PromptDetailModalProps {
  open: boolean;
  creating: boolean;
  prompt: Prompt | null;
  prompts: PromptListItem[];
  versions: PromptVersion[];
  folders: Folder[];
  promptTypeDefinitions: PromptTypeDefinition[];
  knownTags: string[];
  onClose: () => void;
  onCreate: (input: ReturnType<typeof toCreateInput>) => Promise<unknown>;
  onSave: (
    id: string,
    patch: ReturnType<typeof toUpdatePatch>,
  ) => Promise<unknown>;
  onCreateFolder: (input: CreateFolderInput) => Promise<Folder | null>;
  onCreatePromptType: (
    input: CreatePromptTypeInput,
  ) => Promise<PromptTypeDefinition | null>;
  onToggleFavorite: (id: string, next: boolean) => void;
  onTogglePin: (id: string, next: boolean) => void;
  onDuplicate: (id: string) => void;
  onDelete: (id: string) => void;
  onCreateVersion: (note?: string) => void;
  onRollback: (version: number) => void;
}

const TABS: DetailTab[] = ["content", "versions", "run", "references"];


export function PromptDetailModal({
  open,
  creating,
  prompt,
  prompts,
  versions,
  folders,
  promptTypeDefinitions,
  knownTags,
  onClose,
  onCreate,
  onSave,
  onCreateFolder,
  onCreatePromptType,
  onToggleFavorite,
  onTogglePin,
  onDuplicate,
  onDelete,
  onCreateVersion,
  onRollback,
}: PromptDetailModalProps) {
  const { t } = useTranslation();
  const [draft, setDraft] = useState<PromptDraft>(() => toDraft(prompt));
  const [baseline, setBaseline] = useState(() => draftSnapshot(toDraft(prompt)));
  const [tagInput, setTagInput] = useState("");
  const [previewValues, setPreviewValues] = useState<Record<string, string>>(
    {},
  );
  const [readOnly, setReadOnly] = useState(false);
  const [tab, setTab] = useState<DetailTab>("content");
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [menuOpen, setMenuOpen] = useState(false);
  const pendingNav = useRef<((value: "proceed" | "cancel") => void) | null>(
    null,
  );
  const closeIntent = useRef<"close" | "nav" | null>(null);

  const locked = Boolean(prompt?.isLocked);
  const resetKey = creating ? "__new__" : prompt?.id ?? "__none__";

  useEffect(() => {
    const next = toDraft(prompt);
    setDraft(next);
    setBaseline(draftSnapshot(next));
    setTagInput("");
    setPreviewValues({});
    setTab("content");
    setReadOnly(locked);
    setConfirmOpen(false);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [resetKey, locked]);

  const dirty = draftSnapshot(draft) !== baseline;
  const chatMode = isChatMode(draft);

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

  const insertReference = (token: string) => {
    if (chatMode) {
      const messages = [...draft.messages];
      if (messages.length === 0) {
        updateMessages([{ role: "user", content: token }]);
        return;
      }
      const last = messages[messages.length - 1];
      messages[messages.length - 1] = {
        ...last,
        content: last.content ? `${last.content}\n${token}` : token,
      };
      updateMessages(messages);
      return;
    }
    updateText(
      "userPrompt",
      draft.userPrompt ? `${draft.userPrompt}\n${token}` : token,
    );
  };

  const save = useCallback(async (): Promise<SaveResult> => {
    if (!canSubmitDraft(draft)) {
      return { ok: false, errors: { title: !titleIsValid(draft) } };
    }
    if (creating) {
      const created = await onCreate(toCreateInput(draft));
      if (!created) return { ok: false, errors: {} };
      setBaseline(draftSnapshot(draft));
      useToastStore.getState().push({
        message: t("promptsView.toast.saved"),
        tone: "success",
      });
      return { ok: true };
    }
    if (!prompt) return { ok: false, errors: {} };
    const saved = await onSave(prompt.id, toUpdatePatch(draft));
    if (!saved) return { ok: false, errors: {} };
    setBaseline(draftSnapshot(draft));
    useToastStore.getState().push({
      message: t("promptsView.toast.saved"),
      tone: "success",
    });
    return { ok: true };
  }, [creating, draft, onCreate, onSave, prompt, t]);

  const copyFilled = useCallback(async () => {
    if (locked) return;
    const values = {
      ...defaultVariableValues(draft.variables),
      ...previewValues,
    };
    try {
      if (prompt) {
        const copied = await usePromptStore
          .getState()
          .api.copyPrompt(prompt.id, values);
        await navigator.clipboard.writeText(formatCopiedPrompt(copied));
        pushPromptCopyToast(t, "success", prompt.title);
        await recordCopiedPromptUsage(creating ? undefined : prompt.id);
        return;
      }
      await navigator.clipboard.writeText(
        formatCopiedPrompt({
          systemPrompt: draft.systemPrompt,
          userPrompt: draft.userPrompt,
          messages: draft.messages,
        }),
      );
      pushPromptCopyToast(t, "success", draft.title || undefined);
    } catch {
      pushPromptCopyToast(t, "failure");
    }
  }, [creating, draft, locked, previewValues, prompt, t]);

  const resolvePending = (value: "proceed" | "cancel") => {
    pendingNav.current?.(value);
    pendingNav.current = null;
    setConfirmOpen(false);
  };

  const finishProceed = () => {
    const intent = closeIntent.current;
    closeIntent.current = null;
    resolvePending("proceed");
    if (intent !== "nav") onClose();
  };

  const requestClose = () => {
    if (!dirty) {
      onClose();
      return;
    }
    closeIntent.current = "close";
    setConfirmOpen(true);
  };

  const navigationGuard: NavigationGuard = useCallback(async () => {
    if (!dirty) return "proceed";
    closeIntent.current = "nav";
    setConfirmOpen(true);
    return new Promise<"proceed" | "cancel">((resolve) => {
      pendingNav.current = resolve;
    });
  }, [dirty]);

  const detailActions: DetailActions = useMemo(
    () => ({
      save,
      copy: copyFilled,
    }),
    [copyFilled, save],
  );

  const registerNavigationGuard = usePromptStore(
    (state) => state.registerNavigationGuard,
  );
  const registerDetailActions = usePromptStore(
    (state) => state.registerDetailActions,
  );

  useEffect(() => {
    if (!open) return;
    registerNavigationGuard(navigationGuard);
    registerDetailActions(detailActions);
    return () => {
      registerNavigationGuard(null);
      registerDetailActions(null);
    };
  }, [
    detailActions,
    navigationGuard,
    open,
    registerDetailActions,
    registerNavigationGuard,
  ]);

  const modifier = platformModifier().symbol;

  const title = creating
    ? t("promptsView.detail.newPrompt")
    : prompt?.title || t("promptsView.untitled");
  const tabsDisabled = locked || creating;

  const onTabKeyDown = (event: ReactKeyboardEvent<HTMLDivElement>) => {
    const index = TABS.indexOf(tab);
    if (event.key === "ArrowRight") {
      event.preventDefault();
      setTab(TABS[(index + 1) % TABS.length]);
    }
    if (event.key === "ArrowLeft") {
      event.preventDefault();
      setTab(TABS[(index - 1 + TABS.length) % TABS.length]);
    }
  };

  const bodyText = chatMode
    ? draft.messages.map((message) => message.content).join("\n")
    : `${draft.systemPrompt}\n${draft.userPrompt}`;

  return (
    <>
      <Modal open={open} title={title} onClose={requestClose}>
        <div className="flex h-[min(90vh,56rem)] flex-col">
          <header className="flex shrink-0 flex-wrap items-center gap-2 border-b border-border px-4 py-2">
            {!creating && prompt && (
              <CopyPromptButton
                source={{
                  systemPrompt: draft.systemPrompt,
                  userPrompt: draft.userPrompt,
                  messages: draft.messages,
                  variables: draft.variables,
                }}
                promptId={prompt.id}
                locked={locked}
                name={prompt.title}
              />
            )}
            <div className="min-w-0 flex-1">
              <div className="truncate text-body font-semibold text-foreground">
                {title}
              </div>
              <div className="flex flex-wrap items-center gap-2 text-label text-muted-foreground">
                {!creating && prompt && (
                  <span>{t("promptsView.detail.versionChip", { version: prompt.currentVersion })}</span>
                )}
                {prompt?.description && (
                  <span className="truncate">{prompt.description}</span>
                )}
              </div>
            </div>
            <IconButton
              label={readOnly ? t("promptsView.detail.edit") : t("promptsView.detail.readOnly")}
              icon={<PencilIcon className="h-4 w-4" aria-hidden="true" />}
              disabled={locked}
              onClick={() => setReadOnly((value) => !value)}
              aria-pressed={!readOnly}
            />
            {!creating && prompt && (
              <>
                <IconButton
                  label={
                    prompt.isFavorite
                      ? t("promptsView.unfavorite")
                      : t("promptsView.favorite")
                  }
                  icon={<StarIcon
                    className={`h-4 w-4 ${prompt.isFavorite ? "fill-current text-primary" : ""}`}
                    aria-hidden="true"
                  />}
                  onClick={() =>
                    onToggleFavorite(prompt.id, !prompt.isFavorite)
                  }
                  aria-pressed={prompt.isFavorite}
                />
                <IconButton
                  label={prompt.isPinned ? t("promptsView.unpin") : t("promptsView.pin")}
                  icon={<PinIcon
                    className={`h-4 w-4 ${prompt.isPinned ? "fill-current text-primary" : ""}`}
                    aria-hidden="true"
                  />}
                  onClick={() => onTogglePin(prompt.id, !prompt.isPinned)}
                  aria-pressed={prompt.isPinned}
                />
                <div className="relative">
                  <IconButton
                    label={t("promptsView.detail.moreActions")}
                    icon={<MoreHorizontalIcon className="h-4 w-4" aria-hidden="true" />}
                    onClick={() => setMenuOpen((value) => !value)}
                    aria-expanded={menuOpen}
                  />
                  {menuOpen && (
                    <div
                      role="menu"
                      className="absolute right-0 z-10 mt-1 w-40 rounded-md border border-border bg-popover p-1 text-popover-foreground shadow-md"
                    >
                      <button
                        type="button"
                        role="menuitem"
                        onClick={() => {
                          setMenuOpen(false);
                          onDuplicate(prompt.id);
                        }}
                        className="flex w-full items-center gap-2 rounded-sm px-2 py-1.5 text-left text-body hover:bg-accent"
                      >
                        <CopyIcon className="h-3.5 w-3.5" aria-hidden="true" />
                        {t("promptsView.duplicatePrompt")}
                      </button>
                      <button
                        type="button"
                        role="menuitem"
                        onClick={() => {
                          setMenuOpen(false);
                          onDelete(prompt.id);
                        }}
                        className="flex w-full items-center gap-2 rounded-sm px-2 py-1.5 text-left text-body text-destructive hover:bg-destructive/10"
                      >
                        <Trash2Icon className="h-3.5 w-3.5" aria-hidden="true" />
                        {t("promptsView.deletePrompt")}
                      </button>
                    </div>
                  )}
                </div>
              </>
            )}
            <IconButton
              label={t("promptsView.detail.close")}
              icon={<XIcon className="h-4 w-4" aria-hidden="true" />}
              onClick={requestClose}
            />
          </header>

          <div
            role="tablist"
            aria-label={t("promptsView.detail.content")}
            onKeyDown={onTabKeyDown}
            className="flex shrink-0 items-center gap-1 border-b border-border px-4"
          >
            {TABS.map((item) => {
              const disabled =
                item !== "content" && tabsDisabled;
              const label =
                item === "content"
                  ? t("promptsView.detail.content")
                  : item === "versions"
                    ? t("promptsView.detail.versions")
                    : item === "run"
                      ? t("promptsView.detail.run")
                      : t("promptsView.detail.references");
              const count =
                item === "versions" && versions.length > 0
                  ? ` (${versions.length})`
                  : "";
              return (
                <button
                  key={item}
                  type="button"
                  role="tab"
                  aria-selected={tab === item}
                  disabled={disabled}
                  title={disabled ? t("promptsView.detail.lockedTabsHint") : label}
                  onClick={() => setTab(item)}
                  className={cn(
                    "-mb-px min-h-9 border-b-2 px-3 text-body",
                    "transition-colors duration-fast ease-out",
                    "disabled:opacity-50",
                    tab === item
                      ? "border-primary font-medium text-foreground"
                      : "border-transparent text-muted-foreground hover:text-foreground",
                  )}
                >
                  {label}
                  {count}
                </button>
              );
            })}
          </div>

          <div className="min-h-0 flex-1 overflow-hidden">
            {locked && tab === "content" ? (
              <div className="flex h-full flex-col items-center justify-center gap-2 p-8 text-center">
                <LockIcon
                  className="h-control-sm w-control-sm text-muted-foreground"
                  aria-hidden="true"
                />
                <h3 className="text-body font-semibold text-foreground">
                  {t("promptsView.privateLockedTitle")}
                </h3>
                <p className="max-w-sm text-body text-muted-foreground">
                  {t("promptsView.privateLockedHint")}
                </p>
              </div>
            ) : tab === "content" ? (
              <form
                className="prompt-editor flex h-full min-h-0 flex-col"
                onSubmit={(event) => {
                  event.preventDefault();
                  void save();
                }}
              >
                <div className="prompt-editor__body min-h-0 flex-1 overflow-y-auto px-4 py-5">
                  <div className="mx-auto flex w-full max-w-[68ch] flex-col gap-7">
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
                      onSetChatMode={setChatMode}
                      onUpdateText={updateText}
                      onUpdateMessages={updateMessages}
                      onChange={onChange}
                      onPreviewValuesChange={setPreviewValues}
                    />
                    {draft.variables.length > 0 && (
                      <Button
                        size="lg"
                        className="w-fit"
                        onClick={() => void copyFilled()}
                      >
                        {t("promptsView.detail.fillAndCopy")}
                      </Button>
                    )}
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
              </form>
            ) : tab === "versions" && prompt ? (
              <VersionTab
                prompt={prompt}
                versions={versions}
                promptTypeDefinitions={promptTypeDefinitions}
                onCreateVersion={onCreateVersion}
                onRollback={onRollback}
              />
            ) : tab === "run" && prompt ? (
              <RunTab prompt={prompt} versions={versions} />
            ) : (
              <ReferencesTab
                prompt={prompt}
                prompts={prompts}
                onInsert={insertReference}
              />
            )}
          </div>

          <footer className="flex shrink-0 flex-wrap items-center justify-between gap-3 border-t border-border px-4 py-3">
            <div className="flex min-w-0 flex-wrap items-center gap-x-3 gap-y-1">
              <span className="flex items-center gap-1 text-label text-muted-foreground">
                <Kbd>{t("promptsView.detail.saveHint", { mod: modifier })}</Kbd>
                <Kbd>{t("promptsView.detail.copyHint", { mod: modifier })}</Kbd>
              </span>
              {tab === "content" && !locked && (
                <span className="text-label tabular-nums text-muted-foreground-subtle">
                  {t("promptsView.detail.characterCount", {
                    count: bodyText.length,
                  })}
                </span>
              )}
            </div>
            <div className="flex gap-2">
              <Button size="lg" variant="ghost" onClick={requestClose}>
                {t("promptsView.detail.close")}
              </Button>
              <Button
                size="lg"
                variant="primary"
                disabled={!canSubmitDraft(draft) || readOnly || locked}
                onClick={() => void save()}
              >
                {creating
                  ? t("promptsView.editor.create")
                  : t("promptsView.editor.save")}
              </Button>
            </div>
          </footer>
        </div>
      </Modal>

      <Modal
        open={confirmOpen}
        title={t("promptsView.detail.dirtyTitle")}
        onClose={() => resolvePending("cancel")}
        className="max-h-none w-full max-w-md"
      >
        <div className="p-5">
          <p className="text-body text-muted-foreground">
            {t("promptsView.detail.dirtyMessage")}
          </p>
          {/*
            Three destinations with different weight, so they get different
            weight: discarding is the only irreversible one and is marked as
            such, keeping is the quiet escape hatch, saving is the default.
            Reverse order on desktop puts the primary action nearest the corner.
          */}
          <div className="mt-5 flex flex-col gap-2 sm:flex-row-reverse sm:justify-start">
            <button
              type="button"
              onClick={() => {
                void save().then((result) => {
                  if (!result.ok) {
                    resolvePending("cancel");
                    return;
                  }
                  // Create already selected the new Prompt; onClose would deselect it.
                  if (creating) {
                    closeIntent.current = null;
                    resolvePending("proceed");
                    return;
                  }
                  finishProceed();
                });
              }}
              className="rounded-md bg-primary px-3 py-2 text-body font-medium text-primary-foreground transition-colors duration-fast ease-out hover:bg-primary/90"
            >
              {t("promptsView.detail.saveAndClose")}
            </button>
            <Button size="lg" variant="ghost" onClick={() => resolvePending("cancel")}>
              {t("promptsView.detail.keepEditing")}
            </Button>
            <Button
              size="lg"
              variant="ghost"
              className="text-destructive hover:bg-destructive/10 hover:text-destructive"
              onClick={finishProceed}
            >
              {t("promptsView.detail.discardAndClose")}
            </Button>
          </div>
        </div>
      </Modal>
    </>
  );
}
