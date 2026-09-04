import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  ChevronDownIcon,
  ChevronRightIcon,
  FilesIcon,
  FolderIcon,
  FolderPlusIcon,
  PencilIcon,
  Trash2Icon,
} from "lucide-react";
import {
  buildFolderTree,
  reorderSiblings,
  rootFolderIdsInOrder,
  wouldCreateCycle,
  type FolderTreeNode,
} from "../folderTree";
import type { Folder } from "../types";

interface FolderTreeProps {
  folders: Folder[];
  /** The active folder filter, or `null` for "all prompts". */
  selectedFolderId: string | null;
  onSelectFolder: (folderId: string | null) => void;
  onCreateFolder: (name: string, parentId: string | null) => void;
  onRenameFolder: (id: string, name: string) => void;
  onDeleteFolder: (folder: Folder) => void;
  /** Persists a new root-level order after a drag/drop (Req 8.5). */
  onReorder: (orderedIds: string[]) => void;
  /** Reparents a folder onto a drop target, or to root when `null` (Req 8.3). */
  onReparent: (id: string, parentId: string | null) => void;
  /** Direct-membership totals keyed by folder id. */
  counts?: Record<string, number>;
}

/** Decorative hue derived from the folder id so the same folder keeps the same dot. */
export function folderAccentHue(id: string): number {
  let hash = 2166136261;
  for (let i = 0; i < id.length; i += 1) {
    hash ^= id.charCodeAt(i);
    hash = Math.imul(hash, 16777619);
  }
  return (hash >>> 0) % 360;
}

/**
 * The folder tree sidebar (Req 8.2). Renders the hierarchical folder list with
 * expand/collapse, inline create/rename/delete, and drag-and-drop that either
 * reorders root siblings (Req 8.5) or reparents a folder onto a drop target
 * (Req 8.3), guarding against cycles (Req 8.9). All labels come from i18n.
 */
