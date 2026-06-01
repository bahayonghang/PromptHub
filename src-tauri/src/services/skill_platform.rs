//! Skill_Service — platform integration (Requirement 12).
//!
//! This module installs, uninstalls, and reports the install state of skills
//! against external AI-tool platforms (Claude Code, Cursor, Codex, …). Each
//! platform owns a conventional per-user *root directory* (e.g. `~/.claude`) and
//! a *skills directory* under it (`<root>/skills`) into which a skill's complete
//! file set is copied as a subdirectory named for the skill.
//!
//! ## Testability / dependency injection
//!
//! Like the other services in this crate, the rules here are written against
//! *injected* filesystem locations rather than reaching into global state. The
//! conventional root directories are resolved from a supplied `home` directory
//! (the Command_Layer in task 17.1 passes the real per-user home), so every
//! behavior — detection, install, uninstall, status — is exercised against a
//! [`tempfile`] tree in tests with no live window. [`list_platforms`] builds the
//! resolved [`Platform`] descriptors (built-in + caller-supplied custom); the
//! mutating/reading operations take that resolved slice plus a target platform
//! id, so an unknown id is rejected without touching the filesystem (Req 12.7).
//!
//! ## Path confinement (security — Req 12.8)
//!
//! A skill name must resolve to a *direct child* of the target platform's skills
//! directory. Names containing path separators, parent references (`..`), drive
//! prefixes, or absolute paths are rejected with `VALIDATION` *before* any
//! filesystem change. The relative paths of the copied file set are likewise
//! confined to the skill's own subdirectory: `..` components and absolute paths
//! are rejected, and the resolved destination must stay within the skill
//! directory. Because all paths are validated up front, a rejected request makes
//! no filesystem change (Req 2.3, 12.8).
#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::AppError;

/// A built-in supported platform descriptor (id, display name, and the
/// conventional root directory relative to the user's home).
///
/// `skills` is always the skills subdirectory name in the reference, so it is
/// applied uniformly rather than stored per entry.
struct Builtin {
    /// Stable platform identifier.
    id: &'static str,
    /// Human-readable display name.
    name: &'static str,
    /// Root directory relative to the user's home (forward slashes; joined
    /// component-wise so it is correct on every platform).
    root_relative: &'static str,
}

/// The built-in supported platforms, mirroring the reference
/// `SKILL_PLATFORMS` constant (`packages/shared/constants/platforms.ts`).
const BUILTINS: &[Builtin] = &[
    Builtin {
        id: "claude",
        name: "Claude Code",
        root_relative: ".claude",
    },
    Builtin {
        id: "copilot",
        name: "GitHub Copilot",
        root_relative: ".copilot",
    },
    Builtin {
        id: "cursor",
        name: "Cursor",
        root_relative: ".cursor",
    },
    Builtin {
        id: "windsurf",
        name: "Windsurf",
        root_relative: ".codeium/windsurf",
    },
    Builtin {
        id: "kiro",
        name: "Kiro",
        root_relative: ".kiro",
    },
    Builtin {
        id: "gemini",
        name: "Gemini CLI",
        root_relative: ".gemini",
    },
    Builtin {
        id: "antigravity",
        name: "Antigravity",
        root_relative: ".gemini/antigravity",
    },
    Builtin {
        id: "trae",
        name: "Trae",
        root_relative: ".trae",
    },
    Builtin {
        id: "trae-cn",
        name: "Trae CN",
        root_relative: ".trae-cn",
    },
    Builtin {
        id: "opencode",
        name: "OpenCode",
        root_relative: ".config/opencode",
    },
    Builtin {
        id: "cline",
        name: "Cline",
        root_relative: ".cline",
    },
    Builtin {
        id: "codex",
        name: "Codex CLI",
        root_relative: ".codex",
    },
    Builtin {
        id: "kilo",
        name: "Kilo Code",
        root_relative: ".kilo",
    },
    Builtin {
        id: "amp",
        name: "Amp",
        root_relative: ".config/amp",
    },
    Builtin {
        id: "openclaw",
        name: "OpenClaw",
        root_relative: ".openclaw",
    },
    Builtin {
        id: "qoder",
        name: "Qoder",
        root_relative: ".qoder",
    },
    Builtin {
        id: "qoderwork",
        name: "QoderWorker",
        root_relative: ".qoderwork",
    },
    Builtin {
        id: "hermes",
        name: "Hermes Agent",
        root_relative: ".hermes",
    },
    Builtin {
        id: "codebuddy",
        name: "CodeBuddy",
        root_relative: ".codebuddy",
    },
];

