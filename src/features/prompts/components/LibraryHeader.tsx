import { useState } from "react";
import { useTranslation } from "react-i18next";
import { DownloadIcon, PlusIcon, UploadIcon } from "lucide-react";
import { resolveLibraryScope, usePromptStore } from "../promptStore";
import { useSystemStore } from "../../system/systemStore";
import type { BundlePreview, ImportConflictPolicy } from "../types";

const iconButtonClass =
  "flex h-8 w-8 shrink-0 items-center justify-center rounded-md text-muted-foreground hover:bg-accent hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-40";

interface LibraryHeaderProps {
  onCreate: () => void;
  onTransferMessage: (message: string | null) => void;
}

export function libraryScopeTitle(
  t: (key: string, options?: Record<string, unknown>) => string,
  state: Parameters<typeof resolveLibraryScope>[0],
): string {
  const scope = resolveLibraryScope(state);
  if (scope.kind === "view") return t(`promptsView.library.${scope.view}`);
  if (scope.kind === "folder") return scope.folder.name;
  if (scope.kind === "tag") return scope.tag;
  return t("promptsView.library.all");
}

export function LibraryHeader({ onCreate, onTransferMessage }: LibraryHeaderProps) {
  const { t } = useTranslation();
  const folders = usePromptStore((state) => state.folders);
  const filters = usePromptStore((state) => state.filters);
  const activeView = usePromptStore((state) => state.activeView);
  const total = usePromptStore((state) => state.total);
  const exportBundle = usePromptStore((state) => state.exportBundle);
  const previewBundle = usePromptStore((state) => state.previewBundle);
  const importBundle = usePromptStore((state) => state.importBundle);
  const dataPath = useSystemStore((state) => state.runtimePaths?.data ?? null);

  const [showImport, setShowImport] = useState(false);
  const [bundlePath, setBundlePath] = useState("");
  const [conflictPolicy, setConflictPolicy] = useState<ImportConflictPolicy>("skip");
  const [bundlePreview, setBundlePreview] = useState<BundlePreview | null>(null);

  const title = libraryScopeTitle(t, { activeView, filters, folders });
  const subtitle =
    dataPath == null
      ? t("promptsView.chrome.subtitleCount", { count: total })
      : t("promptsView.chrome.subtitleWithPath", { count: total, path: dataPath });

  return (
    <div className="flex flex-col gap-2 border-b border-border p-3">
      <div className="flex flex-wrap items-center gap-2">
        <div className="min-w-0 flex-1">
          <h2 className="truncate text-sm font-semibold text-foreground">{title}</h2>
          <p className="truncate font-mono text-xs text-muted-foreground-subtle">{subtitle}</p>
        </div>
        <div className="flex shrink-0 items-center gap-1">
          <button
            type="button"
            title={t("promptsView.bundle.export")}
            aria-label={t("promptsView.bundle.export")}
            onClick={() =>
              void exportBundle().then((result) => {
                if (result) {
                  onTransferMessage(
                    t("promptsView.bundle.exported", { path: result.filePath }),
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
            onClick={onCreate}
            className="flex h-8 items-center gap-1.5 rounded-md bg-primary px-2.5 text-xs font-medium text-primary-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
          >
            <PlusIcon className="h-3.5 w-3.5" aria-hidden="true" />
            {t("promptsView.newPrompt")}
          </button>
        </div>
      </div>
      {showImport && (
        <div className="flex flex-col gap-2 rounded-md border border-border bg-muted/30 p-3">
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
              onClick={() => void previewBundle(bundlePath.trim()).then(setBundlePreview)}
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
                  void importBundle(bundlePath.trim(), conflictPolicy).then((result) => {
                    if (result) {
                      onTransferMessage(
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
                  })
                }
                className="mt-2 rounded bg-primary px-3 py-1.5 text-xs font-medium text-primary-foreground disabled:cursor-not-allowed disabled:opacity-50"
              >
                {t("promptsView.bundle.confirmImport")}
              </button>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
