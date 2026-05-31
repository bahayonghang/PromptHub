import { useState } from "react";
import { useTranslation } from "react-i18next";
import {
  CheckCircle2Icon,
  CloudIcon,
  DatabaseIcon,
  DownloadIcon,
  Trash2Icon,
  XCircleIcon,
} from "lucide-react";
import type {
  BackupEntry,
  ConnectionTestResult,
  ExportScope,
  S3Config,
  Settings,
  WebDavConfig,
} from "../types";
import { validateS3Config, validateWebDavConfig } from "../validation";

interface SyncPanelProps {
  settings: Settings | null;
  backups: BackupEntry[];
  onTestWebdav: (config: WebDavConfig) => Promise<ConnectionTestResult | null>;
  onTestS3: (config: S3Config) => Promise<ConnectionTestResult | null>;
  onExport: (scope: ExportScope) => Promise<{ filePath?: string | null } | null>;
  onCreateBackup: () => void;
  onRestoreBackup: (id: string) => void;
  onDeleteBackup: (id: string) => void;
}

const inputClass =
  "w-full rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground outline-none focus:ring-1 focus:ring-ring";
const labelClass = "text-xs font-medium text-muted-foreground";

/** The selectable export categories with their i18n label keys (Req 17.5). */
const EXPORT_CATEGORIES: ReadonlyArray<{ key: keyof ExportScope; labelKey: string }> = [
  { key: "data", labelKey: "settingsView.sync.export.data" },
  { key: "media", labelKey: "settingsView.sync.export.media" },
  { key: "skill", labelKey: "settingsView.sync.export.skill" },
  { key: "rule", labelKey: "settingsView.sync.export.rule" },
];

/** Renders a connection-test result line with a pass/fail icon (Req 17.1, 17.3). */
function TestResultLine({ result }: { result: ConnectionTestResult }) {
  return (
    <p
      className={`flex items-center gap-1.5 text-xs ${
        result.success ? "text-primary" : "text-destructive"
      }`}
    >
      {result.success ? (
        <CheckCircle2Icon className="h-3.5 w-3.5" aria-hidden="true" />
      ) : (
        <XCircleIcon className="h-3.5 w-3.5" aria-hidden="true" />
      )}
      {result.message}
    </p>
  );
}

/**
 * Sync & backup panel (Req 17). Provides WebDAV and S3 connection testing with an
 * explicit pass/fail result (Req 17.1, 17.3; malformed config is caught client-
 * side first per Req 17.13), a selective ZIP export of the chosen scope (Req
 * 17.5), and an upgrade-backup list with create/restore/delete (Req 17.6-17.8).
 * Restore reports restart-required, surfaced at the view level (Req 17.7).
 */