/// The skills subdirectory name used by every built-in platform.
const SKILLS_SUBDIR: &str = "skills";

/// A caller-supplied enabled custom platform (Req 12.1).
///
/// The Command_Layer constructs these from the user's settings. `skills_dir`
/// overrides the default `<root_dir>/skills` location when present.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomPlatform {
    /// Stable platform identifier.
    pub id: String,
    /// Human-readable display name.
    pub name: String,
    /// Conventional root directory for the platform.
    pub root_dir: PathBuf,
    /// Optional explicit skills directory; defaults to `<root_dir>/skills`.
    #[serde(default)]
    pub skills_dir: Option<PathBuf>,
}

/// A supported platform with its resolved filesystem locations (Req 12.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Platform {
    /// Stable platform identifier.
    pub id: String,
    /// Human-readable display name.
    pub name: String,
    /// Whether this is a caller-supplied custom platform.
    pub is_custom: bool,
    /// Conventional root directory (existence indicates the tool is installed).
    pub root_dir: PathBuf,
    /// Target skills directory the skill file set is copied into.
    pub skills_dir: PathBuf,
}

/// One file in a skill's complete file set (Req 12.3).
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillFile {
    /// Path relative to the skill's installed directory (may include
    /// subdirectories; confined to the skill directory before any write).
    pub relative_path: String,
    /// File content.
    pub content: String,
}

/// Result of a successful install identifying the target platform (Req 12.3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallResult {
    /// The platform the skill was installed to.
    pub platform_id: String,
    /// Always `true` on a successful install.
    pub installed: bool,
}

/// Per-platform install state for a skill (Req 12.5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformInstallStatus {
    /// The platform this entry describes.
    pub platform_id: String,
    /// Whether the skill is currently installed on the platform.
    pub installed: bool,
    /// The skills directory location associated with the platform.
    pub skills_dir: PathBuf,
}

/// Maps a raw I/O error into an `IO` [`AppError`].
fn io_err(context: &str, e: std::io::Error) -> AppError {
    AppError::io(format!("{context}: {e}"))
}

/// Joins a `/`- or `\\`-separated relative path onto `base`, component-wise, so
/// multi-segment roots (e.g. `.codeium/windsurf`) resolve correctly on every OS.
fn join_relative(base: &Path, rel: &str) -> PathBuf {
    let mut path = base.to_path_buf();
    for comp in rel.split(['/', '\\']) {
        if !comp.is_empty() {
            path.push(comp);
        }
    }
    path
}

/// Returns the supported platforms: every built-in plus every supplied custom
/// platform, each with its resolved root and skills directories (Req 12.1).
///
/// Built-in root directories are resolved relative to `home` (the per-user home
/// directory the Command_Layer supplies). Custom platforms use their supplied
/// `root_dir`, defaulting the skills directory to `<root_dir>/skills` when no
/// explicit `skills_dir` is given.
pub fn list_platforms(home: &Path, custom: &[CustomPlatform]) -> Vec<Platform> {
    let mut platforms = Vec::with_capacity(BUILTINS.len() + custom.len());

    for builtin in BUILTINS {
        let root_dir = join_relative(home, builtin.root_relative);
        let skills_dir = root_dir.join(SKILLS_SUBDIR);
        platforms.push(Platform {
            id: builtin.id.to_string(),
            name: builtin.name.to_string(),
            is_custom: false,
            root_dir,
            skills_dir,
        });
    }

    for c in custom {
        let skills_dir = c
            .skills_dir
            .clone()
            .unwrap_or_else(|| c.root_dir.join(SKILLS_SUBDIR));
        platforms.push(Platform {
            id: c.id.clone(),
            name: c.name.clone(),
            is_custom: true,
            root_dir: c.root_dir.clone(),
            skills_dir,
        });
    }

    platforms
}

