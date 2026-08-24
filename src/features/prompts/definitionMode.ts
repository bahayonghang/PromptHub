/**
 * Remembers whether the prompt editor should present chat mode. The last
 * explicit Text/Chat choice is kept in memory and in localStorage so switching
 * prompts or creating a new one does not snap back to text mode.
 */

const STORAGE_KEY = "prompthub.editor.preferChatMode";

let memoryPreferred = true;
let hydrated = false;

function readStorage(): boolean | null {
  try {
    const raw = globalThis.localStorage?.getItem(STORAGE_KEY);
    if (raw === "0") return false;
    if (raw === "1") return true;
  } catch {
    // Storage can be missing in tests or blocked in a webview.
  }
  return null;
}

function writeStorage(chat: boolean): void {
  try {
    globalThis.localStorage?.setItem(STORAGE_KEY, chat ? "1" : "0");
  } catch {
    // Memory still holds the choice for the rest of the session.
  }
}

/** Returns the remembered editor mode. Defaults to chat. */
export function preferredChatMode(): boolean {
  if (!hydrated) {
    hydrated = true;
    const stored = readStorage();
    if (stored != null) memoryPreferred = stored;
  }
  return memoryPreferred;
}

/** Records an explicit Text/Chat toggle. */
export function setPreferredChatMode(chat: boolean): void {
  memoryPreferred = chat;
  hydrated = true;
  writeStorage(chat);
}

/** Test-only reset so suites do not leak the remembered mode. */
export function resetPreferredChatModeForTests(): void {
  memoryPreferred = true;
  hydrated = false;
  try {
    globalThis.localStorage?.removeItem(STORAGE_KEY);
  } catch {
    // ignore
  }
}
