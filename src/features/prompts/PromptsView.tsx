import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  ArrowLeftIcon,
  ChevronLeftIcon,
  ChevronRightIcon,
  CopyIcon,
  CommandIcon,
  DownloadIcon,
  FlaskConicalIcon,
  HistoryIcon,
  LockIcon,
  PinIcon,
  PlusIcon,
  StarIcon,
  Trash2Icon,
  UploadIcon,
} from "lucide-react";
import { EvaluationWorkbench } from "../evaluation/EvaluationWorkbench";
import {
  selectSelectedPrompt,
  PROMPT_PAGE_SIZE,
  usePromptStore,
} from "./promptStore";
import type { BundlePreview, ImportConflictPolicy } from "./types";
import { SearchBar } from "./components/SearchBar";
import { PromptList } from "./components/PromptList";
import { PromptEditor } from "./components/PromptEditor";
import { VersionHistory } from "./components/VersionHistory";
import { BatchToolbar } from "./components/BatchToolbar";

const iconButtonClass =
  "flex h-8 w-8 shrink-0 items-center justify-center rounded-md text-muted-foreground hover:bg-accent hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-40";

/**
 * The prompt-editing view (Req 22.3). Lays out a searchable/filterable prompt
 * list (Req 5, 6.3) and a prompt editor with version history (Req 6, 7). Folder
 * and tag navigation lives in PromptLibraryNav. All data flows through the
 * prompt store, which routes every backend call through the Runtime_Bridge
 * (Req 3.1).
 */
