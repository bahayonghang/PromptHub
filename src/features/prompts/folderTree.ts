/**
 * Pure helpers for turning the flat `folder.list` result into a renderable tree
 * and computing drag/reorder outcomes (Req 8.2, 8.5). Kept free of React so the
 * ordering and cycle-safety rules can be unit-tested directly.
 */
import type { Folder } from "./types";

/** A folder plus its resolved children and depth, ready for indented rendering. */
export interface FolderTreeNode extends Folder {
  children: FolderTreeNode[];
  depth: number;
}

/**
 * Builds the folder forest from a flat list (Req 8.2). Siblings are ordered by
 * `sortOrder` ascending, then `name` as a stable tiebreaker. A folder whose
 * `parentId` is missing or points at an unknown id is treated as a root, so an
 * orphaned subtree is never dropped. Cycles are broken defensively: a folder
 * already placed is never revisited, so a corrupt parent chain cannot loop.
 */
export function buildFolderTree(folders: readonly Folder[]): FolderTreeNode[] {
  const nodes = new Map<string, FolderTreeNode>();
  for (const folder of folders) {
    nodes.set(folder.id, { ...folder, children: [], depth: 0 });
  }

  const roots: FolderTreeNode[] = [];
  for (const folder of folders) {
    const node = nodes.get(folder.id)!;
    const parent =
      folder.parentId != null ? nodes.get(folder.parentId) : undefined;
    if (parent && parent.id !== node.id) {
      parent.children.push(node);
    } else {
      roots.push(node);
    }
  }

  const compare = (a: FolderTreeNode, b: FolderTreeNode) =>
    a.sortOrder - b.sortOrder || a.name.localeCompare(b.name);

  const visited = new Set<string>();
  const assignDepth = (siblings: FolderTreeNode[], depth: number) => {
    siblings.sort(compare);
    for (const node of siblings) {
      if (visited.has(node.id)) {
        // Defensive: a cycle in `parentId` would otherwise recurse forever.
        node.children = [];
        continue;
      }
      visited.add(node.id);
      node.depth = depth;
      assignDepth(node.children, depth + 1);
    }
  };

  assignDepth(roots, 0);

  // Defensive: any node unreachable from a root belongs to a `parentId` cycle.
  // Surface each such node as a root so no folder is silently dropped.
  for (const folder of folders) {
    const node = nodes.get(folder.id)!;
    if (!visited.has(node.id)) {
      node.children = [];
      node.depth = 0;
      visited.add(node.id);
      roots.push(node);
    }
  }

  return roots;
}

/** Flattens a folder forest into a pre-order list for flat rendering. */
export function flattenFolderTree(
  nodes: readonly FolderTreeNode[],
): FolderTreeNode[] {
  const out: FolderTreeNode[] = [];
  const walk = (siblings: readonly FolderTreeNode[]) => {
    for (const node of siblings) {
      out.push(node);
      walk(node.children);
    }
  };
  walk(nodes);
  return out;
}

/** Returns the ids of `folderId` and all of its descendants (Req 8.4 preview). */
export function collectSubtreeIds(
  folders: readonly Folder[],
  folderId: string,
): Set<string> {
  const childrenByParent = new Map<string, string[]>();
  for (const folder of folders) {
    if (folder.parentId != null) {
      const list = childrenByParent.get(folder.parentId) ?? [];
      list.push(folder.id);
      childrenByParent.set(folder.parentId, list);
    }
  }

  const ids = new Set<string>();
  const stack = [folderId];
  while (stack.length > 0) {
    const id = stack.pop()!;
    if (ids.has(id)) continue;
    ids.add(id);
    for (const child of childrenByParent.get(id) ?? []) {
      stack.push(child);
    }
  }
  return ids;
}

/**
 * Returns whether moving `folderId` under `targetParentId` would create a cycle,
 * i.e. the target is the folder itself or one of its descendants (Req 8.9). The
 * frontend uses this to disable invalid drop targets before calling the backend.
 */
export function wouldCreateCycle(
  folders: readonly Folder[],
  folderId: string,
  targetParentId: string | null,
): boolean {
  if (targetParentId == null) return false;
  return collectSubtreeIds(folders, folderId).has(targetParentId);
}

/**
 * Computes the new sibling id order when `draggedId` is dropped onto `targetId`
 * within the same parent group, as the ordered id list to pass to
 * `folder.reorder` (Req 8.5). The dragged folder is removed and re-inserted at
 * the target's position. Returns the input order unchanged when either id is
 * absent or the two are identical.
 */
export function reorderSiblings(
  orderedIds: readonly string[],
  draggedId: string,
  targetId: string,
): string[] {
  if (draggedId === targetId) return [...orderedIds];
  const from = orderedIds.indexOf(draggedId);
  const to = orderedIds.indexOf(targetId);
  if (from === -1 || to === -1) return [...orderedIds];

  const next = [...orderedIds];
  next.splice(from, 1);
  const insertAt = next.indexOf(targetId);
  // Insert before the target when dragging downward, after when dragging upward,
  // so the dragged item visually lands at the target's slot.
  next.splice(from < to ? insertAt + 1 : insertAt, 0, draggedId);
  return next;
}

/** Returns the root-level folder ids in render order (Req 8.5 reorder scope). */
export function rootFolderIdsInOrder(folders: readonly Folder[]): string[] {
  return folders
    .filter((f) => f.parentId == null)
    .sort((a, b) => a.sortOrder - b.sortOrder || a.name.localeCompare(b.name))
    .map((f) => f.id);
}
