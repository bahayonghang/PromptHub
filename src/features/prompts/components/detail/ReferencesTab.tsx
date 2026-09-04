import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { AlertCircleIcon, LinkIcon } from "lucide-react";
import { promptApi } from "../../api";
import type {
  IncomingReference,
  OutgoingReference,
  Prompt,
  PromptListItem,
  ReferenceList,
} from "../../types";

export interface ReferencesTabProps {
  prompt: Prompt | null;
  prompts: PromptListItem[];
  onInsert: (token: string) => void;
}

function reasonLabel(
  t: (key: string) => string,
  resolution: string,
): string | null {
  if (resolution === "missing") return t("promptsView.detail.unresolvedMissing");
  if (resolution === "ambiguous")
    return t("promptsView.detail.unresolvedAmbiguous");
  if (resolution === "locked") return t("promptsView.detail.unresolvedLocked");
  return null;
}

function OutgoingRow({
  item,
  t,
}: {
  item: OutgoingReference;
  t: (key: string) => string;
}) {
  const reason = reasonLabel(t, item.resolution);
  const title = item.targetTitle ?? item.tokenTitle;
  return (
    <li className="flex items-start gap-2 rounded-md border border-border px-3 py-2 text-sm">
      {reason ? (
        <AlertCircleIcon
          className="mt-0.5 h-4 w-4 shrink-0 text-muted-foreground"
          aria-hidden="true"
        />
      ) : (
        <LinkIcon
          className="mt-0.5 h-4 w-4 shrink-0 text-muted-foreground"
          aria-hidden="true"
        />
      )}
      <div className="min-w-0">
        <div className="truncate text-foreground">{title}</div>
        <div className="text-xs text-muted-foreground">@@{item.tokenTitle}@@</div>
        {reason && <div className="text-xs text-muted-foreground">{reason}</div>}
      </div>
    </li>
  );
}

function IncomingRow({ item }: { item: IncomingReference }) {
  return (
    <li className="flex items-start gap-2 rounded-md border border-border px-3 py-2 text-sm">
      <LinkIcon
        className="mt-0.5 h-4 w-4 shrink-0 text-muted-foreground"
        aria-hidden="true"
      />
      <div className="min-w-0">
        <div className="truncate text-foreground">{item.sourceTitle}</div>
        <div className="text-xs text-muted-foreground">@@{item.tokenTitle}@@</div>
      </div>
    </li>
  );
}

export function ReferencesTab({
  prompt,
  prompts,
  onInsert,
}: ReferencesTabProps) {
  const { t } = useTranslation();
  const [listed, setListed] = useState<ReferenceList | null>(null);
  const [pickerId, setPickerId] = useState("");

  useEffect(() => {
    if (!prompt) {
      setListed(null);
      return;
    }
    let cancelled = false;
    void promptApi.listReferences(prompt.id).then((result) => {
      if (!cancelled) setListed(result);
    });
    return () => {
      cancelled = true;
    };
  }, [prompt]);

  const others = prompts.filter((item) => item.id !== prompt?.id);

  return (
    <div className="flex h-full flex-col gap-6 overflow-y-auto p-4">
      <section aria-labelledby="prompt-refs-outgoing">
        <h3
          id="prompt-refs-outgoing"
          className="text-sm font-semibold text-foreground"
        >
          {t("promptsView.detail.outgoing")}
        </h3>
        {listed && listed.outgoing.length > 0 ? (
          <ul className="mt-2 flex flex-col gap-2">
            {listed.outgoing.map((item) => (
              <OutgoingRow
                key={`${item.tokenTitle}-${item.targetPromptId ?? "none"}`}
                item={item}
                t={t}
              />
            ))}
          </ul>
        ) : (
          <p className="mt-2 text-sm text-muted-foreground">
            {t("promptsView.detail.noOutgoing")}
          </p>
        )}
      </section>

      <section aria-labelledby="prompt-refs-incoming">
        <h3
          id="prompt-refs-incoming"
          className="text-sm font-semibold text-foreground"
        >
          {t("promptsView.detail.incoming")}
        </h3>
        {listed && listed.incoming.length > 0 ? (
          <ul className="mt-2 flex flex-col gap-2">
            {listed.incoming.map((item) => (
              <IncomingRow
                key={`${item.sourcePromptId}-${item.tokenTitle}`}
                item={item}
              />
            ))}
          </ul>
        ) : (
          <p className="mt-2 text-sm text-muted-foreground">
            {t("promptsView.detail.noIncoming")}
          </p>
        )}
      </section>

      <section aria-labelledby="prompt-refs-picker">
        <h3
          id="prompt-refs-picker"
          className="text-sm font-semibold text-foreground"
        >
          {t("promptsView.detail.picker")}
        </h3>
        <div className="mt-2 flex flex-wrap gap-2">
          <select
            aria-label={t("promptsView.detail.pickerPlaceholder")}
            value={pickerId}
            onChange={(event) => setPickerId(event.target.value)}
            className="min-w-0 flex-1 rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground"
          >
            <option value="">
              {t("promptsView.detail.pickerPlaceholder")}
            </option>
            {others.map((item) => (
              <option key={item.id} value={item.id}>
                {item.title}
              </option>
            ))}
          </select>
          <button
            type="button"
            disabled={!pickerId}
            onClick={() => {
              const target = others.find((item) => item.id === pickerId);
              if (!target) return;
              onInsert(`@@${target.title}@@`);
              setPickerId("");
            }}
            className="rounded-md border border-input px-3 py-2 text-sm text-foreground hover:bg-accent disabled:opacity-50"
          >
            {t("promptsView.detail.insertReference")}
          </button>
        </div>
      </section>
    </div>
  );
}
