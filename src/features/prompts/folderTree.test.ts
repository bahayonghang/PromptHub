import { describe, expect, it } from "vitest";
import fc from "fast-check";
import {
  buildFolderTree,
  collectSubtreeIds,
  flattenFolderTree,
  reorderSiblings,
  rootFolderIdsInOrder,
  wouldCreateCycle,
} from "./folderTree";
import type { Folder } from "./types";

/** Minimal folder factory with sensible defaults for the fields under test. */
function folder(partial: Partial<Folder> & { id: string }): Folder {
  return {
    name: partial.id,
    parentId: null,
    sortOrder: 0,
    createdAt: "2024-01-01T00:00:00.000Z",
    updatedAt: null,
    icon: null,
    ...partial,
  };
}

describe("buildFolderTree (Req 8.2)", () => {
  it("nests children under their parents and assigns depth", () => {
    const folders = [
      folder({ id: "a", sortOrder: 0 }),
      folder({ id: "b", parentId: "a", sortOrder: 0 }),
      folder({ id: "c", parentId: "b", sortOrder: 0 }),
    ];
    const tree = buildFolderTree(folders);
    expect(tree).toHaveLength(1);
    expect(tree[0].id).toBe("a");
    expect(tree[0].depth).toBe(0);
    expect(tree[0].children[0].id).toBe("b");
    expect(tree[0].children[0].depth).toBe(1);
    expect(tree[0].children[0].children[0].id).toBe("c");
    expect(tree[0].children[0].children[0].depth).toBe(2);
  });

  it("orders siblings by sortOrder ascending", () => {
    const folders = [
      folder({ id: "x", sortOrder: 2 }),
      folder({ id: "y", sortOrder: 0 }),
      folder({ id: "z", sortOrder: 1 }),
    ];
    const tree = buildFolderTree(folders);
    expect(tree.map((n) => n.id)).toEqual(["y", "z", "x"]);
  });

  it("treats a folder with an unknown parent as a root (no dropped subtrees)", () => {
    const folders = [folder({ id: "orphan", parentId: "missing" })];
    const tree = buildFolderTree(folders);
    expect(tree.map((n) => n.id)).toEqual(["orphan"]);
  });

  it("does not loop on a cyclic parent chain", () => {
    const folders = [
      folder({ id: "a", parentId: "b" }),
      folder({ id: "b", parentId: "a" }),
    ];
    // Must terminate and surface every folder somewhere in the forest.
    const tree = buildFolderTree(folders);
    const ids = flattenFolderTree(tree)
      .map((n) => n.id)
      .sort();
    expect(ids).toEqual(["a", "b"]);
  });

  it("preserves every folder exactly once for any acyclic forest (property)", () => {
    const arb = fc
      .array(
        fc.record({
          id: fc.integer({ min: 0, max: 30 }).map((n) => `f${n}`),
          sortOrder: fc.integer({ min: 0, max: 100 }),
        }),
        { maxLength: 30 },
      )
      .map((rows) => {
        // Dedupe ids, then wire each non-first folder to an earlier one to stay acyclic.
        const seen = new Map<string, { id: string; sortOrder: number }>();
        for (const r of rows) if (!seen.has(r.id)) seen.set(r.id, r);
        const list = [...seen.values()];
        return list.map((r, i) =>
          folder({
            id: r.id,
            sortOrder: r.sortOrder,
            parentId: i === 0 ? null : list[Math.floor(i / 2)].id,
          }),
        );
      });

    fc.assert(
      fc.property(arb, (folders) => {
        const flat = flattenFolderTree(buildFolderTree(folders));
        const inputIds = folders.map((f) => f.id).sort();
        const outputIds = flat.map((n) => n.id).sort();
        expect(outputIds).toEqual(inputIds);
      }),
    );
  });
});

describe("collectSubtreeIds / wouldCreateCycle (Req 8.9)", () => {
  const folders = [
    folder({ id: "a" }),
    folder({ id: "b", parentId: "a" }),
    folder({ id: "c", parentId: "b" }),
    folder({ id: "d" }),
  ];

  it("collects a folder and all its descendants", () => {
    expect([...collectSubtreeIds(folders, "a")].sort()).toEqual(["a", "b", "c"]);
  });

  it("flags reparenting onto self or a descendant as a cycle", () => {
    expect(wouldCreateCycle(folders, "a", "a")).toBe(true);
    expect(wouldCreateCycle(folders, "a", "c")).toBe(true);
  });

  it("allows reparenting onto an unrelated folder or to root", () => {
    expect(wouldCreateCycle(folders, "a", "d")).toBe(false);
    expect(wouldCreateCycle(folders, "a", null)).toBe(false);
  });
});

describe("reorderSiblings (Req 8.5)", () => {
  it("moves an item down to the target slot", () => {
    expect(reorderSiblings(["a", "b", "c", "d"], "a", "c")).toEqual([
      "b",
      "c",
      "a",
      "d",
    ]);
  });

  it("moves an item up to the target slot", () => {
    expect(reorderSiblings(["a", "b", "c", "d"], "d", "b")).toEqual([
      "a",
      "d",
      "b",
      "c",
    ]);
  });

  it("returns the order unchanged for a no-op or unknown id", () => {
    expect(reorderSiblings(["a", "b"], "a", "a")).toEqual(["a", "b"]);
    expect(reorderSiblings(["a", "b"], "z", "a")).toEqual(["a", "b"]);
  });

  it("is a permutation of the input for any move (property)", () => {
    const ids = ["a", "b", "c", "d", "e"];
    fc.assert(
      fc.property(
        fc.constantFrom(...ids),
        fc.constantFrom(...ids),
        (dragged, target) => {
          const result = reorderSiblings(ids, dragged, target);
          expect([...result].sort()).toEqual([...ids].sort());
          expect(result).toHaveLength(ids.length);
        },
      ),
    );
  });
});

describe("rootFolderIdsInOrder (Req 8.5)", () => {
  it("returns only root folders, ordered by sortOrder", () => {
    const folders = [
      folder({ id: "r2", sortOrder: 1 }),
      folder({ id: "child", parentId: "r2" }),
      folder({ id: "r1", sortOrder: 0 }),
    ];
    expect(rootFolderIdsInOrder(folders)).toEqual(["r1", "r2"]);
  });
});
