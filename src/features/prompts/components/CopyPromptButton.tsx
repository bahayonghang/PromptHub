import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { CheckIcon, ClipboardCopyIcon } from "lucide-react";
import { useToastStore } from "../../notifications/toastStore";
import { promptApi } from "../api";
import { usePromptStore } from "../promptStore";
import type { PromptCopyResult } from "../types";
import {
  buildPromptCopyText,
  defaultVariableValues,
  formatCopiedPrompt,
  type PromptCopySource,
} from "../promptText";
import { cn } from "../../../components/ui/cn";

export const PROMPT_COPY_TOAST_GROUP = "prompt-copy";

interface CopyPromptButtonProps {
  source: PromptCopySource;
  promptId?: string;
  copyPrompt?: (id: string, values: Record<string, string>) => Promise<PromptCopyResult>;
  /** Prompt title used in the accessible name and named copy toast. */
  name?: string;
  locked?: boolean;
  writeText?: (text: string) => Promise<void>;
  incrementUsage?: (id: string) => Promise<unknown>;
  className?: string;
}

type CopyStatus = "idle" | "busy" | "copied" | "failed";

const COPIED_MS = 1500;

async function defaultWriteText(text: string): Promise<void> {
  await navigator.clipboard.writeText(text);
}

async function defaultIncrementUsage(id: string): Promise<unknown> {
  return usePromptStore.getState().incrementUsage(id);
}

/** Pushes the copy success or failure toast, replacing any previous copy toast. */
export function pushPromptCopyToast(
  t: (key: string, options?: { title: string }) => string,
  outcome: "success" | "failure",
  name?: string,
): void {
  if (outcome === "success") {
    useToastStore.getState().push({
      message: name
        ? t("promptsView.copyPromptCopiedNamed", { title: name })
        : t("promptsView.copyPromptCopied"),
      tone: "success",
      replaceGroup: PROMPT_COPY_TOAST_GROUP,
    });
    return;
  }
  useToastStore.getState().push({
    message: t("promptsView.copyPromptFailed"),
    tone: "danger",
    replaceGroup: PROMPT_COPY_TOAST_GROUP,
  });
}

/** Records a persisted copy after the clipboard write succeeds. Failures are ignored. */
export async function recordCopiedPromptUsage(
  promptId: string | undefined,
  incrementUsage: (id: string) => Promise<unknown> = defaultIncrementUsage,
): Promise<void> {
  if (!promptId) return;
  try {
    await incrementUsage(promptId);
  } catch {
    // Copy already succeeded; usage can stay stale until the next load.
  }
}

/**
 * Icon-only clipboard copy for a prompt list row or the editor definition
 * header. Success and failure stay on this control and also push a toast.
 */
export function CopyPromptButton({
  source,
  promptId,
  copyPrompt = (id, values) => promptApi.copyPrompt(id, values),
  name,
  locked = false,
  writeText = defaultWriteText,
  incrementUsage = defaultIncrementUsage,
  className = "",
}: CopyPromptButtonProps) {
  const { t } = useTranslation();
  const [status, setStatus] = useState<CopyStatus>("idle");
  const timerRef = useRef<number | null>(null);

  useEffect(() => {
    return () => {
      if (timerRef.current != null) {
        window.clearTimeout(timerRef.current);
      }
    };
  }, []);

  const idleLabel = name
    ? t("promptsView.copyPromptNamed", { title: name })
    : t("promptsView.copyPrompt");
  const label =
    locked
      ? t("promptsView.copyPromptLocked")
      : status === "copied"
        ? t("promptsView.copyPromptCopied")
        : status === "failed"
          ? t("promptsView.copyPromptFailed")
          : idleLabel;

  const copy = async () => {
    if (locked || status === "busy") return;
    if (timerRef.current != null) {
      window.clearTimeout(timerRef.current);
      timerRef.current = null;
    }
    setStatus("busy");
    try {
      const values = defaultVariableValues(source.variables);
      if (promptId) {
        const copied = await copyPrompt(promptId, values);
        await writeText(formatCopiedPrompt(copied));
      } else {
        await writeText(buildPromptCopyText(source));
      }
      setStatus("copied");
      timerRef.current = window.setTimeout(() => {
        setStatus("idle");
        timerRef.current = null;
      }, COPIED_MS);
      pushPromptCopyToast(t, "success", name);
      await recordCopiedPromptUsage(promptId, incrementUsage);
    } catch {
      setStatus("failed");
      pushPromptCopyToast(t, "failure");
    }
  };

  return (
    <button
      type="button"
      title={label}
      aria-label={label}
      disabled={locked || status === "busy"}
      onClick={(event) => {
        event.preventDefault();
        event.stopPropagation();
        void copy();
      }}
      className={cn(
        "flex h-control-lg w-control-lg shrink-0 items-center justify-center rounded-md",
        "transition-[transform,color,background-color,opacity] duration-base ease-spring",
        "hover:bg-accent hover:text-foreground active:scale-[0.96]",
        "disabled:pointer-events-none disabled:opacity-50",
        status === "copied" ? "text-primary" : "text-muted-foreground",
        className,
      )}
    >
      {status === "copied" ? (
        <CheckIcon className="h-5 w-5" aria-hidden="true" />
      ) : (
        <ClipboardCopyIcon className="h-5 w-5" aria-hidden="true" />
      )}
      <span className="sr-only" aria-live="polite">
        {status === "copied" ? t("promptsView.copyPromptCopied") : ""}
      </span>
      <span className="sr-only" aria-live="assertive">
        {status === "failed" ? t("promptsView.copyPromptFailed") : ""}
      </span>
    </button>
  );
}
