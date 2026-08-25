export interface PlatformModifier {
  symbol: string;
  isMatch: (event: KeyboardEvent) => boolean;
}

export function platformModifier(): PlatformModifier {
  const mac =
    typeof navigator !== "undefined" &&
    /mac|iphone|ipad|ipod/i.test(navigator.platform || navigator.userAgent);
  if (mac) {
    return {
      symbol: "⌘",
      isMatch: (event) => event.metaKey,
    };
  }
  return {
    symbol: "Ctrl",
    isMatch: (event) => event.ctrlKey,
  };
}

export function isTypingTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  if (target.isContentEditable) return true;
  const tag = target.tagName;
  return tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT";
}
