import { useState } from "react";
import { useTranslation } from "react-i18next";
import { BellIcon } from "lucide-react";
import { useSystemStore } from "../systemStore";


import { Button, Input, Textarea } from "../../../components/ui";
/** Backend notification length limits (Req 20.7), mirrored for client guarding. */
const MAX_TITLE = 256;
const MAX_BODY = 1000;

/**
 * Notification configuration (Req 20.7, 20.13). Lets the user compose a title
 * (≤256 chars) and body (≤1000 chars) and dispatch a system notification through
 * the system store, which calls the Window_Manager via the Runtime_Bridge
 * (Req 3.1). A denied OS permission surfaces as a structured error through the
 * store rather than displaying a notification (Req 20.13). All text resolves
 * through i18n (Req 21.3) and icons are from Lucide (Req 22.4).
 */
export function NotificationsPanel() {
  const { t } = useTranslation();
  const showNotification = useSystemStore((s) => s.showNotification);

  const [title, setTitle] = useState("");
  const [body, setBody] = useState("");
  const [sent, setSent] = useState(false);

  const labelClass = "text-body font-medium text-foreground";
  const hintClass = "text-label text-muted-foreground";
  const canSend = title.trim() !== "" || body.trim() !== "";

  const send = async () => {
    setSent(false);
    const ok = await showNotification(title, body);
    if (ok) setSent(true);
  };

  return (
    <section className="flex flex-col gap-3">
      <div className="flex flex-col gap-0.5">
        <h3 className={labelClass}>{t("systemView.notifications.title")}</h3>
        <p className={hintClass}>{t("systemView.notifications.hint")}</p>
      </div>

      <label className="flex flex-col gap-1">
        <span className={hintClass}>{t("systemView.notifications.titleLabel")}</span>
        <Input
          type="text"
          value={title}
          maxLength={MAX_TITLE}
          size="lg"
          onChange={(e) => {
            setTitle(e.target.value);
            setSent(false);
          }}
        />
      </label>

      <label className="flex flex-col gap-1">
        <span className={hintClass}>{t("systemView.notifications.bodyLabel")}</span>
        <Textarea
          value={body}
          maxLength={MAX_BODY}
          rows={3}
          size="lg"
          resizable
          onChange={(e) => {
            setBody(e.target.value);
            setSent(false);
          }}
        />
      </label>

      <div className="flex items-center gap-3">
        <Button
          size="lg"
          onClick={() => void send()}
          disabled={!canSend}
        >
          <BellIcon className="h-4 w-4" aria-hidden="true" />
          {t("systemView.notifications.send")}
        </Button>
        {sent && (
          <span className={hintClass} role="status">
            {t("systemView.notifications.sent")}
          </span>
        )}
      </div>
    </section>
  );
}