export function FolderTree({
  folders,
  selectedFolderId,
  onSelectFolder,
  onCreateFolder,
  onRenameFolder,
  onDeleteFolder,
  onReorder,
  onReparent,
  counts,
}: FolderTreeProps) {
  const { t } = useTranslation();
  const tree = useMemo(() => buildFolderTree(folders), [folders]);

  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const [creatingUnder, setCreatingUnder] = useState<string | null | undefined>(
    undefined,
  );
  const [newName, setNewName] = useState("");
  const [renamingId, setRenamingId] = useState<string | null>(null);
  const [renameValue, setRenameValue] = useState("");
  const [dragId, setDragId] = useState<string | null>(null);
  const [dropTargetId, setDropTargetId] = useState<string | null>(null);

  const toggleExpanded = (id: string) =>
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });

  const submitCreate = () => {
    const name = newName.trim();
    if (name !== "") onCreateFolder(name, creatingUnder ?? null);
    setNewName("");
    setCreatingUnder(undefined);
  };

  const submitRename = (id: string) => {
    const name = renameValue.trim();
    if (name !== "") onRenameFolder(id, name);
    setRenamingId(null);
    setRenameValue("");
  };

  const handleDrop = (target: FolderTreeNode | null) => {
    setDropTargetId(null);
    const id = dragId;
    setDragId(null);
    if (id == null) return;

    if (target == null) {
      // Dropped on the root area: move to root and place last.
      onReparent(id, null);
      return;
    }
    if (id === target.id) return;

    const dragged = folders.find((f) => f.id === id);
    const sameParent =
      (dragged?.parentId ?? null) === (target.parentId ?? null);
    if (sameParent && target.parentId == null) {
      // Reorder among root siblings (Req 8.5).
      onReorder(reorderSiblings(rootFolderIdsInOrder(folders), id, target.id));
    } else if (!wouldCreateCycle(folders, id, target.id)) {
      // Reparent onto the target folder (Req 8.3, guarded by Req 8.9).
      onReparent(id, target.id);
    }
  };

  const renderNode = (node: FolderTreeNode) => {
    const hasChildren = node.children.length > 0;
    const isExpanded = expanded.has(node.id);
    const isSelected = selectedFolderId === node.id;
    const isDropTarget = dropTargetId === node.id;

    return (
      <li key={node.id}>
        <div
          draggable={renamingId !== node.id}
          onDragStart={() => setDragId(node.id)}
          onDragEnd={() => {
            setDragId(null);
            setDropTargetId(null);
          }}
          onDragOver={(e) => {
            if (dragId != null && dragId !== node.id) {
              e.preventDefault();
              setDropTargetId(node.id);
            }
          }}
          onDragLeave={() =>
            setDropTargetId((cur) => (cur === node.id ? null : cur))
          }
          onDrop={(e) => {
            e.preventDefault();
            e.stopPropagation();
            handleDrop(node);
          }}
          style={{ paddingLeft: `${node.depth * 12 + 8}px` }}
          className={`group flex items-center gap-1 rounded-md py-1.5 pr-2 text-body transition-colors ${
            isSelected
              ? "bg-state-selected text-foreground"
              : "text-muted-foreground hover:bg-accent hover:text-foreground"
          } ${isDropTarget ? "ring-1 ring-primary" : ""}`}
        >
          <button
            type="button"
            aria-label={isExpanded ? t("common.collapse") : t("common.expand")}
            onClick={() => hasChildren && toggleExpanded(node.id)}
            className={`flex h-4 w-4 shrink-0 items-center justify-center ${
              hasChildren ? "" : "invisible"
            }`}
          >
            {isExpanded ? (
              <ChevronDownIcon className="h-3.5 w-3.5" aria-hidden="true" />
            ) : (
              <ChevronRightIcon className="h-3.5 w-3.5" aria-hidden="true" />
            )}
          </button>

          {renamingId === node.id ? (
            <input
              autoFocus
              value={renameValue}
              onChange={(e) => setRenameValue(e.target.value)}
              onBlur={() => submitRename(node.id)}
              onKeyDown={(e) => {
                if (e.key === "Enter") submitRename(node.id);
                if (e.key === "Escape") {
                  setRenamingId(null);
                  setRenameValue("");
                }
              }}
              className="min-w-0 flex-1 rounded-sm border border-input bg-background px-1 py-0.5 text-body text-foreground outline-none"
            />
          ) : (
            <>
              <button
                type="button"
                onClick={() => onSelectFolder(node.id)}
                aria-pressed={isSelected}
                className="flex min-w-0 flex-1 items-center gap-2 text-left"
              >
                <span
                  aria-hidden="true"
                  className="h-2 w-2 shrink-0 rounded-full"
                  style={{
                    backgroundColor: `hsl(${folderAccentHue(node.id)} 42% 58%)`,
                  }}
                />
                <FolderIcon className="h-4 w-4 shrink-0" aria-hidden="true" />
                <span className="truncate">{node.name}</span>
                {counts?.[node.id] != null && (
                  <span
                    className="ml-auto font-mono text-label text-muted-foreground-subtle"
                    title={t("promptsView.library.bucketCount", {
                      count: counts[node.id],
                    })}
                  >
                    {counts[node.id]}
                  </span>
                )}
              </button>
              <span className="hidden shrink-0 items-center gap-0.5 group-hover:flex">
                <button
                  type="button"
                  title={t("promptsView.newFolder")}
                  aria-label={t("promptsView.newFolder")}
                  onClick={() => {
                    setExpanded((p) => new Set(p).add(node.id));
                    setCreatingUnder(node.id);
                    setNewName("");
                  }}
                  className="rounded-sm p-1 hover:bg-accent hover:text-foreground"
                >
                  <FolderPlusIcon className="h-3.5 w-3.5" aria-hidden="true" />
                </button>
                <button
                  type="button"
                  title={t("promptsView.renameFolder")}
                  aria-label={t("promptsView.renameFolder")}
                  onClick={() => {
                    setRenamingId(node.id);
                    setRenameValue(node.name);
                  }}
                  className="rounded-sm p-1 hover:bg-accent hover:text-foreground"
                >
                  <PencilIcon className="h-3.5 w-3.5" aria-hidden="true" />
                </button>
                <button
                  type="button"
                  title={t("promptsView.deleteFolder")}
                  aria-label={t("promptsView.deleteFolder")}
                  onClick={() => onDeleteFolder(node)}
                  className="rounded-sm p-1 hover:bg-destructive/15 hover:text-destructive"
                >
                  <Trash2Icon className="h-3.5 w-3.5" aria-hidden="true" />
                </button>
              </span>
            </>
          )}
        </div>

        {creatingUnder === node.id && (
          <div style={{ paddingLeft: `${(node.depth + 1) * 12 + 28}px` }}>
            <input
              autoFocus
              value={newName}
              placeholder={t("promptsView.folderNamePlaceholder")}
              onChange={(e) => setNewName(e.target.value)}
              onBlur={submitCreate}
              onKeyDown={(e) => {
                if (e.key === "Enter") submitCreate();
                if (e.key === "Escape") {
                  setNewName("");
                  setCreatingUnder(undefined);
                }
              }}
              className="my-1 w-[calc(100%-0.5rem)] rounded-sm border border-input bg-background px-1.5 py-0.5 text-body text-foreground outline-none"
            />
          </div>
        )}

        {hasChildren && isExpanded && (
          <ul>{node.children.map(renderNode)}</ul>
        )}
      </li>
    );
  };

  return (
    <div
      className="flex h-full flex-col"
      onDragOver={(e) => {
        if (dragId != null) e.preventDefault();
      }}
      onDrop={(e) => {
        e.preventDefault();
        handleDrop(null);
      }}
    >
      <div className="flex items-center justify-between px-2 py-1">
        <span className="text-label font-semibold uppercase tracking-wide text-muted-foreground">
          {t("promptsView.folders")}
        </span>
        <button
          type="button"
          title={t("promptsView.newFolder")}
          aria-label={t("promptsView.newFolder")}
          onClick={() => {
            setCreatingUnder(null);
            setNewName("");
          }}
          className="rounded-sm p-1 text-muted-foreground hover:bg-accent hover:text-foreground"
        >
          <FolderPlusIcon className="h-4 w-4" aria-hidden="true" />
        </button>
      </div>

      <button
        type="button"
        onClick={() => onSelectFolder(null)}
        className={`mx-1 flex items-center gap-2 rounded-md px-2 py-1.5 text-left text-body transition-colors ${
          selectedFolderId == null
            ? "bg-state-selected text-foreground"
            : "text-muted-foreground hover:bg-accent hover:text-foreground"
        }`}
      >
        <FilesIcon className="h-4 w-4 shrink-0" aria-hidden="true" />
        <span className="truncate">{t("promptsView.allFolders")}</span>
      </button>

      <ul className="mt-1 flex-1 overflow-y-auto px-1">
        {tree.map(renderNode)}
        {tree.length === 0 && creatingUnder === undefined && (
          <li className="px-2 py-2 text-label text-muted-foreground">
            {t("promptsView.emptyFolders")}
          </li>
        )}
        {creatingUnder === null && (
          <li className="px-1">
            <input
              autoFocus
              value={newName}
              placeholder={t("promptsView.folderNamePlaceholder")}
              onChange={(e) => setNewName(e.target.value)}
              onBlur={submitCreate}
              onKeyDown={(e) => {
                if (e.key === "Enter") submitCreate();
                if (e.key === "Escape") {
                  setNewName("");
                  setCreatingUnder(undefined);
                }
              }}
              className="my-1 w-full rounded-sm border border-input bg-background px-1.5 py-1 text-body text-foreground outline-none"
            />
          </li>
        )}
      </ul>
    </div>
  );
}
