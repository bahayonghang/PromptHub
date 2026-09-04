import { useMemo, type KeyboardEvent } from "react";
import { useTranslation } from "react-i18next";
import {
  ClockIcon,
  InboxIcon,
  StarIcon,
  type LucideIcon,
} from "lucide-react";
import { useAppStore } from "../../../store/appStore";
import {
  resolveLibraryScope,
  usePromptStore,
  type SavedView,
} from "../promptStore";
import type { Folder } from "../types";
import { FolderTree } from "./FolderTree";
import { TagManager } from "./TagManager";

import { Tag, useConfirm} from "../../../components/ui";
const SAVED_VIEWS: { view: SavedView; icon: LucideIcon }[] = [
  { view: "all", icon: InboxIcon },
  { view: "favorites", icon: StarIcon },
  { view: "recent", icon: ClockIcon },
];

/**
 * Feature-owned library rail: saved views, folder tree, and tag cloud.
 * Mounted in the app sidebar slot; it does not live in PromptsView.
 */
export function PromptLibraryNav() {
  const { confirm, confirmDialog } = useConfirm();

  const { t } = useTranslation();
  const collapsed = useAppStore((state) => state.sidebarCollapsed);
  const setActiveView = useAppStore((state) => state.setActiveView);

  const folders = usePromptStore((state) => state.folders);
  const tags = usePromptStore((state) => state.tags);
  const filters = usePromptStore((state) => state.filters);
  const activeView = usePromptStore((state) => state.activeView);
  const libraryCounts = usePromptStore((state) => state.libraryCounts);
  const selectView = usePromptStore((state) => state.selectView);
  const selectFolder = usePromptStore((state) => state.selectFolder);
  const toggleTagFilter = usePromptStore((state) => state.toggleTagFilter);
  const createFolder = usePromptStore((state) => state.createFolder);
  const updateFolder = usePromptStore((state) => state.updateFolder);
  const deleteFolder = usePromptStore((state) => state.deleteFolder);
  const reorderFolders = usePromptStore((state) => state.reorderFolders);
  const renameTag = usePromptStore((state) => state.renameTag);
  const deleteTag = usePromptStore((state) => state.deleteTag);

  const scope = resolveLibraryScope({ activeView, filters, folders });
  const sortedTags = useMemo(() => {
    return [...tags].sort((left, right) => {
      const byCount =
        (libraryCounts.tags[right] ?? 0) - (libraryCounts.tags[left] ?? 0);
      return byCount !== 0 ? byCount : left.localeCompare(right);
    });
  }, [tags, libraryCounts.tags]);

  const openLibrary = () => setActiveView("prompts");

  const handleSelectView = (view: SavedView) => {
    openLibrary();
    void selectView(view);
  };

  const handleSelectFolder = (folderId: string | null) => {
    openLibrary();
    void selectFolder(folderId);
  };

  const handleToggleTag = (tag: string) => {
    openLibrary();
    void toggleTagFilter(tag);
  };

  const handleDeleteFolder = async (folder: Folder) => {
    if (
      await confirm({
        title: t("promptsView.deleteFolderTitle"),
        message: t("promptsView.deleteFolderConfirm", { name: folder.name }),
        destructive: true,
      })
    ) {
      void deleteFolder(folder.id);
    }
  };

  const handleTagKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (sortedTags.length === 0) return;
    if (event.key !== "ArrowRight" && event.key !== "ArrowLeft") return;
    event.preventDefault();
    const chips = Array.from(
      event.currentTarget.querySelectorAll<HTMLButtonElement>("[data-tag-chip]"),
    );
    const current = chips.findIndex((chip) => chip === document.activeElement);
    const delta = event.key === "ArrowRight" ? 1 : -1;
    const next = chips[(current + delta + chips.length) % chips.length];
    next?.focus();
  };

  if (collapsed) {
    const scopeLabel =
      scope.kind === "view"
        ? t(`promptsView.library.${scope.view}`)
        : scope.kind === "folder"
          ? scope.folder.name
          : scope.kind === "tag"
            ? scope.tag
            : t("promptsView.library.all");
    return (
      <nav
        className="flex flex-1 flex-col items-center gap-1 overflow-y-auto p-1"
        aria-label={t("promptsView.library.nav")}
      >
        {SAVED_VIEWS.map(({ view, icon: Icon }) => {
          const current = activeView === view;
          const count = view === "recent" ? undefined : libraryCounts.views[view];
          return (
            <button
              key={view}
              type="button"
              title={t(`promptsView.library.${view}`)}
              aria-label={t(`promptsView.library.${view}`)}
              aria-current={current ? "true" : undefined}
              onClick={() => handleSelectView(view)}
              className={`flex h-10 w-10 flex-col items-center justify-center rounded-lg text-micro font-mono ${
                current
                  ? "bg-primary/15 text-foreground"
                  : "text-sidebar-foreground/70 hover:bg-sidebar-accent"
              }`}
            >
              <Icon className="h-4 w-4" aria-hidden="true" />
              {count != null && <span aria-hidden="true">{count}</span>}
            </button>
          );
        })}
        <p className="sr-only">{scopeLabel}</p>
      </nav>
    );
  }

  return (
    <nav
      className="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto p-2"
      aria-label={t("promptsView.library.nav")}
    >
      <section aria-label={t("promptsView.library.savedViews")}>
        <h2 className="px-2 pb-1 text-meta font-semibold uppercase tracking-wide text-muted-foreground">
          {t("promptsView.library.savedViews")}
        </h2>
        <div role="list" className="flex flex-col gap-0.5">
          {SAVED_VIEWS.map(({ view, icon: Icon }) => {
            const current = activeView === view;
            const count = view === "recent" ? undefined : libraryCounts.views[view];
            return (
              <button
                key={view}
                type="button"
                role="listitem"
                aria-current={current ? "true" : undefined}
                onClick={() => handleSelectView(view)}
                className={`flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-body ${
                  current
                    ? "bg-primary/15 text-foreground"
                    : "text-sidebar-foreground/80 hover:bg-sidebar-accent hover:text-sidebar-foreground"
                }`}
              >
                <Icon className="h-4 w-4 shrink-0" aria-hidden="true" />
                <span className="min-w-0 flex-1 truncate">
                  {t(`promptsView.library.${view}`)}
                </span>
                {count != null && (
                  <span
                    className="font-mono text-label text-muted-foreground-subtle"
                    aria-label={t("promptsView.library.bucketCount", { count })}
                  >
                    {count}
                  </span>
                )}
              </button>
            );
          })}
        </div>
      </section>

      <section className="min-h-0 flex-1" aria-label={t("promptsView.library.folders")}>
        <FolderTree
          folders={folders}
          selectedFolderId={filters.folderId}
          counts={libraryCounts.folders}
          onSelectFolder={handleSelectFolder}
          onCreateFolder={(name, parentId) => void createFolder({ name, parentId })}
          onRenameFolder={(id, name) => void updateFolder(id, { name })}
          onDeleteFolder={handleDeleteFolder}
          onReorder={(orderedIds) => void reorderFolders(orderedIds)}
          onReparent={(id, parentId) => void updateFolder(id, { parentId })}
        />
      </section>

      <section aria-label={t("promptsView.library.tags")}>
        <h2 className="px-2 pb-1 text-meta font-semibold uppercase tracking-wide text-muted-foreground">
          {t("promptsView.library.tags")}
        </h2>
        <div
          role="toolbar"
          aria-label={t("promptsView.library.tags")}
          tabIndex={sortedTags.length > 0 ? 0 : undefined}
          onKeyDown={handleTagKeyDown}
          className="flex flex-wrap gap-1 px-1"
        >
          {sortedTags.map((tag) => {
            const pressed = filters.tags.includes(tag);
            const count = libraryCounts.tags[tag];
            return (
              <Tag
                key={tag}
                name={tag}
                data-tag-chip=""
                pressed={pressed}
                onToggle={() => handleToggleTag(tag)}
                count={count ?? undefined}
                countLabel={
                  count != null
                    ? t("promptsView.library.bucketCount", { count })
                    : undefined
                }
              />
            );
          })}
        </div>
        <TagManager
          tags={tags}
          onRename={(old, next) => void renameTag(old, next)}
          onDelete={(tag) => {
            void (async () => {
              if (
                await confirm({
                  title: t("promptsView.tags.deleteTitle"),
                  message: t("promptsView.tags.deleteConfirm", { tag }),
                  destructive: true,
                })
              ) {
                void deleteTag(tag);
              }
            })();
          }}
        />
      </section>
      {confirmDialog}
    </nav>
  );
}
