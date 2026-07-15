import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  ChevronLeftIcon,
  ChevronRightIcon,
  CopyIcon,
  CommandIcon,
  DownloadIcon,
  HistoryIcon,
  LockIcon,
  PinIcon,
  PlusIcon,
  StarIcon,
  Trash2Icon,
  UploadIcon,
} from "lucide-react";
import {
  selectSelectedPrompt,
  PROMPT_PAGE_SIZE,
  usePromptStore,
} from "./promptStore";
import type { BundlePreview, Folder, ImportConflictPolicy } from "./types";
import { FolderTree } from "./components/FolderTree";
import { SearchBar } from "./components/SearchBar";
import { PromptList } from "./components/PromptList";
import { PromptEditor } from "./components/PromptEditor";
import { VersionHistory } from "./components/VersionHistory";
import { BatchToolbar } from "./components/BatchToolbar";
import { TagManager } from "./components/TagManager";

/**
 * The prompt-editing view (Req 22.3). Lays out a folder tree (Req 8), a
 * searchable/filterable prompt list (Req 5, 6.3), and a prompt editor with
 * version history (Req 6, 7). All data flows through the prompt store, which
 * routes every backend call through the Runtime_Bridge (Req 3.1).
 */
export function PromptsView() {
  const { t } = useTranslation();

  const folders = usePromptStore((s) => s.folders);
  const prompts = usePromptStore((s) => s.prompts);
  const total = usePromptStore((s) => s.total);
  const offset = usePromptStore((s) => s.offset);
  const tags = usePromptStore((s) => s.tags);
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
  const renameTag = usePromptStore((s) => s.renameTag);
  const deleteTag = usePromptStore((s) => s.deleteTag);
  const exportBundle = usePromptStore((s) => s.exportBundle);
  const previewBundle = usePromptStore((s) => s.previewBundle);
  const importBundle = usePromptStore((s) => s.importBundle);
  const createFolder = usePromptStore((s) => s.createFolder);
  const updateFolder = usePromptStore((s) => s.updateFolder);
  const deleteFolder = usePromptStore((s) => s.deleteFolder);
  const reorderFolders = usePromptStore((s) => s.reorderFolders);
  const createVersion = usePromptStore((s) => s.createVersion);
  const rollbackVersion = usePromptStore((s) => s.rollbackVersion);

  const [creating, setCreating] = useState(false);
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
    void selectPrompt(null);
  };

  const handleDeleteFolder = (folder: Folder) => {
    if (
      window.confirm(
        t("promptsView.deleteFolderConfirm", { name: folder.name }),
      )
    ) {
      void deleteFolder(folder.id);
    }
  };

  const handleDeletePrompt = (id: string) => {
    if (window.confirm(t("promptsView.deletePromptConfirm"))) {
      void deletePrompt(id);
    }
  };

  const editorActive = creating || selectedPrompt != null;

  return (
    <div className="flex h-full w-full">
      {/* Folder tree */}
      <aside className="flex w-56 shrink-0 flex-col border-r border-border bg-card/40 py-2">
        <FolderTree
          folders={folders}
          selectedFolderId={filters.folderId}
          onSelectFolder={(folderId) => void setFilters({ folderId })}
          onCreateFolder={(name, parentId) =>
            void createFolder({ name, parentId })
          }
          onRenameFolder={(id, name) => void updateFolder(id, { name })}
          onDeleteFolder={handleDeleteFolder}
          onReorder={(orderedIds) => void reorderFolders(orderedIds)}
          onReparent={(id, parentId) => void updateFolder(id, { parentId })}
        />
      </aside>

      {/* Prompt list + search */}
      <section className="flex w-80 shrink-0 flex-col border-r border-border">
        <div className="flex flex-col gap-2 border-b border-border p-3">
          <div className="flex items-center justify-between">
            <h2 className="text-sm font-semibold text-foreground">
              {t("common.prompts")}
            </h2>
            <div className="flex items-center gap-1">
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
                className="rounded p-1.5 text-muted-foreground hover:bg-accent hover:text-foreground"
              >
                <DownloadIcon className="h-4 w-4" aria-hidden="true" />
              </button>
              <button
                type="button"
                title={t("promptsView.bundle.import")}
                aria-label={t("promptsView.bundle.import")}
                aria-pressed={showImport}
                onClick={() => setShowImport((open) => !open)}
                className="rounded p-1.5 text-muted-foreground hover:bg-accent hover:text-foreground"
              >
                <UploadIcon className="h-4 w-4" aria-hidden="true" />
              </button>
            <button
              type="button"
              onClick={startCreate}
              className="flex items-center gap-1.5 rounded-md bg-primary px-2.5 py-1.5 text-xs font-medium text-primary-foreground"
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
                {bundlePreview.privatePrompts > 0 && (
                  <p className="mt-1 text-foreground">
                    {t("promptsView.bundle.privateKeyWarning", {
                      count: bundlePreview.privatePrompts,
                    })}
                  </p>
                )}
                <button
                  type="button"
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
                  className="mt-2 rounded bg-primary px-3 py-1.5 text-xs font-medium text-primary-foreground"
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
        <TagManager
          tags={tags}
          onRename={(old, next) => void renameTag(old, next)}
          onDelete={(tag) => {
            if (window.confirm(t("promptsView.tags.deleteConfirm", { tag }))) {
              void deleteTag(tag);
            }
          }}
        />
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
            selectedPromptId={selectedPromptId}
            selectedPromptIds={selectedPromptIds}
            loading={loading}
            onToggleSelection={togglePromptSelection}
            onSelect={(id) => {
              setCreating(false);
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
              className="rounded p-1 text-muted-foreground hover:bg-accent hover:text-foreground disabled:pointer-events-none disabled:opacity-40"
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
              className="rounded p-1 text-muted-foreground hover:bg-accent hover:text-foreground disabled:pointer-events-none disabled:opacity-40"
            >
              <ChevronRightIcon className="h-4 w-4" aria-hidden="true" />
            </button>
          </div>
        </div>
      </section>

      {/* Editor + version history */}
      <section className="flex min-w-0 flex-1 flex-col">
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
            <div className="flex shrink-0 items-center gap-2 border-b border-border px-4 py-2">
              <span className="min-w-0 flex-1 truncate text-sm font-medium text-foreground">
                {creating
                  ? t("promptsView.editor.create")
                  : selectedPrompt?.title || t("promptsView.untitled")}
              </span>
              {!creating && selectedPrompt && (
                <>
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
                    className="rounded p-1.5 text-muted-foreground hover:bg-accent hover:text-foreground"
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
                  <button
                    type="button"
                    title={t("promptsView.duplicatePrompt")}
                    aria-label={t("promptsView.duplicatePrompt")}
                    onClick={() => void duplicatePrompt(selectedPrompt.id)}
                    className="rounded p-1.5 text-muted-foreground hover:bg-accent hover:text-foreground"
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
                    className="rounded p-1.5 text-muted-foreground hover:bg-accent hover:text-foreground"
                  >
                    <StarIcon
                      className={`h-4 w-4 ${
                        selectedPrompt.isFavorite ? "fill-current text-primary" : ""
                      }`}
                      aria-hidden="true"
                    />
                  </button>
                  <button
                    type="button"
                    title={t("promptsView.history.title")}
                    aria-label={t("promptsView.history.title")}
                    aria-pressed={showHistory}
                    onClick={() => setShowHistory((v) => !v)}
                    className={`rounded p-1.5 transition-colors ${
                      showHistory
                        ? "bg-primary/15 text-foreground"
                        : "text-muted-foreground hover:bg-accent hover:text-foreground"
                    }`}
                  >
                    <HistoryIcon className="h-4 w-4" aria-hidden="true" />
                  </button>
                  <button
                    type="button"
                    title={t("promptsView.deletePrompt")}
                    aria-label={t("promptsView.deletePrompt")}
                    onClick={() => handleDeletePrompt(selectedPrompt.id)}
                    className="rounded p-1.5 text-muted-foreground hover:bg-destructive/15 hover:text-destructive"
                  >
                    <Trash2Icon className="h-4 w-4" aria-hidden="true" />
                  </button>
                </>
              )}
            </div>

            <div className="flex min-h-0 flex-1">
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
                ) : (
                  <PromptEditor
                  prompt={creating ? null : selectedPrompt}
                  creating={creating}
                  folders={folders}
                  knownTags={tags}
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
              {!creating && showHistory && selectedPrompt && (
                <aside className="w-72 shrink-0 border-l border-border bg-card/40">
                  <VersionHistory
                    prompt={selectedPrompt}
                    versions={versions}
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