export function SyncPanel({
  settings,
  backups,
  onTestWebdav,
  onTestS3,
  onExport,
  onCreateBackup,
  onRestoreBackup,
  onDeleteBackup,
}: SyncPanelProps) {
  const { t } = useTranslation();

  const sync = settings?.sync ?? null;

  // WebDAV form, seeded from stored sync settings when the provider matches.
  const [webdav, setWebdav] = useState<WebDavConfig>({
    url: sync?.provider === "webdav" ? sync?.endpoint ?? "" : "",
    username: sync?.provider === "webdav" ? sync?.username ?? "" : "",
    password: "",
  });
  const [webdavResult, setWebdavResult] = useState<ConnectionTestResult | null>(null);
  const [webdavError, setWebdavError] = useState<string | null>(null);
  const [webdavTesting, setWebdavTesting] = useState(false);

  // S3 form.
  const [s3, setS3] = useState<S3Config>({
    endpoint: sync?.provider === "s3" ? sync?.endpoint ?? "" : "",
    region: "",
    bucket: "",
    accessKeyId: sync?.provider === "s3" ? sync?.username ?? "" : "",
    secretAccessKey: "",
  });
  const [s3Result, setS3Result] = useState<ConnectionTestResult | null>(null);
  const [s3Error, setS3Error] = useState<string | null>(null);
  const [s3Testing, setS3Testing] = useState(false);

  // Export scope (all categories selected by default).
  const [scope, setScope] = useState<ExportScope>({
    data: true,
    media: true,
    skill: true,
    rule: true,
  });
  const [exportPath, setExportPath] = useState<string | null>(null);
  const [exporting, setExporting] = useState(false);

  const testWebdav = async () => {
    const error = validateWebDavConfig(webdav);
    if (error) {
      setWebdavError(error);
      setWebdavResult(null);
      return;
    }
    setWebdavError(null);
    setWebdavTesting(true);
    const result = await onTestWebdav(webdav);
    setWebdavTesting(false);
    setWebdavResult(result);
  };

  const testS3 = async () => {
    const error = validateS3Config(s3);
    if (error) {
      setS3Error(error);
      setS3Result(null);
      return;
    }
    setS3Error(null);
    setS3Testing(true);
    const result = await onTestS3(s3);
    setS3Testing(false);
    setS3Result(result);
  };

  const runExport = async () => {
    setExporting(true);
    setExportPath(null);
    const result = await onExport(scope);
    setExporting(false);
    if (result?.filePath) setExportPath(result.filePath);
  };

  const anyScopeSelected = Object.values(scope).some(Boolean);

  return (
    <div className="flex flex-col gap-8">
      {/* WebDAV */}
      <section className="flex flex-col gap-3">
        <h3 className="flex items-center gap-2 text-sm font-medium text-foreground">
          <CloudIcon className="h-4 w-4 text-muted-foreground" aria-hidden="true" />
          {t("settingsView.sync.webdav.title")}
        </h3>
        <div className="flex flex-col gap-2">
          <div className="flex flex-col gap-1">
            <label className={labelClass} htmlFor="webdav-url">
              {t("settingsView.sync.webdav.url")}
            </label>
            <input
              id="webdav-url"
              value={webdav.url}
              placeholder="https://dav.example.com/remote.php/dav"
              onChange={(e) => setWebdav((c) => ({ ...c, url: e.target.value }))}
              className={inputClass}
            />
          </div>
          <div className="grid grid-cols-2 gap-2">
            <div className="flex flex-col gap-1">
              <label className={labelClass} htmlFor="webdav-username">
                {t("settingsView.sync.webdav.username")}
              </label>
              <input
                id="webdav-username"
                value={webdav.username}
                autoComplete="username"
                onChange={(e) => setWebdav((c) => ({ ...c, username: e.target.value }))}
                className={inputClass}
              />
            </div>
            <div className="flex flex-col gap-1">
              <label className={labelClass} htmlFor="webdav-password">
                {t("settingsView.sync.webdav.password")}
              </label>
              <input
                id="webdav-password"
                type="password"
                autoComplete="current-password"
                value={webdav.password}
                onChange={(e) => setWebdav((c) => ({ ...c, password: e.target.value }))}
                className={inputClass}
              />
            </div>
          </div>
          <div className="flex items-center gap-3">
            <button
              type="button"
              onClick={() => void testWebdav()}
              disabled={webdavTesting}
              className="w-fit rounded-md border border-input px-4 py-2 text-sm text-foreground hover:bg-accent disabled:opacity-50"
            >
              {webdavTesting
                ? t("settingsView.sync.testing")
                : t("settingsView.sync.testConnection")}
            </button>
            {webdavError && <p className="text-xs text-destructive">{t(webdavError)}</p>}
            {webdavResult && <TestResultLine result={webdavResult} />}
          </div>
        </div>
      </section>

      {/* S3 */}
      <section className="flex flex-col gap-3">
        <h3 className="flex items-center gap-2 text-sm font-medium text-foreground">
          <DatabaseIcon className="h-4 w-4 text-muted-foreground" aria-hidden="true" />
          {t("settingsView.sync.s3.title")}
        </h3>
        <div className="flex flex-col gap-2">
          <div className="grid grid-cols-2 gap-2">
            <div className="flex flex-col gap-1">
              <label className={labelClass} htmlFor="s3-endpoint">
                {t("settingsView.sync.s3.endpoint")}
              </label>
              <input
                id="s3-endpoint"
                value={s3.endpoint}
                placeholder="https://s3.amazonaws.com"
                onChange={(e) => setS3((c) => ({ ...c, endpoint: e.target.value }))}
                className={inputClass}
              />
            </div>
            <div className="flex flex-col gap-1">
              <label className={labelClass} htmlFor="s3-region">
                {t("settingsView.sync.s3.region")}
              </label>
              <input
                id="s3-region"
                value={s3.region}
                placeholder="us-east-1"
                onChange={(e) => setS3((c) => ({ ...c, region: e.target.value }))}
                className={inputClass}
              />
            </div>
          </div>
          <div className="flex flex-col gap-1">
            <label className={labelClass} htmlFor="s3-bucket">
              {t("settingsView.sync.s3.bucket")}
            </label>
            <input
              id="s3-bucket"
              value={s3.bucket}
              onChange={(e) => setS3((c) => ({ ...c, bucket: e.target.value }))}
              className={inputClass}
            />
          </div>
          <div className="grid grid-cols-2 gap-2">
            <div className="flex flex-col gap-1">
              <label className={labelClass} htmlFor="s3-access-key">
                {t("settingsView.sync.s3.accessKey")}
              </label>
              <input
                id="s3-access-key"
                value={s3.accessKeyId}
                autoComplete="off"
                onChange={(e) => setS3((c) => ({ ...c, accessKeyId: e.target.value }))}
                className={inputClass}
              />
            </div>
            <div className="flex flex-col gap-1">
              <label className={labelClass} htmlFor="s3-secret-key">
                {t("settingsView.sync.s3.secretKey")}
              </label>
              <input
                id="s3-secret-key"
                type="password"
                autoComplete="off"
                value={s3.secretAccessKey}
                onChange={(e) => setS3((c) => ({ ...c, secretAccessKey: e.target.value }))}
                className={inputClass}
              />
            </div>
          </div>
          <div className="flex items-center gap-3">
            <button
              type="button"
              onClick={() => void testS3()}
              disabled={s3Testing}
              className="w-fit rounded-md border border-input px-4 py-2 text-sm text-foreground hover:bg-accent disabled:opacity-50"
            >
              {s3Testing ? t("settingsView.sync.testing") : t("settingsView.sync.testConnection")}
            </button>
            {s3Error && <p className="text-xs text-destructive">{t(s3Error)}</p>}
            {s3Result && <TestResultLine result={s3Result} />}
          </div>
        </div>
      </section>

      {/* Export */}
      <section className="flex flex-col gap-3">
        <div className="flex flex-col gap-0.5">
          <h3 className="text-sm font-medium text-foreground">
            {t("settingsView.sync.export.title")}
          </h3>
          <p className="text-xs text-muted-foreground">
            {t("settingsView.sync.export.hint")}
          </p>
        </div>
        <div className="flex flex-wrap gap-3">
          {EXPORT_CATEGORIES.map(({ key, labelKey }) => (
            <label key={key} className="flex items-center gap-2 text-sm text-foreground">
              <input
                type="checkbox"
                checked={scope[key]}
                onChange={(e) => setScope((s) => ({ ...s, [key]: e.target.checked }))}
                className="h-4 w-4 rounded border-input"
              />
              {t(labelKey)}
            </label>
          ))}
        </div>
        <div className="flex items-center gap-3">
          <button
            type="button"
            onClick={() => void runExport()}
            disabled={exporting || !anyScopeSelected}
            className="flex w-fit items-center gap-1.5 rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground disabled:opacity-50"
          >
            <DownloadIcon className="h-4 w-4" aria-hidden="true" />
            {exporting ? t("settingsView.sync.export.exporting") : t("settingsView.sync.export.button")}
          </button>
          {exportPath && (
            <p className="min-w-0 break-all text-xs text-muted-foreground">
              {t("settingsView.sync.export.done")} {exportPath}
            </p>
          )}
        </div>
      </section>

      {/* Backups */}
      <section className="flex flex-col gap-3">
        <div className="flex items-center justify-between gap-2">
          <div className="flex flex-col gap-0.5">
            <h3 className="text-sm font-medium text-foreground">
              {t("settingsView.sync.backup.title")}
            </h3>
            <p className="text-xs text-muted-foreground">
              {t("settingsView.sync.backup.hint")}
            </p>
          </div>
          <button
            type="button"
            onClick={onCreateBackup}
            className="shrink-0 rounded-md border border-input px-3 py-2 text-sm text-foreground hover:bg-accent"
          >
            {t("settingsView.sync.backup.create")}
          </button>
        </div>

        {backups.length === 0 ? (
          <p className="text-xs text-muted-foreground">
            {t("settingsView.sync.backup.empty")}
          </p>
        ) : (
          <ul className="flex flex-col gap-1.5">
            {backups.map((backup) => (
              <li
                key={backup.id}
                className="flex items-center gap-2 rounded-md border border-border px-3 py-2"
              >
                <span className="flex min-w-0 flex-1 flex-col">
                  <span className="truncate text-sm text-foreground">{backup.id}</span>
                  <span className="text-[11px] text-muted-foreground">{backup.createdAt}</span>
                </span>
                <button
                  type="button"
                  onClick={() => onRestoreBackup(backup.id)}
                  className="shrink-0 rounded-md bg-primary px-2.5 py-1.5 text-xs font-medium text-primary-foreground"
                >
                  {t("settingsView.sync.backup.restore")}
                </button>
                <button
                  type="button"
                  aria-label={t("settingsView.sync.backup.delete")}
                  title={t("settingsView.sync.backup.delete")}
                  onClick={() => onDeleteBackup(backup.id)}
                  className="shrink-0 rounded-md p-1.5 text-muted-foreground hover:bg-destructive/15 hover:text-destructive"
                >
                  <Trash2Icon className="h-4 w-4" aria-hidden="true" />
                </button>
              </li>
            ))}
          </ul>
        )}
      </section>
    </div>
  );
}
