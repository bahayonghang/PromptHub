export type ShortcutId =
  | "togglePalette"
  | "newPrompt"
  | "toggleTheme"
  | "savePrompt"
  | "copyPrompt";

export interface ShortcutBinding {
  id: ShortcutId;
  key: string;
  shift?: boolean;
  allowWhileTyping: boolean;
}

export const SHORTCUT_BINDINGS: readonly ShortcutBinding[] = [
  { id: "togglePalette", key: "k", allowWhileTyping: false },
  { id: "newPrompt", key: "n", allowWhileTyping: false },
  { id: "toggleTheme", key: "l", shift: true, allowWhileTyping: false },
  { id: "savePrompt", key: "s", allowWhileTyping: true },
  { id: "copyPrompt", key: "enter", allowWhileTyping: true },
];

export function formatBinding(
  binding: ShortcutBinding,
  modifierSymbol: string,
): string {
  const key = binding.key === "enter" ? "Enter" : binding.key.toUpperCase();
  return binding.shift
    ? `${modifierSymbol}+Shift+${key}`
    : `${modifierSymbol}+${key}`;
}

export function matchesBinding(
  event: KeyboardEvent,
  binding: ShortcutBinding,
  modifierMatches: boolean,
): boolean {
  if (!modifierMatches) return false;
  if (Boolean(binding.shift) !== event.shiftKey) return false;
  return event.key.toLowerCase() === binding.key;
}