/// Returns the identifiers of the supported platforms whose conventional root
/// directory exists on the host filesystem (Req 12.2).
///
/// Returns an empty vector when none exist. Order follows the supplied slice.
pub fn detect(platforms: &[Platform]) -> Vec<String> {
    platforms
        .iter()
        .filter(|p| p.root_dir.exists())
        .map(|p| p.id.clone())
        .collect()
}

/// Looks up a platform by id, returning `NOT_FOUND` when the id is absent from
/// the supported platform list (Req 12.7).
fn find_platform<'a>(platforms: &'a [Platform], id: &str) -> Result<&'a Platform, AppError> {
    platforms.iter().find(|p| p.id == id).ok_or_else(|| {
        AppError::not_found(format!(
            "platform `{id}` is not in the supported platform list"
        ))
    })
}

/// Resolves a skill name to its install directory directly under `skills_dir`,
/// rejecting path-escaping names with `VALIDATION` before any filesystem change
/// (Req 12.8).
///
/// A valid name is a single path component: not empty, with no path separators,
/// not `.`/`..`, and not a drive/absolute reference. The result is verified to be
/// a direct child of `skills_dir` as defense in depth.
fn safe_skill_dir(skills_dir: &Path, skill_name: &str) -> Result<PathBuf, AppError> {
    let name = skill_name.trim();
    if name.is_empty() {
        return Err(AppError::validation("skill name is required"));
    }
    if name.contains('\0') {
        return Err(AppError::validation(
            "skill name must not contain null bytes",
        ));
    }
    if name.contains('/') || name.contains('\\') {
        return Err(AppError::validation(
            "skill name must not contain path separators",
        ));
    }
    if name == "." || name == ".." {
        return Err(AppError::validation(
            "skill name must not be a path reference",
        ));
    }

    let candidate = skills_dir.join(name);
    // Defense in depth: the install location must be a direct child of the
    // platform skills directory (catches drive-relative names on Windows).
    if candidate.parent() != Some(skills_dir) || !candidate.starts_with(skills_dir) {
        return Err(AppError::validation(
            "skill name resolves outside the platform skills directory",
        ));
    }
    Ok(candidate)
}

/// Joins a relative file path onto `base`, neutralizing escapes (Req 12.8).
///
/// Empty/`.` segments are skipped, `..` segments and absolute/drive-rooted paths
/// are rejected, and the resolved destination must remain within `base`.
fn safe_relative_join(base: &Path, rel: &str) -> Result<PathBuf, AppError> {
    if rel.contains('\0') {
        return Err(AppError::validation(
            "file path must not contain null bytes",
        ));
    }

    let mut out = base.to_path_buf();
    let mut pushed = false;
    for comp in rel.split(['/', '\\']) {
        if comp.is_empty() || comp == "." {
            continue;
        }
        if comp == ".." {
            return Err(AppError::validation(
                "file path must not escape the skill directory",
            ));
        }
        out.push(comp);
        pushed = true;
    }

    if !pushed {
        return Err(AppError::validation("file path must not be empty"));
    }
    // A pushed absolute/drive component resets the path; this catches that.
    if !out.starts_with(base) {
        return Err(AppError::validation(
            "file path resolves outside the skill directory",
        ));
    }
    Ok(out)
}

/// Installs a skill's complete file set into a platform's skills directory under
/// a subdirectory named for the skill (Req 12.3, 12.8).
///
/// The platform id must be present in `platforms` (else `NOT_FOUND`, Req 12.7),
/// and the skill name must resolve inside the platform skills directory (else
/// `VALIDATION`, Req 12.8). Every file path is validated *before* any write so a
/// rejected request makes no filesystem change. The install is clean: any
/// existing installed directory for the skill is removed first, so the result
/// matches the supplied file set exactly.
pub fn install(
    platforms: &[Platform],
    platform_id: &str,
    skill_name: &str,
    files: &[SkillFile],
) -> Result<InstallResult, AppError> {
    let platform = find_platform(platforms, platform_id)?;
    let target = safe_skill_dir(&platform.skills_dir, skill_name)?;

    // Pre-validate every destination so an escaping path causes no FS change.
    let mut planned: Vec<(PathBuf, &str)> = Vec::with_capacity(files.len());
    for file in files {
        let dest = safe_relative_join(&target, &file.relative_path)?;
        planned.push((dest, &file.content));
    }

    // All paths valid — perform a clean install.
    if target.exists() {
        fs::remove_dir_all(&target)
            .map_err(|e| io_err("failed to clear existing skill install directory", e))?;
    }
    fs::create_dir_all(&target)
        .map_err(|e| io_err("failed to create skill install directory", e))?;
    for (dest, content) in planned {
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| io_err("failed to create skill file directory", e))?;
        }
        fs::write(&dest, content).map_err(|e| io_err("failed to write skill file", e))?;
    }

    Ok(InstallResult {
        platform_id: platform.id.clone(),
        installed: true,
    })
}

