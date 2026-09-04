import { useTranslation } from "react-i18next";
import { EmptyHint } from "../../../components/ui";
import type { RenderedPrompt } from "../types";

/**
 * Right column of the Playground tab: the resolved messages on top, the live
 * model output underneath.
 */
export function OutputPane({
  rendered,
  streamedOutput,
  streaming,
}: {
  rendered: RenderedPrompt | null;
  streamedOutput: string;
  streaming: boolean;
}) {
  const { t } = useTranslation();

  return (
    <section className="grid min-h-0 grid-rows-2">
      <div className="min-w-0 overflow-y-auto border-b border-border p-4">
        <h3 className="mb-2 text-label font-semibold text-foreground">
          {t("evaluation.renderedInput")}
        </h3>
        {rendered?.messages.length ? (
          <div className="space-y-2">
            {rendered.messages.map((message, index) => (
              <div key={index} className="grid grid-cols-[4.5rem_minmax(0,1fr)] gap-2 text-label">
                <span className="text-meta uppercase tracking-wide text-muted-foreground">
                  {message.role}
                </span>
                <pre className="min-w-0 whitespace-pre-wrap break-words font-sans text-foreground">
                  {message.content}
                </pre>
              </div>
            ))}
          </div>
        ) : (
          <EmptyHint>{t("evaluation.previewEmpty")}</EmptyHint>
        )}
      </div>

      <div className="min-w-0 overflow-y-auto p-4" aria-live="polite" aria-busy={streaming}>
        <h3 className="mb-2 flex items-center gap-2 text-label font-semibold text-foreground">
          {t("evaluation.output")}
          {streaming && (
            <span className="h-1.5 w-1.5 animate-pulse rounded-full bg-primary" aria-hidden="true" />
          )}
        </h3>
        {streamedOutput ? (
          <pre className="whitespace-pre-wrap break-words font-sans text-body text-foreground">
            {streamedOutput}
          </pre>
        ) : (
          <EmptyHint>{t("evaluation.outputEmpty")}</EmptyHint>
        )}
      </div>
    </section>
  );
}
