import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { CheckIcon, ClipboardCopyIcon } from "lucide-react";
import { promptApi } from "../api";
import type { PromptCopyResult } from "../types";
import {
  buildPromptCopyText,
  defaultVariableValues,
  formatCopiedPrompt,
  type PromptCopySource,
} from "../promptText";

interface CopyPromptButtonProps {
  source: PromptCopySource;
  promptId?: string;
  copyPrompt?: (id: string, values: Record<string, string>) => Promise<PromptCopyResult>;
  /** Prompt title used in the list's accessible name. */
  name?: string;
  locked?: boolean;
  /** List rows use the compact 28px control; the editor header uses 32px. */
  compact?: boolean;
  writeText?: (text: string) => Promise<void>;
}

type CopyStatus = "idle" | "busy" | "copied" | "failed";

const COPIED_MS = 1500;

async function defaultWriteText(text: string): Promise<void> {
  await navigator.clipboard.writeText(text);
}

/**
 * Icon-only clipboard copy for a prompt list row or the editor definition
 * header. Success and failure stay on this control.
 */
export function CopyPromptButton({
  source,
  promptId,
  copyPrompt = (id, values) => promptApi.copyPrompt(id, values),
  name,
  locked = false,
  compact = false,
  writeText = defaultWriteText,
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
    } catch {
      setStatus("failed");
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
      className={`flex shrink-0 items-center justify-center rounded-md transition-[transform,color,background-color] hover:bg-accent hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring active:scale-[0.96] disabled:pointer-events-none disabled:opacity-40 ${
        compact ? "h-7 w-7" : "h-8 w-8"
      } ${
        status === "copied" ? "text-primary" : "text-muted-foreground"
      }`}
    >
      {status === "copied" ? (
        <CheckIcon className="h-3.5 w-3.5" aria-hidden="true" />
      ) : (
        <ClipboardCopyIcon className="h-3.5 w-3.5" aria-hidden="true" />
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
