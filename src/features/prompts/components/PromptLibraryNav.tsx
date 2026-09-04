import { useMemo, useState, type KeyboardEvent } from "react";
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

import { Tag, useConfirm } from "../../../components/ui";
import { cn } from "../../../components/ui/cn";

const SAVED_VIEWS: { view: SavedView; icon: LucideIcon }[] = [
  { view: "all", icon: InboxIcon },
  { view: "favorites", icon: StarIcon },
  { view: "recent", icon: ClockIcon },
];

/** How many tag chips to show before the cloud collapses (plan §6.4). */
export const TAG_CLOUD_PREVIEW = 8;

const NAV_ROW =
  "relative flex h-control-md w-full items-center gap-2 rounded-md px-2 text-left text-body transition-colors duration-fast ease-out";

function SelectionAccent({ active }: { active: boolean }) {
  if (!active) return null;
  return (
    <span
      aria-hidden="true"
      className="absolute inset-y-1.5 left-0 w-0.5 rounded-full bg-primary"
    />
  );
}

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

  const [tagsExpanded, setTagsExpanded] = useState(false);

  const scope = resolveLibraryScope({ activeView, filters, folders });
  const sortedTags = useMemo(() => {
    return [...tags].sort((left, right) => {
      const byCount =
        (libraryCounts.tags[right] ?? 0) - (libraryCounts.tags[left] ?? 0);
      return byCount !== 0 ? byCount : left.localeCompare(right);
    });
  }, [tags, libraryCounts.tags]);
  const visibleTags =
    tagsExpanded || sortedTags.length <= TAG_CLOUD_PREVIEW
      ? sortedTags
      : sortedTags.slice(0, TAG_CLOUD_PREVIEW);
  const hiddenTagCount = Math.max(0, sortedTags.length - TAG_CLOUD_PREVIEW);

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
              className={cn(
                "relative flex h-10 w-10 flex-col items-center justify-center rounded-lg text-micro font-mono",
                current
                  ? "bg-state-selected text-foreground"
                  : "text-sidebar-foreground/70 hover:bg-sidebar-accent",
              )}
            >
              <SelectionAccent active={current} />
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
        <h2 className="px-2 pb-1 text-meta font-medium text-muted-foreground-subtle">
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
                className={cn(
                  NAV_ROW,
                  current
                    ? "bg-state-selected text-foreground"
                    : "text-sidebar-foreground/80 hover:bg-sidebar-accent hover:text-sidebar-foreground",
                )}
              >
                <SelectionAccent active={current} />
                <Icon className="h-4 w-4 shrink-0" aria-hidden="true" />
                <span className="min-w-0 flex-1 truncate">
                  {t(`promptsView.library.${view}`)}
                </span>
                {count != null && (
                  <span
                    className="text-meta tabular-nums text-muted-foreground-subtle"
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
        <h2 className="px-2 pb-1 text-meta font-medium text-muted-foreground-subtle">
          {t("promptsView.library.tags")}
        </h2>
        <div
          role="toolbar"
          aria-label={t("promptsView.library.tags")}
          tabIndex={sortedTags.length > 0 ? 0 : undefined}
          onKeyDown={handleTagKeyDown}
          className="flex flex-wrap gap-1 px-1"
        >
          {visibleTags.map((tag) => {
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
        {hiddenTagCount > 0 && (
          <button
            type="button"
            onClick={() => setTagsExpanded((open) => !open)}
            className="mt-1 px-2 text-meta text-muted-foreground transition-colors duration-fast ease-out hover:text-foreground"
          >
            {tagsExpanded
              ? t("promptsView.library.showFewerTags")
              : t("promptsView.library.showMoreTags", { count: hiddenTagCount })}
          </button>
        )}
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
