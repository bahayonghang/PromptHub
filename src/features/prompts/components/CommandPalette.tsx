import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { EmptyHint } from "../../../components/ui";
import { Modal } from "../../../components/ui/Modal";
import { promptApi } from "../api";
import { usePromptStore } from "../promptStore";
import { usePaletteStore } from "../paletteStore";
import { useSettingsStore } from "../../settings/settingsStore";
import type { PromptListItem } from "../types";
import { formatBinding, SHORTCUT_BINDINGS } from "../../../shortcuts/bindings";
import { platformModifier } from "../../../shortcuts/platform";

const SEARCH_DEBOUNCE_MS = 150;
const RESULT_LIMIT = 5;

type PaletteRow =
  | { kind: "prompt"; id: string; prompt: PromptListItem }
  | { kind: "action"; id: string; label: string; hint?: string };

export function CommandPalette() {
  const { t } = useTranslation();
  const open = usePaletteStore((state) => state.open);
  const setOpen = usePaletteStore((state) => state.setOpen);
  const [query, setQuery] = useState("");
  const [prompts, setPrompts] = useState<PromptListItem[]>([]);
  const [activeIndex, setActiveIndex] = useState(0);
  const sequence = useRef(0);
  const modifier = platformModifier();

  useEffect(() => {
    if (!open) {
      setQuery("");
      setPrompts([]);
      setActiveIndex(0);
      return;
    }
    const handle = window.setTimeout(() => {
      const current = ++sequence.current;
      void promptApi
        .searchPrompts({ keyword: query.trim() || undefined, limit: RESULT_LIMIT })
        .then((page) => {
          if (current === sequence.current) setPrompts(page.items);
        })
        .catch(() => {
          if (current !== sequence.current) return;
        });
    }, SEARCH_DEBOUNCE_MS);
    return () => window.clearTimeout(handle);
  }, [open, query]);

  const hint = (id: (typeof SHORTCUT_BINDINGS)[number]["id"]) => {
    const binding = SHORTCUT_BINDINGS.find((item) => item.id === id);
    return binding ? formatBinding(binding, modifier.symbol) : undefined;
  };

  const actions: PaletteRow[] = useMemo(
    () => [
      {
        kind: "action",
        id: "new",
        label: t("promptsView.palette.newPrompt"),
        hint: hint("newPrompt"),
      },
      {
        kind: "action",
        id: "favorites",
        label: t("promptsView.palette.favorites"),
      },
      {
        kind: "action",
        id: "viewMode",
        label: t("promptsView.palette.toggleView"),
      },
      {
        kind: "action",
        id: "theme",
        label: t("promptsView.palette.toggleTheme"),
        hint: hint("toggleTheme"),
      },
    ],
    [t, modifier.symbol],
  );

  const rows: PaletteRow[] = [
    ...prompts.map((prompt) => ({
      kind: "prompt" as const,
      id: prompt.id,
      prompt,
    })),
    ...actions,
  ];

  useEffect(() => {
    setActiveIndex(0);
  }, [query, prompts]);

  const activate = async (row: PaletteRow) => {
    if (row.kind === "prompt") {
      const ok = await usePromptStore.getState().requestSelectPrompt(row.prompt.id);
      if (ok) setOpen(false);
      return;
    }
    if (row.id === "new") {
      usePromptStore.getState().createPromptAction?.();
      setOpen(false);
      return;
    }
    if (row.id === "favorites") {
      void usePromptStore.getState().selectView("favorites");
      setOpen(false);
      return;
    }
    if (row.id === "viewMode") {
      const current = usePromptStore.getState().viewMode;
      usePromptStore.getState().setViewMode(current === "grid" ? "list" : "grid");
      setOpen(false);
      return;
    }
    if (row.id === "theme") {
      const theme = useSettingsStore.getState().settings?.theme ?? "dark";
      const isDark =
        theme === "dark" ||
        (theme === "system" &&
          document.documentElement.classList.contains("dark"));
      void useSettingsStore
        .getState()
        .setPreference("theme", isDark ? "light" : "dark");
      setOpen(false);
    }
  };

  const activeId = rows[activeIndex]?.id ?? "";

  return (
    <Modal
      open={open}
      title={t("promptsView.library.commandPalette")}
      onClose={() => setOpen(false)}
      className="max-h-[min(28rem,80vh)] w-[min(32rem,100%)]"
    >
      <div className="flex flex-col">
        <input
          value={query}
          data-command-palette-input=""
          onChange={(event) => setQuery(event.target.value)}
          placeholder={t("promptsView.palette.placeholder")}
          aria-label={t("promptsView.palette.placeholder")}
          role="combobox"
          aria-expanded
          aria-controls="command-palette-list"
          aria-activedescendant={activeId ? `palette-row-${activeId}` : undefined}
          onKeyDown={(event) => {
            if (event.key === "ArrowDown") {
              event.preventDefault();
              setActiveIndex((index) => Math.min(index + 1, rows.length - 1));
            }
            if (event.key === "ArrowUp") {
              event.preventDefault();
              setActiveIndex((index) => Math.max(index - 1, 0));
            }
            if (event.key === "Enter") {
              event.preventDefault();
              const row = rows[activeIndex];
              if (row) void activate(row);
            }
          }}
          className="border-b border-border bg-transparent px-4 py-3 text-body text-foreground outline-none"
        />
        <div
          id="command-palette-list"
          role="listbox"
          aria-label={t("promptsView.library.commandPalette")}
          className="max-h-80 overflow-y-auto p-2"
        >
          {prompts.length > 0 && (
            <p className="px-2 py-1 text-meta font-medium uppercase tracking-wide text-muted-foreground">
              {t("promptsView.palette.groupPrompts")}
            </p>
          )}
          {rows
            .filter((row) => row.kind === "prompt")
            .map((row) => (
              <PaletteOption
                key={row.id}
                id={`palette-row-${row.id}`}
                label={row.prompt.title}
                hint={
                  row.prompt.usageCount > 0
                    ? String(row.prompt.usageCount)
                    : undefined
                }
                active={row.id === activeId}
                onClick={() => void activate(row)}
              />
            ))}
          <p className="px-2 py-1 text-meta font-medium uppercase tracking-wide text-muted-foreground">
            {t("promptsView.palette.groupActions")}
          </p>
          {rows
            .filter((row) => row.kind === "action")
            .map((row) => (
              <PaletteOption
                key={row.id}
                id={`palette-row-${row.id}`}
                label={row.label}
                hint={row.hint}
                active={row.id === activeId}
                onClick={() => void activate(row)}
              />
            ))}
          {prompts.length === 0 && query.trim() !== "" && (
            <EmptyHint className="px-2 py-3">{t("promptsView.palette.empty")}</EmptyHint>
          )}
        </div>
      </div>
    </Modal>
  );
}

function PaletteOption({
  id,
  label,
  hint,
  active,
  onClick,
}: {
  id: string;
  label: string;
  hint?: string;
  active: boolean;
  onClick: () => void;
}) {
  return (
    <div
      id={id}
      role="option"
      aria-selected={active}
      onMouseEnter={() => undefined}
      onClick={onClick}
      className={`flex cursor-pointer items-center justify-between rounded-md px-2 py-2 text-body ${
        active ? "bg-accent text-foreground" : "text-foreground"
      }`}
    >
      <span className="truncate">{label}</span>
      {hint && (
        <kbd className="font-mono text-meta text-muted-foreground">{hint}</kbd>
      )}
    </div>
  );
}