/// Removes a skill's installed file set from a platform's skills directory
/// (Req 12.4), completing successfully when the skill is not installed.
///
/// The platform id must be present in `platforms` (else `NOT_FOUND`, Req 12.7),
/// and the skill name must resolve inside the platform skills directory (else
/// `VALIDATION`, Req 12.8). When the install directory does not exist the call
/// is a no-op and returns success.
pub fn uninstall(
    platforms: &[Platform],
    platform_id: &str,
    skill_name: &str,
) -> Result<(), AppError> {
    let platform = find_platform(platforms, platform_id)?;
    let target = safe_skill_dir(&platform.skills_dir, skill_name)?;

    if target.exists() {
        fs::remove_dir_all(&target)
            .map_err(|e| io_err("failed to remove installed skill directory", e))?;
    }
    Ok(())
}

/// Reports, for each supported platform, whether the skill is installed and the
/// platform's skills directory location (Req 12.5).
///
/// The skill name is confined to each platform's skills directory; an escaping
/// name is rejected with `VALIDATION` before any read (Req 12.8).
pub fn status(
    platforms: &[Platform],
    skill_name: &str,
) -> Result<Vec<PlatformInstallStatus>, AppError> {
    let mut out = Vec::with_capacity(platforms.len());
    for platform in platforms {
        let target = safe_skill_dir(&platform.skills_dir, skill_name)?;
        out.push(PlatformInstallStatus {
            platform_id: platform.id.clone(),
            installed: target.is_dir(),
            skills_dir: platform.skills_dir.clone(),
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorCode;
    use std::fs;
    use tempfile::TempDir;

    fn file(path: &str, content: &str) -> SkillFile {
        SkillFile {
            relative_path: path.to_string(),
            content: content.to_string(),
        }
    }

    fn sample_files() -> Vec<SkillFile> {
        vec![
            file("SKILL.md", "---\nname: demo\ndescription: d\n---\nbody"),
            file("docs/README.md", "# readme"),
        ]
    }

    /// Helper: a single resolved platform whose skills dir lives under `base`.
    fn one_platform(base: &Path, id: &str) -> Vec<Platform> {
        let root_dir = base.join(format!(".{id}"));
        let skills_dir = root_dir.join(SKILLS_SUBDIR);
        vec![Platform {
            id: id.to_string(),
            name: id.to_string(),
            is_custom: false,
            root_dir,
            skills_dir,
        }]
    }

    // --- list_platforms (Req 12.1) -----------------------------------------

    #[test]
    fn list_platforms_returns_builtins_with_resolved_dirs() {
        let home = TempDir::new().unwrap();
        let platforms = list_platforms(home.path(), &[]);

        // Every built-in is present.
        assert_eq!(platforms.len(), BUILTINS.len());

        let claude = platforms.iter().find(|p| p.id == "claude").unwrap();
        assert_eq!(claude.name, "Claude Code");
        assert!(!claude.is_custom);
        assert_eq!(claude.root_dir, home.path().join(".claude"));
        assert_eq!(
            claude.skills_dir,
            home.path().join(".claude").join("skills")
        );

        // Multi-segment root resolves component-wise.
        let windsurf = platforms.iter().find(|p| p.id == "windsurf").unwrap();
        assert_eq!(
            windsurf.root_dir,
            home.path().join(".codeium").join("windsurf")
        );
    }

    #[test]
    fn list_platforms_includes_custom_platforms() {
        let home = TempDir::new().unwrap();
        let custom_root = TempDir::new().unwrap();
        let custom = vec![CustomPlatform {
            id: "team-agents".to_string(),
            name: "Team Agents".to_string(),
            root_dir: custom_root.path().to_path_buf(),
            skills_dir: None,
        }];

        let platforms = list_platforms(home.path(), &custom);
        assert_eq!(platforms.len(), BUILTINS.len() + 1);

        let entry = platforms.iter().find(|p| p.id == "team-agents").unwrap();
        assert!(entry.is_custom);
        assert_eq!(entry.name, "Team Agents");
        assert_eq!(entry.skills_dir, custom_root.path().join("skills"));
    }

    #[test]
    fn list_platforms_custom_skills_dir_override_is_used() {
        let home = TempDir::new().unwrap();
        let root = TempDir::new().unwrap();
        let explicit = root.path().join("custom-skills");
        let custom = vec![CustomPlatform {
            id: "x".to_string(),
            name: "X".to_string(),
            root_dir: root.path().to_path_buf(),
            skills_dir: Some(explicit.clone()),
        }];

        let platforms = list_platforms(home.path(), &custom);
        let entry = platforms.iter().find(|p| p.id == "x").unwrap();
        assert_eq!(entry.skills_dir, explicit);
    }

    // --- detect (Req 12.2) -------------------------------------------------

    #[test]
    fn detect_reports_existing_root_dirs_only() {
        let home = TempDir::new().unwrap();
        // Create the conventional root dir for claude only.
        fs::create_dir_all(home.path().join(".claude")).unwrap();

        let platforms = list_platforms(home.path(), &[]);
        let detected = detect(&platforms);

        assert!(detected.contains(&"claude".to_string()));
        assert!(!detected.contains(&"cursor".to_string()));
    }

    #[test]
    fn detect_returns_empty_when_no_root_dirs_exist() {
        let home = TempDir::new().unwrap();
        let platforms = list_platforms(home.path(), &[]);
        assert!(detect(&platforms).is_empty());
    }

    // --- install (Req 12.3, 12.7, 12.8) ------------------------------------

    #[test]
    fn install_copies_file_set_into_named_subdirectory() {
        let base = TempDir::new().unwrap();
        let platforms = one_platform(base.path(), "claude");

        let result = install(&platforms, "claude", "my-skill", &sample_files()).unwrap();
        assert_eq!(result.platform_id, "claude");
        assert!(result.installed);

        let skill_dir = platforms[0].skills_dir.join("my-skill");
        assert_eq!(
            fs::read_to_string(skill_dir.join("SKILL.md")).unwrap(),
            "---\nname: demo\ndescription: d\n---\nbody"
        );
        assert_eq!(
            fs::read_to_string(skill_dir.join("docs").join("README.md")).unwrap(),
            "# readme"
        );
    }

    #[test]
    fn install_is_clean_replacing_previous_file_set() {
        let base = TempDir::new().unwrap();
        let platforms = one_platform(base.path(), "claude");

        install(&platforms, "claude", "my-skill", &sample_files()).unwrap();
        // Re-install with a smaller set; the stale file must be gone.
        install(&platforms, "claude", "my-skill", &[file("SKILL.md", "new")]).unwrap();

        let skill_dir = platforms[0].skills_dir.join("my-skill");
        assert_eq!(
            fs::read_to_string(skill_dir.join("SKILL.md")).unwrap(),
            "new"
        );
        assert!(!skill_dir.join("docs").join("README.md").exists());
    }

    #[test]
    fn install_rejects_unknown_platform_without_fs_change() {
        let base = TempDir::new().unwrap();
        let platforms = one_platform(base.path(), "claude");

        let err = install(&platforms, "nope", "my-skill", &sample_files()).unwrap_err();
        assert_eq!(err.code, ErrorCode::NotFound);

        // No skills directory was created for the (nonexistent) platform.
        assert!(!platforms[0].skills_dir.exists());
    }

    #[test]
    fn install_rejects_parent_escaping_skill_name_without_writing_outside() {
        let base = TempDir::new().unwrap();
        let platforms = one_platform(base.path(), "claude");

        let err = install(&platforms, "claude", "../evil", &sample_files()).unwrap_err();
        assert_eq!(err.code, ErrorCode::Validation);

        // Nothing was written anywhere under the base.
        assert!(!base.path().join(".claude").join("evil").exists());
        assert!(!platforms[0].skills_dir.exists());
    }

    #[test]
    fn install_rejects_separator_skill_name() {
        let base = TempDir::new().unwrap();
        let platforms = one_platform(base.path(), "claude");

        let err = install(&platforms, "claude", "nested/skill", &sample_files()).unwrap_err();
        assert_eq!(err.code, ErrorCode::Validation);
        assert!(!platforms[0].skills_dir.exists());
    }

    #[test]
    fn install_rejects_absolute_skill_name() {
        let base = TempDir::new().unwrap();
        let platforms = one_platform(base.path(), "claude");

        // Absolute path contains separators -> rejected with no write.
        let abs = if cfg!(windows) { "C:\\evil" } else { "/evil" };
        let err = install(&platforms, "claude", abs, &sample_files()).unwrap_err();
        assert_eq!(err.code, ErrorCode::Validation);
        assert!(!platforms[0].skills_dir.exists());
    }

    #[test]
    fn install_rejects_escaping_file_relative_path_without_writing() {
        let base = TempDir::new().unwrap();
        let platforms = one_platform(base.path(), "claude");

        let err = install(
            &platforms,
            "claude",
            "my-skill",
            &[file("../escape.txt", "x")],
        )
        .unwrap_err();
        assert_eq!(err.code, ErrorCode::Validation);

        // The skill directory was not created (validation happens before writes).
        assert!(!platforms[0].skills_dir.join("my-skill").exists());
        assert!(!platforms[0].skills_dir.join("escape.txt").exists());
    }

    // --- uninstall (Req 12.4, 12.7, 12.8) ----------------------------------

    #[test]
    fn uninstall_removes_installed_skill() {
        let base = TempDir::new().unwrap();
        let platforms = one_platform(base.path(), "claude");
        install(&platforms, "claude", "my-skill", &sample_files()).unwrap();
        let skill_dir = platforms[0].skills_dir.join("my-skill");
        assert!(skill_dir.exists());

        uninstall(&platforms, "claude", "my-skill").unwrap();
        assert!(!skill_dir.exists());
    }

    #[test]
    fn uninstall_is_noop_when_not_installed() {
        let base = TempDir::new().unwrap();
        let platforms = one_platform(base.path(), "claude");

        // Skill was never installed — succeeds without error (Req 12.4).
        uninstall(&platforms, "claude", "absent-skill").unwrap();
    }

    #[test]
    fn uninstall_rejects_unknown_platform() {
        let base = TempDir::new().unwrap();
        let platforms = one_platform(base.path(), "claude");
        let err = uninstall(&platforms, "nope", "my-skill").unwrap_err();
        assert_eq!(err.code, ErrorCode::NotFound);
    }

    #[test]
    fn uninstall_rejects_escaping_skill_name() {
        let base = TempDir::new().unwrap();
        let platforms = one_platform(base.path(), "claude");
        let err = uninstall(&platforms, "claude", "..").unwrap_err();
        assert_eq!(err.code, ErrorCode::Validation);
    }

    // --- status (Req 12.5, 12.8) -------------------------------------------

    #[test]
    fn status_reports_per_platform_install_state() {
        let home = TempDir::new().unwrap();
        let platforms = list_platforms(home.path(), &[]);

        // Install onto claude only.
        install(&platforms, "claude", "my-skill", &sample_files()).unwrap();

        let report = status(&platforms, "my-skill").unwrap();
        assert_eq!(report.len(), platforms.len());

        let claude = report.iter().find(|s| s.platform_id == "claude").unwrap();
        assert!(claude.installed);
        assert_eq!(
            claude.skills_dir,
            home.path().join(".claude").join("skills")
        );

        let cursor = report.iter().find(|s| s.platform_id == "cursor").unwrap();
        assert!(!cursor.installed);
    }

    #[test]
    fn status_rejects_escaping_skill_name() {
        let home = TempDir::new().unwrap();
        let platforms = list_platforms(home.path(), &[]);
        let err = status(&platforms, "../evil").unwrap_err();
        assert_eq!(err.code, ErrorCode::Validation);
    }
}