export function PromptsView() {
  const { t } = useTranslation();

  const folders = usePromptStore((s) => s.folders);
  const prompts = usePromptStore((s) => s.prompts);
  const total = usePromptStore((s) => s.total);
  const offset = usePromptStore((s) => s.offset);
  const tags = usePromptStore((s) => s.tags);
  const promptTypeDefinitions = usePromptStore((s) => s.promptTypeDefinitions);
  const filters = usePromptStore((s) => s.filters);
  const selectedPromptId = usePromptStore((s) => s.selectedPromptId);
  const versions = usePromptStore((s) => s.versions);
  const loading = usePromptStore((s) => s.loading);
  const error = usePromptStore((s) => s.error);
  const selectedPrompt = usePromptStore(selectSelectedPrompt);
  const selectedPromptIds = usePromptStore((s) => s.selectedPromptIds);

  const load = usePromptStore((s) => s.load);
  const setFilters = usePromptStore((s) => s.setFilters);
  const toggleTagFilter = usePromptStore((s) => s.toggleTagFilter);
  const loadPreviousPage = usePromptStore((s) => s.loadPreviousPage);
  const loadNextPage = usePromptStore((s) => s.loadNextPage);
  const selectPrompt = usePromptStore((s) => s.selectPrompt);
  const createPrompt = usePromptStore((s) => s.createPrompt);
  const savePrompt = usePromptStore((s) => s.savePrompt);
  const deletePrompt = usePromptStore((s) => s.deletePrompt);
  const duplicatePrompt = usePromptStore((s) => s.duplicatePrompt);
  const togglePromptSelection = usePromptStore((s) => s.togglePromptSelection);
  const selectPage = usePromptStore((s) => s.selectPage);
  const clearPromptSelection = usePromptStore((s) => s.clearPromptSelection);
  const batchMove = usePromptStore((s) => s.batchMove);
  const batchTag = usePromptStore((s) => s.batchTag);
  const batchDelete = usePromptStore((s) => s.batchDelete);
  const exportBundle = usePromptStore((s) => s.exportBundle);
  const previewBundle = usePromptStore((s) => s.previewBundle);
  const importBundle = usePromptStore((s) => s.importBundle);
  const createFolder = usePromptStore((s) => s.createFolder);
  const createPromptType = usePromptStore((s) => s.createPromptType);
  const createVersion = usePromptStore((s) => s.createVersion);
  const rollbackVersion = usePromptStore((s) => s.rollbackVersion);

  const [creating, setCreating] = useState(false);
  const [compactPane, setCompactPane] = useState<"list" | "detail">("list");
  const [workspaceMode, setWorkspaceMode] = useState<"editor" | "evaluation">("editor");
  const [showHistory, setShowHistory] = useState(false);
  const [showImport, setShowImport] = useState(false);
  const [bundlePath, setBundlePath] = useState("");
  const [conflictPolicy, setConflictPolicy] =
    useState<ImportConflictPolicy>("skip");
  const [bundlePreview, setBundlePreview] = useState<BundlePreview | null>(null);
  const [transferMessage, setTransferMessage] = useState<string | null>(null);

  useEffect(() => {
    void load();
  }, [load]);

  const startCreate = () => {
    setCreating(true);
    setCompactPane("detail");
    void selectPrompt(null);
  };

  const handleDeletePrompt = (id: string) => {
    if (window.confirm(t("promptsView.deletePromptConfirm"))) {
      void deletePrompt(id);
    }
  };

  const editorActive = creating || selectedPrompt != null;
  const navigationVisible = workspaceMode !== "evaluation" || !selectedPrompt;
  const discoveryPaneClass = navigationVisible
    ? `prompt-workspace__discovery min-w-0 w-full shrink-0 flex-col border-r border-border ${
        compactPane === "list" ? "flex" : "hidden"
      }`
    : "hidden";
  const detailPaneClass = `prompt-workspace__detail min-w-0 flex-1 flex-col ${
    compactPane === "detail" || !navigationVisible ? "flex" : "hidden"
  }`;

  return (
    <div className="prompt-workspace relative flex h-full min-h-0 w-full overflow-hidden">
      {/* Prompt list + search */}
      <section
        aria-label={t("common.prompts")}
        className={discoveryPaneClass}
      >
        <div className="flex flex-col gap-2 border-b border-border p-3">
          <div className="flex flex-wrap items-center gap-2">
            <div className="flex min-w-0 flex-1 items-center gap-2">
              <h2 className="truncate text-sm font-semibold text-foreground">
                {t("common.prompts")}
              </h2>
            </div>
            <div className="flex shrink-0 items-center gap-1">
              <button
                type="button"
                title={t("promptsView.bundle.export")}
                aria-label={t("promptsView.bundle.export")}
                onClick={() =>
                  void exportBundle().then((result) => {
                    if (result) {
                      setTransferMessage(
                        t("promptsView.bundle.exported", {
                          path: result.filePath,
                        }),
                      );
                    }
                  })
                }
                className={iconButtonClass}
              >
                <DownloadIcon className="h-4 w-4" aria-hidden="true" />
              </button>
              <button
                type="button"
                title={t("promptsView.bundle.import")}
                aria-label={t("promptsView.bundle.import")}
                aria-pressed={showImport}
                onClick={() => setShowImport((open) => !open)}
                className={iconButtonClass}
              >
                <UploadIcon className="h-4 w-4" aria-hidden="true" />
              </button>
              <button
                type="button"
                onClick={startCreate}
                className="flex h-8 items-center gap-1.5 rounded-md bg-primary px-2.5 text-xs font-medium text-primary-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
              >
                <PlusIcon className="h-3.5 w-3.5" aria-hidden="true" />
                {t("promptsView.newPrompt")}
              </button>
            </div>
          </div>
          <SearchBar
            filters={filters}
            tags={tags}
            onChange={(patch) => void setFilters(patch)}
            onToggleTag={(tag) => void toggleTagFilter(tag)}
            onClear={() =>
              void setFilters({ tags: [], favoritesOnly: false })
            }
          />
        </div>
        {showImport && (
          <div className="flex flex-col gap-2 border-b border-border bg-muted/30 p-3">
            <input
              value={bundlePath}
              onChange={(event) => {
                setBundlePath(event.target.value);
                setBundlePreview(null);
              }}
              placeholder={t("promptsView.bundle.pathPlaceholder")}
              aria-label={t("promptsView.bundle.path")}
              className="w-full rounded border border-input bg-background px-2 py-1.5 text-xs text-foreground"
            />
            <div className="flex items-center gap-2">
              <select
                value={conflictPolicy}
                onChange={(event) =>
                  setConflictPolicy(event.target.value as ImportConflictPolicy)
                }
                aria-label={t("promptsView.bundle.conflictPolicy")}
                className="min-w-0 flex-1 rounded border border-input bg-background px-2 py-1.5 text-xs text-foreground"
              >
                <option value="skip">{t("promptsView.bundle.skip")}</option>
                <option value="duplicate">{t("promptsView.bundle.duplicate")}</option>
                <option value="replace">{t("promptsView.bundle.replace")}</option>
              </select>
              <button
                type="button"
                disabled={bundlePath.trim() === ""}
                onClick={() =>
                  void previewBundle(bundlePath.trim()).then(setBundlePreview)
                }
                className="rounded border border-input px-2 py-1.5 text-xs text-foreground hover:bg-accent disabled:opacity-40"
              >
                {t("promptsView.bundle.preview")}
              </button>
            </div>
            {bundlePreview && (
              <div className="text-xs text-muted-foreground">
                <p>
                  {t("promptsView.bundle.previewSummary", {
                    prompts: bundlePreview.prompts,
                    revisions: bundlePreview.revisions,
                    conflicts: bundlePreview.conflicts,
                    mediaFiles: bundlePreview.mediaFiles,
                  })}
                </p>
                <p>
                  {t("promptsView.bundle.typeDefinitionSummary", {
                    additions: bundlePreview.typeDefinitionAdditions,
                    conflicts: bundlePreview.typeDefinitionConflicts,
                  })}
                </p>
                {bundlePreview.typeDefinitionConflicts > 0 && (
                  <p role="alert" className="mt-1 text-destructive">
                    {t("promptsView.bundle.typeDefinitionConflict")}
                  </p>
                )}
                {bundlePreview.privatePrompts > 0 && (
                  <p className="mt-1 text-foreground">
                    {t("promptsView.bundle.privateKeyWarning", {
                      count: bundlePreview.privatePrompts,
                    })}
                  </p>
                )}
                <button
                  type="button"
                  disabled={bundlePreview.typeDefinitionConflicts > 0}
                  onClick={() =>
                    void importBundle(bundlePath.trim(), conflictPolicy).then(
                      (result) => {
                        if (result) {
                          setTransferMessage(
                            t("promptsView.bundle.imported", {
                              added: result.added,
                              replaced: result.replaced,
                              skipped: result.skipped,
                              backupId: result.backupId,
                            }),
                          );
                          setShowImport(false);
                          setBundlePreview(null);
                        }
                      },
                    )
                  }
                  className="mt-2 rounded bg-primary px-3 py-1.5 text-xs font-medium text-primary-foreground disabled:cursor-not-allowed disabled:opacity-50"
                >
                  {t("promptsView.bundle.confirmImport")}
                </button>
              </div>
            )}
          </div>
        )}
        {transferMessage && (
          <div className="border-b border-border px-3 py-2 text-xs text-muted-foreground">
            {transferMessage}
          </div>
        )}
        {selectedPromptIds.length > 0 && (
          <BatchToolbar
            selectedCount={selectedPromptIds.length}
            folders={folders}
            onSelectPage={selectPage}
            onClear={clearPromptSelection}
            onMove={(folderId) => void batchMove(folderId)}
            onTag={(selectedTags) => void batchTag(selectedTags)}
            onDelete={() => {
              if (window.confirm(t("promptsView.batch.deleteConfirm"))) {
                void batchDelete();
              }
            }}
          />
        )}
        <div className="min-h-0 flex-1 overflow-y-auto">
          <PromptList
            prompts={prompts}
            promptTypeDefinitions={promptTypeDefinitions}
            selectedPromptId={selectedPromptId}
            selectedPromptIds={selectedPromptIds}
            loading={loading}
            onToggleSelection={togglePromptSelection}
            onSelect={(id) => {
              setCreating(false);
              setCompactPane("detail");
              void selectPrompt(id);
            }}
          />
        </div>
        <div className="flex h-10 shrink-0 items-center justify-between border-t border-border px-2">
          <span className="text-xs tabular-nums text-muted-foreground">
            {t("promptsView.pagination.summary", {
              from: total === 0 ? 0 : offset + 1,
              to: Math.min(offset + prompts.length, total),
              total,
            })}
          </span>
          <div className="flex items-center gap-1">
            <button
              type="button"
              title={t("promptsView.pagination.previous")}
              aria-label={t("promptsView.pagination.previous")}
              disabled={loading || offset === 0}
              onClick={() => void loadPreviousPage()}
              className={iconButtonClass}
            >
              <ChevronLeftIcon className="h-4 w-4" aria-hidden="true" />
            </button>
            <button
              type="button"
              title={t("promptsView.pagination.next")}
              aria-label={t("promptsView.pagination.next")}
              disabled={
                loading || offset + PROMPT_PAGE_SIZE >= total
              }
              onClick={() => void loadNextPage()}
              className={iconButtonClass}
            >
              <ChevronRightIcon className="h-4 w-4" aria-hidden="true" />
            </button>
          </div>
        </div>
      </section>

      {/* Editor + version history */}
      <section
        aria-label={t("evaluation.workspaceMode")}
        className={detailPaneClass}
      >
        {error && (
          <div
            role="alert"
            className="border-b border-destructive/40 bg-destructive/10 px-4 py-2 text-sm text-destructive"
          >
            {error}
          </div>
        )}

        {editorActive ? (
          <>
            <div className="prompt-workspace__detail-header flex shrink-0 flex-wrap items-center gap-2 border-b border-border px-3 py-2">
              {navigationVisible && (
                <button
                  type="button"
                  onClick={() => setCompactPane("list")}
                  className="prompt-workspace__compact-control flex h-8 shrink-0 items-center gap-1.5 rounded-md border border-input px-2 text-xs text-muted-foreground hover:bg-accent hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                >
                  <ArrowLeftIcon className="h-4 w-4" aria-hidden="true" />
                  {t("common.prompts")}
                </button>
              )}
              <span className="prompt-workspace__detail-title min-w-0 flex-1 truncate text-sm font-medium text-foreground">
                {creating
                  ? t("promptsView.editor.create")
                  : selectedPrompt?.title || t("promptsView.untitled")}
              </span>
              {!creating && selectedPrompt && (
                <>
                  <div
                    role="tablist"
                    aria-label={t("evaluation.workspaceMode")}
                    className="mr-1 flex rounded-md border border-input p-0.5"
                  >
                    <button
                      type="button"
                      role="tab"
                      aria-selected={workspaceMode === "editor"}
                      onClick={() => setWorkspaceMode("editor")}
                      className={`min-h-8 rounded px-2 text-xs focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring ${
                        workspaceMode === "editor"
                          ? "bg-accent text-foreground"
                          : "text-muted-foreground"
                      }`}
                    >
                      {t("evaluation.editorTab")}
                    </button>
                    <button
                      type="button"
                      role="tab"
                      aria-selected={workspaceMode === "evaluation"}
                      onClick={() => {
                        setWorkspaceMode("evaluation");
                        setShowHistory(false);
                      }}
                      className={`flex min-h-8 items-center gap-1 rounded px-2 text-xs focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring ${
                        workspaceMode === "evaluation"
                          ? "bg-accent text-foreground"
                          : "text-muted-foreground"
                      }`}
                    >
                      <FlaskConicalIcon
                        className="h-3.5 w-3.5"
                        aria-hidden="true"
                      />
                      {t("evaluation.evaluationTab")}
                    </button>
                  </div>
                  {workspaceMode === "editor" && (
                    <button
                      type="button"
                      title={
                        selectedPrompt.isPinned
                          ? t("promptsView.unpin")
                          : t("promptsView.pin")
                      }
                      aria-label={
                        selectedPrompt.isPinned
                          ? t("promptsView.unpin")
                          : t("promptsView.pin")
                      }
                      aria-pressed={selectedPrompt.isPinned}
                      onClick={() =>
                        void savePrompt(selectedPrompt.id, {
                          isPinned: !selectedPrompt.isPinned,
                        })
                      }
                      className={iconButtonClass}
                    >
                      <PinIcon
                        className={`h-4 w-4 ${
                          selectedPrompt.isPinned
                            ? "fill-current text-primary"
                            : ""
                        }`}
                        aria-hidden="true"
                      />
                    </button>
                  )}
                  <button
                    type="button"
                    title={t("promptsView.duplicatePrompt")}
                    aria-label={t("promptsView.duplicatePrompt")}
                    onClick={() => void duplicatePrompt(selectedPrompt.id)}
                    className={iconButtonClass}
                  >
                    <CopyIcon className="h-4 w-4" aria-hidden="true" />
                  </button>
                  <button
                    type="button"
                    title={
                      selectedPrompt.isFavorite
                        ? t("promptsView.unfavorite")
                        : t("promptsView.favorite")
                    }
                    aria-label={
                      selectedPrompt.isFavorite
                        ? t("promptsView.unfavorite")
                        : t("promptsView.favorite")
                    }
                    aria-pressed={selectedPrompt.isFavorite}
                    onClick={() =>
                      void savePrompt(selectedPrompt.id, {
                        isFavorite: !selectedPrompt.isFavorite,
                      })
                    }
                    className={iconButtonClass}
                  >
                    <StarIcon
                      className={`h-4 w-4 ${
                        selectedPrompt.isFavorite ? "fill-current text-primary" : ""
                      }`}
                      aria-hidden="true"
                    />
                  </button>
                  {workspaceMode === "editor" && (
                    <button
                      type="button"
                      title={t("promptsView.history.title")}
                      aria-label={t("promptsView.history.title")}
                      aria-pressed={showHistory}
                      onClick={() => setShowHistory((v) => !v)}
                      className={`${iconButtonClass} transition-colors ${
                        showHistory ? "bg-primary/15 text-foreground" : ""
                      }`}
                    >
                      <HistoryIcon className="h-4 w-4" aria-hidden="true" />
                    </button>
                  )}
                  <button
                    type="button"
                    title={t("promptsView.deletePrompt")}
                    aria-label={t("promptsView.deletePrompt")}
                    onClick={() => handleDeletePrompt(selectedPrompt.id)}
                    className={`${iconButtonClass} hover:bg-destructive/15 hover:text-destructive`}
                  >
                    <Trash2Icon className="h-4 w-4" aria-hidden="true" />
                  </button>
                </>
              )}
            </div>

            <div className="relative flex min-h-0 flex-1 overflow-hidden">
              <div className="min-w-0 flex-1">
                {selectedPrompt?.isLocked ? (
                  <div className="flex h-full flex-col items-center justify-center gap-2 p-8 text-center">
                    <LockIcon
                      className="h-7 w-7 text-muted-foreground"
                      aria-hidden="true"
                    />
                    <h3 className="text-sm font-semibold text-foreground">
                      {t("promptsView.privateLockedTitle")}
                    </h3>
                    <p className="max-w-sm text-sm text-muted-foreground">
                      {t("promptsView.privateLockedHint")}
                    </p>
                  </div>
                ) : workspaceMode === "evaluation" && !creating && selectedPrompt ? (
                  <EvaluationWorkbench prompt={selectedPrompt} versions={versions} />
                ) : (
                  <PromptEditor
                    prompt={creating ? null : selectedPrompt}
                    creating={creating}
                    folders={folders}
                    promptTypeDefinitions={promptTypeDefinitions}
                    knownTags={tags}
                    onCreateFolder={createFolder}
                    onCreatePromptType={createPromptType}
                    onCreate={(input) => {
                      void createPrompt(input).then((created) => {
                        if (created) setCreating(false);
                      });
                    }}
                    onSave={(id, patch) => void savePrompt(id, patch)}
                    onCancelCreate={() => setCreating(false)}
                  />
                )}
              </div>
              {!creating && workspaceMode === "editor" && showHistory && selectedPrompt && (
                <aside
                  aria-label={t("promptsView.history.title")}
                  className="prompt-workspace__history absolute inset-y-0 right-0 z-10 w-[min(20rem,100%)] shrink-0 border-l border-border bg-card shadow-lg"
                >
                  <VersionHistory
                    prompt={selectedPrompt}
                    versions={versions}
                    promptTypeDefinitions={promptTypeDefinitions}
                    onCreateVersion={(note) => void createVersion(note)}
                    onRollback={(version) => {
                      if (
                        window.confirm(
                          t("promptsView.history.restoreConfirm", { version }),
                        )
                      ) {
                        void rollbackVersion(version);
                      }
                    }}
                  />
                </aside>
              )}
            </div>
          </>
        ) : (
          <div className="flex h-full flex-col items-center justify-center gap-3 p-8 text-center">
            {navigationVisible && (
              <button
                type="button"
                onClick={() => setCompactPane("list")}
                className="prompt-workspace__compact-control absolute left-3 top-3 flex h-8 items-center gap-1.5 rounded-md border border-input px-2 text-xs text-muted-foreground hover:bg-accent hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
              >
                <ArrowLeftIcon className="h-4 w-4" aria-hidden="true" />
                {t("common.prompts")}
              </button>
            )}
            <span className="flex h-14 w-14 items-center justify-center rounded-2xl bg-accent text-accent-foreground">
              <CommandIcon className="h-7 w-7" aria-hidden="true" />
            </span>
            <h2 className="text-lg font-semibold text-foreground">
              {t("promptsView.selectPromptTitle")}
            </h2>
            <p className="max-w-sm text-sm text-muted-foreground">
              {t("promptsView.selectPromptHint")}
            </p>
            <button
              type="button"
              onClick={startCreate}
              className="flex items-center gap-1.5 rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground"
            >
              <PlusIcon className="h-4 w-4" aria-hidden="true" />
              {t("promptsView.newPrompt")}
            </button>
          </div>
        )}
      </section>
    </div>
  );
}
