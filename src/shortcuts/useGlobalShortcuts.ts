import { useEffect } from "react";
import { useSettingsStore } from "../features/settings/settingsStore";
import { usePromptStore } from "../features/prompts/promptStore";
import { usePaletteStore } from "../features/prompts/paletteStore";
import { SHORTCUT_BINDINGS, matchesBinding } from "./bindings";
import { isTypingTarget, platformModifier } from "./platform";

export function useGlobalShortcuts(): void {
  useEffect(() => {
    const modifier = platformModifier();
    const onKeyDown = (event: KeyboardEvent) => {
      const typing = isTypingTarget(event.target);
      const paletteInput =
        event.target instanceof HTMLElement &&
        event.target.hasAttribute("data-command-palette-input");
      for (const binding of SHORTCUT_BINDINGS) {
        if (!matchesBinding(event, binding, modifier.isMatch(event))) continue;
        if (typing && !binding.allowWhileTyping && !paletteInput) return;
        event.preventDefault();
        runBinding(binding.id);
        return;
      }
    };
    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  }, []);
}

function runBinding(id: (typeof SHORTCUT_BINDINGS)[number]["id"]): void {
  if (id === "togglePalette") {
    usePaletteStore.getState().toggle();
    return;
  }
  if (id === "newPrompt") {
    usePromptStore.getState().createPromptAction?.();
    return;
  }
  if (id === "toggleTheme") {
    const settings = useSettingsStore.getState().settings;
    const current = settings?.theme ?? "dark";
    const isDark =
      current === "dark" ||
      (current === "system" &&
        typeof document !== "undefined" &&
        document.documentElement.classList.contains("dark"));
    void useSettingsStore.getState().setPreference("theme", isDark ? "light" : "dark");
    return;
  }
  const actions = usePromptStore.getState().detailActions;
  if (id === "savePrompt") {
    void actions?.save();
    return;
  }
  if (id === "copyPrompt") {
    void actions?.copy();
  }
}
