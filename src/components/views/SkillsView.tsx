/**
 * Skill management view (Req 22.3). The real implementation — searchable skill
 * list, skill editor, SKILL.md preview, version history, platform install/
 * uninstall, and safety scanning — lives under `src/features/skills`. This
 * module re-exports it so the application shell's view registry (which imports
 * from `../views/SkillsView`) resolves to the full view (task 22.3).
 */
export { SkillsView } from "../../features/skills/SkillsView";
