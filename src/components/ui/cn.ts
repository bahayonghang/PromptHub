/**
 * Joins conditional class names. Falsy entries are dropped so callers can write
 * `cn("base", active && "is-active")` without emitting "false" into the DOM.
 *
 * Deliberately dependency-free: the project ships no `clsx`/`tailwind-merge`, and
 * the primitives here compose class strings in a fixed order (base -> variant ->
 * size -> caller override), so last-wins conflicts are not a concern in practice.
 */
export function cn(...parts: Array<string | false | null | undefined>): string {
  return parts.filter(Boolean).join(" ");
}
