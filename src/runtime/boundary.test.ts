import { describe, expect, it } from "vitest";
// @ts-expect-error - node types are not installed in this frontend project; the
// modules are available at runtime under Vitest's Node environment.
import { readdirSync, readFileSync } from "node:fs";
// @ts-expect-error - see above.
import { join } from "node:path";
// @ts-expect-error - see above.
import { fileURLToPath } from "node:url";

/**
 * Runtime Bridge import boundary (Req 3.1).
 *
 * `src/runtime` is the only module allowed to import `@tauri-apps/api`; every
 * other module reaches the backend through the bridge. `AppearancePanel.test.tsx`
 * pins the rule for one component. This test pins it for the whole `src/` tree,
 * so a new file cannot reintroduce a direct import.
 */

const srcRoot = fileURLToPath(new URL("..", import.meta.url));
const bridgeDir = "runtime";

/** Matches a static or dynamic import of the Tauri API, not a mention in prose. */
const tauriImport =
  /(?:^|\n)\s*import\s[^;]*?from\s*["']@tauri-apps\/api|import\s*\(\s*["']@tauri-apps\/api/;

/** Collects every `.ts`/`.tsx` file under `directory`, as `/`-joined paths. */
function sourceFiles(directory: string, prefix = ""): string[] {
  const collected: string[] = [];
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    const relative = prefix ? `${prefix}/${entry.name}` : entry.name;
    if (entry.isDirectory()) {
      collected.push(...sourceFiles(join(directory, entry.name), relative));
    } else if (/\.tsx?$/.test(entry.name)) {
      collected.push(relative);
    }
  }
  return collected;
}

const files = sourceFiles(srcRoot);

function read(file: string): string {
  return readFileSync(join(srcRoot, file), "utf8");
}

describe("Runtime Bridge import boundary", () => {
  it("walks the frontend sources", () => {
    // Guards the two assertions below against an empty file list.
    expect(files.length).toBeGreaterThan(20);
  });

  it("detects the import inside src/runtime", () => {
    expect(tauriImport.test(read(`${bridgeDir}/index.ts`))).toBe(true);
  });

  it("finds no import outside src/runtime", () => {
    const offenders = files
      .filter((file) => !file.startsWith(`${bridgeDir}/`))
      .filter((file) => tauriImport.test(read(file)));
    expect(offenders).toEqual([]);
  });
});
