import { useState } from "react";
import { useTranslation } from "react-i18next";
import {
  LockIcon,
  LockKeyholeIcon,
  ShieldCheckIcon,
  UnlockIcon,
} from "lucide-react";
import type { SecurityStatus } from "../types";
import { validateNewPassword, validatePasswordLength } from "../validation";

interface SecurityPanelProps {
  status: SecurityStatus | null;
  onSetMasterPassword: (password: string) => Promise<boolean>;
  onChangeMasterPassword: (
    currentPassword: string,
    newPassword: string,
  ) => Promise<boolean>;
  onUnlock: (password: string) => Promise<boolean>;
  onLock: () => void;
}

const inputClass =
  "w-full rounded-md border border-input bg-background px-3 py-2 text-body text-foreground outline-none";
const labelClass = "text-label font-medium text-muted-foreground";

/**
 * Security panel (Req 15). Shows the master-password and lock status (Req 15.1)
 * and offers the appropriate action depending on state:
 *  - no master password set -> set one (8-128 chars, Req 15.2, 15.3);
 *  - master password set + locked -> unlock (Req 15.6);
 *  - master password set + unlocked -> lock (Req 15.8) and change password
 *    (Req 15.4), which reports restart-required at the view level.
 *
 * Password inputs are local component state passed straight to the handlers and
 * are never logged. Inputs clear on a successful action.
 */
