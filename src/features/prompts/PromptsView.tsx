import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  ChevronLeftIcon,
  ChevronRightIcon,
} from "lucide-react";
import {
  selectSelectedPrompt,
  PROMPT_PAGE_SIZE,
  usePromptStore,
} from "./promptStore";
import { LibraryHeader } from "./components/LibraryHeader";
import { LibraryToolbar } from "./components/LibraryToolbar";
import { FilterChips } from "./components/FilterChips";
import { PromptList } from "./components/PromptList";
import { PromptGrid } from "./components/PromptGrid";
import { toLibraryItem } from "./libraryItem";
import { BatchToolbar } from "./components/BatchToolbar";
import { PromptDetailModal } from "./components/detail/PromptDetailModal";
import { useToastStore } from "../notifications/toastStore";

const iconButtonClass =
  "flex h-8 w-8 shrink-0 items-center justify-center rounded-md text-muted-foreground hover:bg-accent hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-40";

/**
 * The prompt-editing view (Req 22.3). Library chrome fills the workspace;
 * prompt detail opens in an overlay.
 */
export function PromptsView() {
  const { t } = useTranslation();

  const folders = usePromptStore((s) => s.folders);
  const prompts = usePromptStore((s) => s.prompts);
  const total = usePromptStore((s) => s.total);
  const offset = usePromptStore((s) => s.offset);
  const tags = usePromptStore((s) => s.tags);
  const promptTypeDefinitions = usePromptStore((s) => s.promptTypeDefinitions);
  const batchMode = usePromptStore((s) => s.batchMode);
  const viewMode = usePromptStore((s) => s.viewMode);
  const selectedPromptId = usePromptStore((s) => s.selectedPromptId);
  const versions = usePromptStore((s) => s.versions);
  const loading = usePromptStore((s) => s.loading);
  const error = usePromptStore((s) => s.error);
  const selectedPrompt = usePromptStore(selectSelectedPrompt);
  const selectedPromptIds = usePromptStore((s) => s.selectedPromptIds);

  const load = usePromptStore((s) => s.load);
  const setBatchMode = usePromptStore((s) => s.setBatchMode);
  const resetLibraryFilters = usePromptStore((s) => s.resetLibraryFilters);
  const loadPreviousPage = usePromptStore((s) => s.loadPreviousPage);
  const loadNextPage = usePromptStore((s) => s.loadNextPage);
  const requestSelectPrompt = usePromptStore((s) => s.requestSelectPrompt);
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
  const createFolder = usePromptStore((s) => s.createFolder);
  const createPromptType = usePromptStore((s) => s.createPromptType);
  const createVersion = usePromptStore((s) => s.createVersion);
  const rollbackVersion = usePromptStore((s) => s.rollbackVersion);

  const [creating, setCreating] = useState(false);
  const registerCreatePromptAction = usePromptStore(
    (s) => s.registerCreatePromptAction,
  );

  useEffect(() => {
    void load();
  }, [load]);

  const startCreate = () => {
    void requestSelectPrompt(null).then((ok) => {
      if (!ok) return;
      setCreating(true);
    });
  };

  useEffect(() => {
    registerCreatePromptAction(startCreate);
    return () => registerCreatePromptAction(null);
  });

  const handleDeletePrompt = (id: string) => {
    if (window.confirm(t("promptsView.deletePromptConfirm"))) {
      void deletePrompt(id);
      setCreating(false);
    }
  };

  const overlayOpen = creating || selectedPrompt != null;
  const libraryItems = useMemo(
    () => prompts.map((prompt) => toLibraryItem(prompt, promptTypeDefinitions, t)),
    [prompts, promptTypeDefinitions, t],
  );
  const libraryScrollRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (libraryScrollRef.current) libraryScrollRef.current.scrollTop = 0;
  }, [viewMode]);

  useEffect(() => {
    if (selectedPromptId != null) setCreating(false);
  }, [selectedPromptId]);

  return (
    <div className="prompt-workspace relative flex h-full min-h-0 w-full overflow-hidden">
      <section
        aria-label={t("common.prompts")}
        className="flex min-w-0 w-full flex-1 flex-col"
      >
        <LibraryHeader onCreate={startCreate} />
        <LibraryToolbar />
        <FilterChips />
        {error && (
          <div
            role="alert"
            className="border-b border-destructive/40 bg-destructive/10 px-4 py-2 text-sm text-destructive"
          >
            {error}
          </div>
        )}
        {batchMode && (
          <BatchToolbar
            selectedCount={selectedPromptIds.length}
            folders={folders}
            onSelectPage={selectPage}
            onClear={clearPromptSelection}
            onMove={(folderId) => void batchMove(folderId)}
            onTag={(selectedTags) => void batchTag(selectedTags)}
            onDelete={() => {
              if (window.confirm(t("promptsView.batch.deleteConfirm"))) {
                void batchDelete().then(() => {
                  useToastStore.getState().push({
                    message: t("promptsView.toast.batchDeleted"),
                    tone: "success",
                  });
                });
              }
            }}
            onExit={() => setBatchMode(false)}
          />
        )}
        <div ref={libraryScrollRef} className="min-h-0 flex-1 overflow-y-auto">
          {loading ? (
            <div className="flex h-full items-center justify-center p-6 text-sm text-muted-foreground">
              {t("promptsView.loading")}
            </div>
          ) : prompts.length === 0 ? (
            <div className="flex h-full flex-col items-center justify-center gap-2 p-6 text-center">
              <p className="text-sm font-medium text-foreground">{t("promptsView.noPrompts")}</p>
              <p className="max-w-xs text-xs text-muted-foreground">{t("promptsView.noPromptsHint")}</p>
              <button
                type="button"
                onClick={() => void resetLibraryFilters()}
                className="mt-1 rounded-md border border-input px-2 py-1 text-xs text-foreground hover:bg-accent"
              >
                {t("promptsView.chrome.clearAll")}
              </button>
            </div>
          ) : viewMode === "grid" ? (
            <PromptGrid
              items={libraryItems}
              selectedPromptId={selectedPromptId}
              selectedPromptIds={selectedPromptIds}
              batchMode={batchMode}
              onToggleSelection={togglePromptSelection}
              onToggleFavorite={(id, next) => void savePrompt(id, { isFavorite: next })}
              onSelect={(id) => {
                void requestSelectPrompt(id);
              }}
            />
          ) : (
            <PromptList
              items={libraryItems}
              selectedPromptId={selectedPromptId}
              selectedPromptIds={selectedPromptIds}
              batchMode={batchMode}
              onToggleSelection={togglePromptSelection}
              onToggleFavorite={(id, next) => void savePrompt(id, { isFavorite: next })}
              onSelect={(id) => {
                void requestSelectPrompt(id);
              }}
            />
          )}
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
              disabled={loading || offset + PROMPT_PAGE_SIZE >= total}
              onClick={() => void loadNextPage()}
              className={iconButtonClass}
            >
              <ChevronRightIcon className="h-4 w-4" aria-hidden="true" />
            </button>
          </div>
        </div>
      </section>

      <PromptDetailModal
        open={overlayOpen}
        creating={creating}
        prompt={creating ? null : selectedPrompt}
        prompts={prompts}
        versions={versions}
        folders={folders}
        promptTypeDefinitions={promptTypeDefinitions}
        knownTags={tags}
        onClose={() => {
          setCreating(false);
          void selectPrompt(null);
        }}
        onCreate={async (input) => {
          const created = await createPrompt(input);
          if (created) setCreating(false);
          return created;
        }}
        onSave={(id, patch) => savePrompt(id, patch)}
        onCreateFolder={createFolder}
        onCreatePromptType={createPromptType}
        onToggleFavorite={(id, next) => void savePrompt(id, { isFavorite: next })}
        onTogglePin={(id, next) => void savePrompt(id, { isPinned: next })}
        onDuplicate={(id) => void duplicatePrompt(id)}
        onDelete={handleDeletePrompt}
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
    </div>
  );
}
