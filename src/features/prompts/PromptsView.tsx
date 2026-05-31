import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  CommandIcon,
  HistoryIcon,
  PlusIcon,
  StarIcon,
  Trash2Icon,
} from "lucide-react";
import {
  selectSelectedPrompt,
  usePromptStore,
} from "./promptStore";
import type { Folder } from "./types";
import { FolderTree } from "./components/FolderTree";
import { SearchBar } from "./components/SearchBar";
import { PromptList } from "./components/PromptList";
import { PromptEditor } from "./components/PromptEditor";
import { VersionHistory } from "./components/VersionHistory";

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
  const tags = usePromptStore((s) => s.tags);
  const filters = usePromptStore((s) => s.filters);
  const selectedPromptId = usePromptStore((s) => s.selectedPromptId);
  const versions = usePromptStore((s) => s.versions);
  const loading = usePromptStore((s) => s.loading);
  const error = usePromptStore((s) => s.error);
  const selectedPrompt = usePromptStore(selectSelectedPrompt);

  const load = usePromptStore((s) => s.load);
  const setFilters = usePromptStore((s) => s.setFilters);
  const toggleTagFilter = usePromptStore((s) => s.toggleTagFilter);
  const selectPrompt = usePromptStore((s) => s.selectPrompt);
  const createPrompt = usePromptStore((s) => s.createPrompt);
  const savePrompt = usePromptStore((s) => s.savePrompt);
  const deletePrompt = usePromptStore((s) => s.deletePrompt);
  const createFolder = usePromptStore((s) => s.createFolder);
  const updateFolder = usePromptStore((s) => s.updateFolder);
  const deleteFolder = usePromptStore((s) => s.deleteFolder);
  const reorderFolders = usePromptStore((s) => s.reorderFolders);
  const createVersion = usePromptStore((s) => s.createVersion);
  const rollbackVersion = usePromptStore((s) => s.rollbackVersion);
  const deleteVersion = usePromptStore((s) => s.deleteVersion);

  const [creating, setCreating] = useState(false);
  const [showHistory, setShowHistory] = useState(false);

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
            <button
              type="button"
              onClick={startCreate}
              className="flex items-center gap-1.5 rounded-md bg-primary px-2.5 py-1.5 text-xs font-medium text-primary-foreground"
            >
              <PlusIcon className="h-3.5 w-3.5" aria-hidden="true" />
              {t("promptsView.newPrompt")}
            </button>
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
        <div className="min-h-0 flex-1 overflow-y-auto">
          <PromptList
            prompts={prompts}
            selectedPromptId={selectedPromptId}
            loading={loading}
            onSelect={(id) => {
              setCreating(false);
              void selectPrompt(id);
            }}
          />
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
              </div>
              {!creating && showHistory && selectedPrompt && (
                <aside className="w-72 shrink-0 border-l border-border bg-card/40">
                  <VersionHistory
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
                    onDeleteVersion={(versionId) => void deleteVersion(versionId)}
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