export function SecurityPanel({
  status,
  onSetMasterPassword,
  onChangeMasterPassword,
  onUnlock,
  onLock,
}: SecurityPanelProps) {
  const { t } = useTranslation();

  // Set-master-password form (when none is configured).
  const [newPw, setNewPw] = useState("");
  const [confirmPw, setConfirmPw] = useState("");
  // Unlock form (when configured + locked).
  const [unlockPw, setUnlockPw] = useState("");
  // Change-password form (when configured + unlocked).
  const [currentPw, setCurrentPw] = useState("");
  const [changeNewPw, setChangeNewPw] = useState("");
  const [changeConfirmPw, setChangeConfirmPw] = useState("");
  const [showChange, setShowChange] = useState(false);
  // Inline validation error (an i18n key) for the active form.
  const [localError, setLocalError] = useState<string | null>(null);

  const hasMasterPassword = status?.hasMasterPassword ?? false;
  const isLocked = status?.isLocked ?? true;

  const submitSet = async () => {
    const error = validateNewPassword(newPw, confirmPw);
    if (error) {
      setLocalError(error);
      return;
    }
    setLocalError(null);
    const ok = await onSetMasterPassword(newPw);
    if (ok) {
      setNewPw("");
      setConfirmPw("");
    }
  };

  const submitUnlock = async () => {
    if (unlockPw === "") {
      setLocalError("settingsView.security.passwordRequired");
      return;
    }
    setLocalError(null);
    const ok = await onUnlock(unlockPw);
    if (ok) setUnlockPw("");
  };

  const submitChange = async () => {
    if (currentPw === "") {
      setLocalError("settingsView.security.currentPasswordRequired");
      return;
    }
    const error = validateNewPassword(changeNewPw, changeConfirmPw);
    if (error) {
      setLocalError(error);
      return;
    }
    setLocalError(null);
    const ok = await onChangeMasterPassword(currentPw, changeNewPw);
    if (ok) {
      setCurrentPw("");
      setChangeNewPw("");
      setChangeConfirmPw("");
      setShowChange(false);
    }
  };

  return (
    <div className="flex flex-col gap-6">
      {/* Status */}
      <section className="flex items-center gap-3 rounded-md border border-border p-4">
        <span
          className={`flex h-10 w-10 items-center justify-center rounded-full ${
            hasMasterPassword ? "bg-primary/15 text-primary" : "bg-muted text-muted-foreground"
          }`}
        >
          {hasMasterPassword ? (
            <ShieldCheckIcon className="h-5 w-5" aria-hidden="true" />
          ) : (
            <LockKeyholeIcon className="h-5 w-5" aria-hidden="true" />
          )}
        </span>
        <div className="flex min-w-0 flex-1 flex-col">
          <span className="text-body font-medium text-foreground">
            {hasMasterPassword
              ? t("settingsView.security.statusConfigured")
              : t("settingsView.security.statusNotConfigured")}
          </span>
          <span className="text-label text-muted-foreground">
            {hasMasterPassword
              ? isLocked
                ? t("settingsView.security.statusLocked")
                : t("settingsView.security.statusUnlocked")
              : t("settingsView.security.statusNotConfiguredHint")}
          </span>
        </div>
      </section>

      {localError && (
        <p role="alert" className="text-body text-destructive">
          {t(localError)}
        </p>
      )}

      {/* Action area depends on the current state. */}
      {!hasMasterPassword ? (
        <section className="flex flex-col gap-3">
          <h3 className="text-body font-medium text-foreground">
            {t("settingsView.security.setTitle")}
          </h3>
          <p className="text-label text-muted-foreground">
            {t("settingsView.security.setHint")}
          </p>
          <div className="flex flex-col gap-1">
            <label className={labelClass} htmlFor="set-master-pw">
              {t("settingsView.security.newPassword")}
            </label>
            <input
              id="set-master-pw"
              type="password"
              autoComplete="new-password"
              value={newPw}
              onChange={(e) => setNewPw(e.target.value)}
              className={inputClass}
            />
          </div>
          <div className="flex flex-col gap-1">
            <label className={labelClass} htmlFor="set-master-pw-confirm">
              {t("settingsView.security.confirmPassword")}
            </label>
            <input
              id="set-master-pw-confirm"
              type="password"
              autoComplete="new-password"
              value={confirmPw}
              onChange={(e) => setConfirmPw(e.target.value)}
              className={inputClass}
            />
          </div>
          <button
            type="button"
            onClick={() => void submitSet()}
            disabled={validatePasswordLength(newPw) !== null}
            className="flex w-fit items-center gap-2 rounded-md bg-primary px-4 py-2 text-body font-medium text-primary-foreground disabled:opacity-50"
          >
            <ShieldCheckIcon className="h-4 w-4" aria-hidden="true" />
            {t("settingsView.security.setButton")}
          </button>
        </section>
      ) : isLocked ? (
        <section className="flex flex-col gap-3">
          <h3 className="text-body font-medium text-foreground">
            {t("settingsView.security.unlockTitle")}
          </h3>
          <div className="flex flex-col gap-1">
            <label className={labelClass} htmlFor="unlock-pw">
              {t("settingsView.security.password")}
            </label>
            <input
              id="unlock-pw"
              type="password"
              autoComplete="current-password"
              value={unlockPw}
              onChange={(e) => setUnlockPw(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") void submitUnlock();
              }}
              className={inputClass}
            />
          </div>
          <button
            type="button"
            onClick={() => void submitUnlock()}
            className="flex w-fit items-center gap-2 rounded-md bg-primary px-4 py-2 text-body font-medium text-primary-foreground"
          >
            <UnlockIcon className="h-4 w-4" aria-hidden="true" />
            {t("settingsView.security.unlockButton")}
          </button>
        </section>
      ) : (
        <section className="flex flex-col gap-4">
          <button
            type="button"
            onClick={onLock}
            className="flex w-fit items-center gap-2 rounded-md border border-input px-4 py-2 text-body text-foreground hover:bg-accent"
          >
            <LockIcon className="h-4 w-4" aria-hidden="true" />
            {t("settingsView.security.lockButton")}
          </button>

          {!showChange ? (
            <button
              type="button"
              onClick={() => {
                setLocalError(null);
                setShowChange(true);
              }}
              className="w-fit text-body font-medium text-primary hover:underline"
            >
              {t("settingsView.security.changeTitle")}
            </button>
          ) : (
            <div className="flex flex-col gap-3 rounded-md border border-border p-4">
              <h3 className="text-body font-medium text-foreground">
                {t("settingsView.security.changeTitle")}
              </h3>
              <p className="text-label text-muted-foreground">
                {t("settingsView.security.changeRestartHint")}
              </p>
              <div className="flex flex-col gap-1">
                <label className={labelClass} htmlFor="change-current-pw">
                  {t("settingsView.security.currentPassword")}
                </label>
                <input
                  id="change-current-pw"
                  type="password"
                  autoComplete="current-password"
                  value={currentPw}
                  onChange={(e) => setCurrentPw(e.target.value)}
                  className={inputClass}
                />
              </div>
              <div className="flex flex-col gap-1">
                <label className={labelClass} htmlFor="change-new-pw">
                  {t("settingsView.security.newPassword")}
                </label>
                <input
                  id="change-new-pw"
                  type="password"
                  autoComplete="new-password"
                  value={changeNewPw}
                  onChange={(e) => setChangeNewPw(e.target.value)}
                  className={inputClass}
                />
              </div>
              <div className="flex flex-col gap-1">
                <label className={labelClass} htmlFor="change-confirm-pw">
                  {t("settingsView.security.confirmPassword")}
                </label>
                <input
                  id="change-confirm-pw"
                  type="password"
                  autoComplete="new-password"
                  value={changeConfirmPw}
                  onChange={(e) => setChangeConfirmPw(e.target.value)}
                  className={inputClass}
                />
              </div>
              <div className="flex items-center gap-2">
                <button
                  type="button"
                  onClick={() => void submitChange()}
                  className="rounded-md bg-primary px-4 py-2 text-body font-medium text-primary-foreground"
                >
                  {t("settingsView.security.changeButton")}
                </button>
                <button
                  type="button"
                  onClick={() => {
                    setShowChange(false);
                    setLocalError(null);
                    setCurrentPw("");
                    setChangeNewPw("");
                    setChangeConfirmPw("");
                  }}
                  className="rounded-md border border-input px-4 py-2 text-body text-muted-foreground hover:bg-accent hover:text-foreground"
                >
                  {t("common.cancel")}
                </button>
              </div>
            </div>
          )}
        </section>
      )}
    </div>
  );
}
