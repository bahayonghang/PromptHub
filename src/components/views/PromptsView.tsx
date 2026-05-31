/**
 * Prompt editing view (Req 22.3). The real implementation — folder tree,
 * searchable prompt list, prompt editor, and version history — lives under
 * `src/features/prompts`. This module re-exports it so the application shell's
 * view registry (which imports from `../views/PromptsView`) resolves to the
 * full view (task 22.2).
 */
export { PromptsView } from "../../features/prompts/PromptsView";
