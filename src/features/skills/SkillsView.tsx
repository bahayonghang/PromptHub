import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  BookOpenIcon,
  EyeIcon,
  HistoryIcon,
  PlusIcon,
  StarIcon,
  Trash2Icon,
} from "lucide-react";
import {
  selectFilteredSkills,
  selectSelectedSkill,
  useSkillStore,
} from "./skillStore";
import { SkillList } from "./components/SkillList";
import { SkillEditor } from "./components/SkillEditor";
import { SkillMdPreview } from "./components/SkillMdPreview";
import { SkillVersionHistory } from "./components/SkillVersionHistory";
import { PlatformIntegration } from "./components/PlatformIntegration";
import { SafetyPanel } from "./components/SafetyPanel";

/** Which optional side panel is open next to the skill editor. */
type SidePanel = "none" | "preview" | "history";

/**
 * The skill-management view (Req 22.3). Lays out a searchable skill list
 * (Req 9.3), a skill editor with SKILL.md preview and version history (Req 9, 10),
 * and platform install/uninstall plus safety scanning panels (Req 12, 13). All
 * data flows through the skill store, which routes every backend call through
 * the Runtime_Bridge (Req 3.1).
 */
export function SkillsView() {
  const { t } = useTranslation();

  const keyword = useSkillStore((s) => s.keyword);
  const filteredSkills = useSkillStore(selectFilteredSkills);
  const selectedSkillId = useSkillStore((s) => s.selectedSkillId);
  const selectedSkill = useSkillStore(selectSelectedSkill);
  const skills = useSkillStore((s) => s.skills);
  const versions = useSkillStore((s) => s.versions);
  const platforms = useSkillStore((s) => s.platforms);
  const detectedPlatforms = useSkillStore((s) => s.detectedPlatforms);
  const installStatus = useSkillStore((s) => s.installStatus);
  const platformUnavailable = useSkillStore((s) => s.platformUnavailable);
  const safetyReport = useSkillStore((s) => s.safetyReport);
  const loading = useSkillStore((s) => s.loading);
  const error = useSkillStore((s) => s.error);

  const load = useSkillStore((s) => s.load);
  const setKeyword = useSkillStore((s) => s.setKeyword);
  const selectSkill = useSkillStore((s) => s.selectSkill);
  const createSkill = useSkillStore((s) => s.createSkill);
  const saveSkill = useSkillStore((s) => s.saveSkill);
  const deleteSkill = useSkillStore((s) => s.deleteSkill);
  const createVersion = useSkillStore((s) => s.createVersion);
  const rollbackVersion = useSkillStore((s) => s.rollbackVersion);
  const deleteVersion = useSkillStore((s) => s.deleteVersion);
  const installToPlatform = useSkillStore((s) => s.installToPlatform);
  const uninstallFromPlatform = useSkillStore((s) => s.uninstallFromPlatform);
  const scanSafety = useSkillStore((s) => s.scanSafety);

  const [creating, setCreating] = useState(false);
  const [sidePanel, setSidePanel] = useState<SidePanel>("none");
  const [scanning, setScanning] = useState(false);

  useEffect(() => {
    void load();
  }, [load]);

  // The set of all known tags across skills, for the editor's tag suggestions.
  const knownTags = useMemo(() => {
    const tags = new Set<string>();
    for (const skill of skills) {
      for (const tag of skill.tags) tags.add(tag);
    }
    return [...tags].sort();
  }, [skills]);

  const startCreate = () => {
    setCreating(true);
    setSidePanel("none");
    void selectSkill(null);
  };

  const handleDeleteSkill = (id: string) => {
    if (window.confirm(t("skillsView.deleteSkillConfirm"))) {
      void deleteSkill(id);
    }
  };

  const handleScanSafety = () => {
    setScanning(true);
    void scanSafety().finally(() => setScanning(false));
  };

  const togglePanel = (panel: Exclude<SidePanel, "none">) =>
    setSidePanel((current) => (current === panel ? "none" : panel));

  const editorActive = creating || selectedSkill != null;

  return (
    <div className="flex h-full w-full">
      {/* Skill list + search */}
      <section className="flex w-80 shrink-0 flex-col border-r border-border">
        <div className="flex flex-col gap-2 border-b border-border p-3">
          <div className="flex items-center justify-between">
            <h2 className="text-sm font-semibold text-foreground">
              {t("common.skills")}
            </h2>
            <button
              type="button"
              onClick={startCreate}
              className="flex items-center gap-1.5 rounded-md bg-primary px-2.5 py-1.5 text-xs font-medium text-primary-foreground"
            >
              <PlusIcon className="h-3.5 w-3.5" aria-hidden="true" />
              {t("skillsView.newSkill")}
            </button>
          </div>
          <input
            type="search"
            value={keyword}
            placeholder={t("skillsView.searchPlaceholder")}
            aria-label={t("skillsView.searchPlaceholder")}
            onChange={(e) => setKeyword(e.target.value)}
            className="w-full rounded-lg border border-input bg-background px-3 py-2 text-sm text-foreground outline-none focus:ring-1 focus:ring-ring"
          />
        </div>
        <div className="min-h-0 flex-1 overflow-y-auto">
          <SkillList
            skills={filteredSkills}
            selectedSkillId={selectedSkillId}
            loading={loading}
            onSelect={(id) => {
              setCreating(false);
              void selectSkill(id);
            }}
          />
        </div>
      </section>

      {/* Editor + panels */}
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
                  ? t("skillsView.editor.create")
                  : selectedSkill?.name || t("skillsView.untitled")}
              </span>
              {!creating && selectedSkill && (
                <>
                  <button
                    type="button"
                    title={
                      selectedSkill.isFavorite
                        ? t("skillsView.unfavorite")
                        : t("skillsView.favorite")
                    }
                    aria-label={
                      selectedSkill.isFavorite
                        ? t("skillsView.unfavorite")
                        : t("skillsView.favorite")
                    }
                    aria-pressed={selectedSkill.isFavorite}
                    onClick={() =>
                      void saveSkill(selectedSkill.id, {
                        isFavorite: !selectedSkill.isFavorite,
                      })
                    }
                    className="rounded p-1.5 text-muted-foreground hover:bg-accent hover:text-foreground"
                  >
                    <StarIcon
                      className={`h-4 w-4 ${
                        selectedSkill.isFavorite ? "fill-current text-primary" : ""
                      }`}
                      aria-hidden="true"
                    />
                  </button>
                  <button
                    type="button"
                    title={t("skillsView.preview.title")}
                    aria-label={t("skillsView.preview.title")}
                    aria-pressed={sidePanel === "preview"}
                    onClick={() => togglePanel("preview")}
                    className={`rounded p-1.5 transition-colors ${
                      sidePanel === "preview"
                        ? "bg-primary/15 text-foreground"
                        : "text-muted-foreground hover:bg-accent hover:text-foreground"
                    }`}
                  >
                    <EyeIcon className="h-4 w-4" aria-hidden="true" />
                  </button>
                  <button
                    type="button"
                    title={t("skillsView.history.title")}
                    aria-label={t("skillsView.history.title")}
                    aria-pressed={sidePanel === "history"}
                    onClick={() => togglePanel("history")}
                    className={`rounded p-1.5 transition-colors ${
                      sidePanel === "history"
                        ? "bg-primary/15 text-foreground"
                        : "text-muted-foreground hover:bg-accent hover:text-foreground"
                    }`}
                  >
                    <HistoryIcon className="h-4 w-4" aria-hidden="true" />
                  </button>
                  <button
                    type="button"
                    title={t("skillsView.deleteSkill")}
                    aria-label={t("skillsView.deleteSkill")}
                    onClick={() => handleDeleteSkill(selectedSkill.id)}
                    className="rounded p-1.5 text-muted-foreground hover:bg-destructive/15 hover:text-destructive"
                  >
                    <Trash2Icon className="h-4 w-4" aria-hidden="true" />
                  </button>
                </>
              )}
            </div>

            <div className="flex min-h-0 flex-1">
              <div className="flex min-w-0 flex-1 flex-col overflow-y-auto">
                <SkillEditor
                  skill={creating ? null : selectedSkill}
                  creating={creating}
                  knownTags={knownTags}
                  onCreate={(input) => {
                    void createSkill(input).then((created) => {
                      if (created) setCreating(false);
                    });
                  }}
                  onSave={(id, patch) => void saveSkill(id, patch)}
                  onCancelCreate={() => setCreating(false)}
                />

                {/* Platform + safety panels apply to a saved skill only. */}
                {!creating && selectedSkill && (
                  <>
                    <PlatformIntegration
                      platforms={platforms}
                      detectedPlatforms={detectedPlatforms}
                      installStatus={installStatus}
                      unavailable={platformUnavailable}
                      onInstall={(platformId) => void installToPlatform(platformId)}
                      onUninstall={(platformId) =>
                        void uninstallFromPlatform(platformId)
                      }
                    />
                    <SafetyPanel
                      report={safetyReport}
                      scanning={scanning}
                      onScan={handleScanSafety}
                    />
                  </>
                )}
              </div>

              {!creating && selectedSkill && sidePanel === "preview" && (
                <aside className="w-80 shrink-0 border-l border-border bg-card/40">
                  <SkillMdPreview skill={selectedSkill} />
                </aside>
              )}
              {!creating && selectedSkill && sidePanel === "history" && (
                <aside className="w-72 shrink-0 border-l border-border bg-card/40">
                  <SkillVersionHistory
                    versions={versions}
                    onCreateVersion={(note) => void createVersion(note)}
                    onRollback={(version) => {
                      if (
                        window.confirm(
                          t("skillsView.history.restoreConfirm", { version }),
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
              <BookOpenIcon className="h-7 w-7" aria-hidden="true" />
            </span>
            <h2 className="text-lg font-semibold text-foreground">
              {t("skillsView.selectSkillTitle")}
            </h2>
            <p className="max-w-sm text-sm text-muted-foreground">
              {t("skillsView.selectSkillHint")}
            </p>
            <button
              type="button"
              onClick={startCreate}
              className="flex items-center gap-1.5 rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground"
            >
              <PlusIcon className="h-4 w-4" aria-hidden="true" />
              {t("skillsView.newSkill")}
            </button>
          </div>
        )}
      </section>
    </div>
  );
}
