//! Skill_Service — local repository synchronization (Requirement 11).
//!
//! A skill may be backed by a *local repository*: a directory on disk that holds
//! the skill's `SKILL.md` plus any supporting files. This module discovers those
//! repositories ([`scan`]), lists their contents ([`tree`]), edits files
//! ([`read`], [`write`], [`mkdir`], [`rename`], [`delete`]), and reconciles a
//! skill record with its repository ([`sync`]).
//!
//! ## Testability / dependency injection
//!
//! Like the sibling services, every function takes its filesystem location as an
//! argument — a repository root `&Path` plus relative paths — rather than reaching
//! into global state. The Command_Layer (task 17.1) passes the configured skill
//! repository root; the unit tests below pass a [`tempfile`] tree. Nothing here
//! needs a live window.
//!
//! ## Path confinement (security — Req 11.8)
//!
//! Every relative path is resolved through [`resolve_within`], which is the
//! single security gate for this module. It works in two layers:
//!
//! 1. **Lexical normalization (always).** The relative path is split on both `/`
//!    and `\`, `.` segments are dropped, and each `..` pops the running component
//!    stack. A `..` that would pop *above* the root, an absolute path (`/x`), or a
//!    Windows drive/scheme prefix (`C:\x`) is rejected outright. Because the stack
//!    can never underflow past the root, the lexically-normalized result is always
//!    inside the root. This rejection happens *before any filesystem call*, so an
//!    escaping request performs no read, write, or delete (Req 11.8).
//! 2. **Symlink defense (when the path or an ancestor exists).** The normalized
//!    target — or its nearest existing ancestor — is canonicalized and verified to
//!    stay under the canonicalized root. This catches a symlink inside the
//!    repository that points outside it.
//!
//! Missing targets for read/rename/delete yield `NOT_FOUND` (Req 11.9); create
//! (`mkdir`/`rename` destination) onto an existing path yields `CONFLICT`
//! (Req 11.10).
#![allow(dead_code)]

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::models::Skill;
use crate::services::skill::{self, SkillUpdate};

/// The canonical skill definition file name discovered by [`scan`].
const SKILL_MD: &str = "SKILL.md";

/// A skill repository discovered by [`scan`] (Req 11.1).
///
/// `repo_path` is the per-skill repository root — the directory that contains the
/// discovered `SKILL.md` — and is what the other operations in this module accept
/// as their `repo_root`. `skill_md_relative_path` is the `SKILL.md` path relative
/// to that root (`SKILL.md`); the two joined locate the definition file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanEntry {
    /// The skill's local repository root (the directory holding its `SKILL.md`).
    pub repo_path: PathBuf,
    /// The `SKILL.md` path relative to `repo_path`.
    pub skill_md_relative_path: String,
}

/// One node in a skill repository file tree (Req 11.3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TreeEntry {
    /// Path relative to the repository root, using `/` separators.
    pub relative_path: String,
    /// `true` for a directory, `false` for a file.
    pub is_dir: bool,
}

/// Maps a raw I/O error into an `IO` [`AppError`].
fn io_err(context: &str, e: std::io::Error) -> AppError {
    AppError::io(format!("{context}: {e}"))
}

/// Returns `true` when `rel` looks like an absolute path: a leading separator
/// (`/foo`, `\foo`) or a Windows drive prefix (`C:\foo`, `c:foo`).
fn is_absolute_like(rel: &str) -> bool {
    if rel.starts_with('/') || rel.starts_with('\\') {
        return true;
    }
    let mut chars = rel.chars();
    matches!((chars.next(), chars.next()), (Some(a), Some(':')) if a.is_ascii_alphabetic())
}

/// Resolves a repository-relative path to an absolute path confined to `root`
/// (Req 11.8), or returns an error and touches nothing.
///
/// See the module documentation for the two-layer (lexical + symlink) approach.
fn resolve_within(root: &Path, rel: &str) -> Result<PathBuf, AppError> {
    if rel.contains('\0') {
        return Err(AppError::validation("path must not contain null bytes"));
    }
    if is_absolute_like(rel) {
        return Err(AppError::validation(
            "path must be relative to the repository root",
        ));
    }

    // Lexical normalization: build the component stack, refusing any escape.
    let mut stack: Vec<&str> = Vec::new();
    for comp in rel.split(|c| c == '/' || c == '\\') {
        match comp {
            "" | "." => continue,
            ".." => {
                if stack.pop().is_none() {
                    return Err(AppError::validation("path escapes the repository root"));
                }
            }
            other if other.contains(':') => {
                return Err(AppError::validation(
                    "path must not contain a drive or scheme prefix",
                ));
            }
            other => stack.push(other),
        }
    }

    let mut normalized = root.to_path_buf();
    for comp in &stack {
        normalized.push(comp);
    }

    // Symlink-escape defense for paths (or ancestors) that exist on disk.
    verify_canonical_within(root, &normalized)?;
    Ok(normalized)
}

/// Verifies that `target` — or its nearest existing ancestor — canonicalizes to a
/// location under the canonicalized `root`, defending against symlink escape.
///
/// When neither `root` nor any ancestor of `target` exists yet, the lexical
/// confinement performed by [`resolve_within`] already guarantees safety, so this
/// is a no-op in that case.
fn verify_canonical_within(root: &Path, target: &Path) -> Result<(), AppError> {
    let canon_root = match root.canonicalize() {
        Ok(p) => p,
        // Root not materialized; lexical confinement already holds.
        Err(_) => return Ok(()),
    };

    // Walk up to the nearest ancestor that exists (the target itself if present).
    let mut existing = target;
    loop {
        if existing.exists() {
            break;
        }
        match existing.parent() {
            Some(parent) => existing = parent,
            None => return Ok(()),
        }
    }

    let canon_existing = existing
        .canonicalize()
        .map_err(|e| io_err("failed to resolve path", e))?;
    if !canon_existing.starts_with(&canon_root) {
        return Err(AppError::validation("path escapes the repository root"));
    }
    Ok(())
}

/// Joins a path's components (relative to a root) into a `/`-separated string.
fn rel_to_slash(rel: &Path) -> String {
    rel.components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect::<Vec<_>>()
        .join("/")
}

/// Discovers every `SKILL.md` within the configured local skill locations
/// (Req 11.1), returning an empty list — never an error — when none exist
/// (Req 11.2).
///
/// Each discovered `SKILL.md` yields one [`ScanEntry`] whose `repo_path` is the
/// directory containing it. Locations that do not exist (or are not directories)
/// are skipped. Symlinked directories are not traversed, so the walk cannot loop.
/// Results are sorted by `repo_path` for a deterministic listing.
pub fn scan(locations: &[PathBuf]) -> Result<Vec<ScanEntry>, AppError> {
    let mut out = Vec::new();
    for location in locations {
        if location.is_dir() {
            collect_skill_md(location, &mut out)?;
        }
    }
    out.sort_by(|a, b| a.repo_path.cmp(&b.repo_path));
    Ok(out)
}

/// Recursively collects `SKILL.md` files under `dir` into `out`.
fn collect_skill_md(dir: &Path, out: &mut Vec<ScanEntry>) -> Result<(), AppError> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        // An unreadable directory is skipped rather than failing the whole scan.
        Err(_) => return Ok(()),
    };
    for entry in entries {
        let entry = entry.map_err(|e| io_err("failed to read directory entry", e))?;
        let file_type = entry
            .file_type()
            .map_err(|e| io_err("failed to determine entry type", e))?;
        let path = entry.path();
        if file_type.is_dir() {
            // `file_type` does not follow symlinks, so symlinked dirs are skipped.
            collect_skill_md(&path, out)?;
        } else if file_type.is_file() && entry.file_name() == OsStr::new(SKILL_MD) {
            if let Some(parent) = path.parent() {
                out.push(ScanEntry {
                    repo_path: parent.to_path_buf(),
                    skill_md_relative_path: SKILL_MD.to_string(),
                });
            }
        }
    }
    Ok(())
}

/// Lists every file and directory under a skill's repository root (Req 11.3).
///
/// Each [`TreeEntry`] carries its path relative to `repo_root` (with `/`
/// separators) and whether it is a directory. Returns `NOT_FOUND` when the
/// repository root does not exist. Results are sorted by relative path.
pub fn tree(repo_root: &Path) -> Result<Vec<TreeEntry>, AppError> {
    if !repo_root.is_dir() {
        return Err(AppError::not_found(format!(
            "skill repository `{}` not found",
            repo_root.display()
        )));
    }
    let mut out = Vec::new();
    collect_tree(repo_root, repo_root, &mut out)?;
    out.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    Ok(out)
}

/// Recursively collects tree entries under `dir`, relative to `root`.
fn collect_tree(root: &Path, dir: &Path, out: &mut Vec<TreeEntry>) -> Result<(), AppError> {
    for entry in fs::read_dir(dir).map_err(|e| io_err("failed to read directory", e))? {
        let entry = entry.map_err(|e| io_err("failed to read directory entry", e))?;
        let file_type = entry
            .file_type()
            .map_err(|e| io_err("failed to determine entry type", e))?;
        let path = entry.path();
        let relative = path.strip_prefix(root).unwrap_or(&path);
        let is_dir = file_type.is_dir();
        out.push(TreeEntry {
            relative_path: rel_to_slash(relative),
            is_dir,
        });
        if is_dir {
            collect_tree(root, &path, out)?;
        }
    }
    Ok(())
}

/// Reads a skill local file's content by relative path (Req 11.4).
///
/// Returns `NOT_FOUND` when no file exists at the path (Req 11.9), and rejects
/// path-escaping inputs with `VALIDATION` before any read (Req 11.8).
pub fn read(repo_root: &Path, rel: &str) -> Result<String, AppError> {
    let target = resolve_within(repo_root, rel)?;
    if !target.is_file() {
        return Err(AppError::not_found(format!("file `{rel}` not found")));
    }
    fs::read_to_string(&target).map_err(|e| io_err("failed to read file", e))
}

/// Writes content to a skill local file, creating missing parent directories
/// within the repository and replacing any existing content (Req 11.5).
///
/// Rejects path-escaping inputs with `VALIDATION` before any write (Req 11.8).
pub fn write(repo_root: &Path, rel: &str, content: &str) -> Result<(), AppError> {
    let target = resolve_within(repo_root, rel)?;
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|e| io_err("failed to create parent directories", e))?;
    }
    fs::write(&target, content).map_err(|e| io_err("failed to write file", e))
}

/// Creates a directory (and any missing parents) at a relative path (Req 11.6).
///
/// Returns `CONFLICT` when the path already exists (Req 11.10), and rejects
/// path-escaping inputs with `VALIDATION` before any change (Req 11.8).
pub fn mkdir(repo_root: &Path, rel: &str) -> Result<(), AppError> {
    let target = resolve_within(repo_root, rel)?;
    if target.exists() {
        return Err(AppError::conflict(format!("`{rel}` already exists")));
    }
    fs::create_dir_all(&target).map_err(|e| io_err("failed to create directory", e))
}

/// Renames/moves a file or directory within the repository (Req 11.6).
///
/// Returns `NOT_FOUND` when the source is missing (Req 11.9), `CONFLICT` when the
/// destination already exists (Req 11.10), and `VALIDATION` for path-escaping
/// inputs before any change (Req 11.8). Missing parent directories of the
/// destination are created.
pub fn rename(repo_root: &Path, from_rel: &str, to_rel: &str) -> Result<(), AppError> {
    let from = resolve_within(repo_root, from_rel)?;
    let to = resolve_within(repo_root, to_rel)?;
    if !from.exists() {
        return Err(AppError::not_found(format!("`{from_rel}` not found")));
    }
    if to.exists() {
        return Err(AppError::conflict(format!("`{to_rel}` already exists")));
    }
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| io_err("failed to create destination parent directories", e))?;
    }
    fs::rename(&from, &to).map_err(|e| io_err("failed to rename", e))
}

/// Deletes a file or directory within the repository (Req 11.6).
///
/// Returns `NOT_FOUND` when the target is missing (Req 11.9), and rejects
/// path-escaping inputs with `VALIDATION` before any deletion (Req 11.8).
/// Directories are removed recursively.
pub fn delete(repo_root: &Path, rel: &str) -> Result<(), AppError> {
    let target = resolve_within(repo_root, rel)?;
    if !target.exists() {
        return Err(AppError::not_found(format!("`{rel}` not found")));
    }
    if target.is_dir() {
        fs::remove_dir_all(&target).map_err(|e| io_err("failed to delete directory", e))
    } else {
        fs::remove_file(&target).map_err(|e| io_err("failed to delete file", e))
    }
}

/// Reconciles a skill record with its local repository (Req 11.7).
///
/// Reads the repository's `SKILL.md` and updates the skill's stored `content` to
/// match it, refreshing `updatedAt` through the skill service. Returns the updated
/// skill. Returns `NOT_FOUND` when the repository has no `SKILL.md` (Req 11.9) or
/// when the skill record does not exist.
pub fn sync(conn: &Connection, skill_id: &str, repo_root: &Path) -> Result<Skill, AppError> {
    let content = read(repo_root, SKILL_MD)?;
    skill::update(
        conn,
        skill_id,
        SkillUpdate {
            content: Some(content),
            ..Default::default()
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorCode;
    use crate::services::skill::{self, SkillCreate};
    use crate::storage::{create_memory_pool, init_schema, DbPool};
    use tempfile::TempDir;

    /// Builds an in-memory pool with the schema initialized (for [`sync`]).
    fn schema_pool() -> DbPool {
        let pool = create_memory_pool().expect("memory pool");
        init_schema(&pool.get().expect("conn")).expect("schema");
        pool
    }

    // --- scan (Req 11.1, 11.2) ---------------------------------------------

    #[test]
    fn scan_discovers_nested_skill_md_files() {
        let base = TempDir::new().unwrap();
        // location/release-sync/SKILL.md and location/spec-init/SKILL.md
        let loc = base.path().join("skills");
        fs::create_dir_all(loc.join("release-sync")).unwrap();
        fs::create_dir_all(loc.join("spec-init")).unwrap();
        fs::write(loc.join("release-sync").join("SKILL.md"), "a").unwrap();
        fs::write(loc.join("spec-init").join("SKILL.md"), "b").unwrap();
        // A non-SKILL file is ignored.
        fs::write(loc.join("release-sync").join("notes.md"), "x").unwrap();

        let entries = scan(&[loc.clone()]).unwrap();
        assert_eq!(entries.len(), 2);
        let paths: Vec<&PathBuf> = entries.iter().map(|e| &e.repo_path).collect();
        assert!(paths.contains(&&loc.join("release-sync")));
        assert!(paths.contains(&&loc.join("spec-init")));
        assert!(entries
            .iter()
            .all(|e| e.skill_md_relative_path == "SKILL.md"));
    }

    #[test]
    fn scan_returns_empty_without_error_when_none_found() {
        let base = TempDir::new().unwrap();
        let empty = base.path().join("empty");
        fs::create_dir_all(&empty).unwrap();
        // Existing-but-empty directory plus a nonexistent location.
        let missing = base.path().join("does-not-exist");

        let entries = scan(&[empty, missing]).unwrap();
        assert!(entries.is_empty());
    }

    // --- tree (Req 11.3) ---------------------------------------------------

    #[test]
    fn tree_lists_files_and_dirs_with_relative_paths() {
        let repo = TempDir::new().unwrap();
        fs::create_dir_all(repo.path().join("docs")).unwrap();
        fs::write(repo.path().join("SKILL.md"), "a").unwrap();
        fs::write(repo.path().join("docs").join("README.md"), "b").unwrap();

        let entries = tree(repo.path()).unwrap();
        // SKILL.md (file), docs (dir), docs/README.md (file).
        assert_eq!(entries.len(), 3);

        let skill_md = entries
            .iter()
            .find(|e| e.relative_path == "SKILL.md")
            .unwrap();
        assert!(!skill_md.is_dir);

        let docs = entries.iter().find(|e| e.relative_path == "docs").unwrap();
        assert!(docs.is_dir);

        let readme = entries
            .iter()
            .find(|e| e.relative_path == "docs/README.md")
            .unwrap();
        assert!(!readme.is_dir);
    }

    #[test]
    fn tree_missing_repo_returns_not_found() {
        let base = TempDir::new().unwrap();
        let err = tree(&base.path().join("nope")).unwrap_err();
        assert_eq!(err.code, ErrorCode::NotFound);
    }

    // --- read / write (Req 11.4, 11.5, 11.9) -------------------------------

    #[test]
    fn write_then_read_round_trips_and_creates_parents() {
        let repo = TempDir::new().unwrap();
        // Parent directory `a/b` does not exist yet; write must create it.
        write(repo.path(), "a/b/file.txt", "hello").unwrap();
        assert!(repo.path().join("a").join("b").join("file.txt").is_file());
        assert_eq!(read(repo.path(), "a/b/file.txt").unwrap(), "hello");
    }

    #[test]
    fn write_replaces_existing_content() {
        let repo = TempDir::new().unwrap();
        write(repo.path(), "SKILL.md", "first").unwrap();
        write(repo.path(), "SKILL.md", "second").unwrap();
        assert_eq!(read(repo.path(), "SKILL.md").unwrap(), "second");
    }

    #[test]
    fn read_missing_returns_not_found() {
        let repo = TempDir::new().unwrap();
        let err = read(repo.path(), "missing.txt").unwrap_err();
        assert_eq!(err.code, ErrorCode::NotFound);
    }

    // --- mkdir / rename / delete (Req 11.6, 11.9, 11.10) -------------------

    #[test]
    fn mkdir_creates_directory() {
        let repo = TempDir::new().unwrap();
        mkdir(repo.path(), "nested/dir").unwrap();
        assert!(repo.path().join("nested").join("dir").is_dir());
    }

    #[test]
    fn mkdir_onto_existing_path_returns_conflict() {
        let repo = TempDir::new().unwrap();
        write(repo.path(), "existing.txt", "x").unwrap();
        let err = mkdir(repo.path(), "existing.txt").unwrap_err();
        assert_eq!(err.code, ErrorCode::Conflict);
    }

    #[test]
    fn rename_moves_file() {
        let repo = TempDir::new().unwrap();
        write(repo.path(), "old.txt", "content").unwrap();
        rename(repo.path(), "old.txt", "sub/new.txt").unwrap();
        assert!(!repo.path().join("old.txt").exists());
        assert_eq!(read(repo.path(), "sub/new.txt").unwrap(), "content");
    }

    #[test]
    fn rename_onto_existing_returns_conflict_and_leaves_both() {
        let repo = TempDir::new().unwrap();
        write(repo.path(), "a.txt", "A").unwrap();
        write(repo.path(), "b.txt", "B").unwrap();
        let err = rename(repo.path(), "a.txt", "b.txt").unwrap_err();
        assert_eq!(err.code, ErrorCode::Conflict);
        // Both files untouched.
        assert_eq!(read(repo.path(), "a.txt").unwrap(), "A");
        assert_eq!(read(repo.path(), "b.txt").unwrap(), "B");
    }

    #[test]
    fn rename_missing_source_returns_not_found() {
        let repo = TempDir::new().unwrap();
        let err = rename(repo.path(), "missing.txt", "dest.txt").unwrap_err();
        assert_eq!(err.code, ErrorCode::NotFound);
    }

    #[test]
    fn delete_removes_file_and_directory() {
        let repo = TempDir::new().unwrap();
        write(repo.path(), "f.txt", "x").unwrap();
        write(repo.path(), "dir/inner.txt", "y").unwrap();

        delete(repo.path(), "f.txt").unwrap();
        assert!(!repo.path().join("f.txt").exists());

        delete(repo.path(), "dir").unwrap();
        assert!(!repo.path().join("dir").exists());
    }

    #[test]
    fn delete_missing_returns_not_found() {
        let repo = TempDir::new().unwrap();
        let err = delete(repo.path(), "missing.txt").unwrap_err();
        assert_eq!(err.code, ErrorCode::NotFound);
    }

    // --- sync (Req 11.7) ---------------------------------------------------

    #[test]
    fn sync_updates_skill_content_from_repo() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let repo = TempDir::new().unwrap();

        let created = skill::create(
            &conn,
            SkillCreate {
                name: "Synced".into(),
                content: Some("stale".into()),
                ..Default::default()
            },
        )
        .unwrap();

        let fresh = "---\nname: Synced\ndescription: d\n---\nfresh body";
        fs::write(repo.path().join("SKILL.md"), fresh).unwrap();

        let updated = sync(&conn, &created.id, repo.path()).unwrap();
        assert_eq!(updated.content.as_deref(), Some(fresh));
        // Persisted, not just returned.
        assert_eq!(
            skill::get(&conn, &created.id).unwrap().content.as_deref(),
            Some(fresh)
        );
    }

    #[test]
    fn sync_missing_skill_md_returns_not_found() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let repo = TempDir::new().unwrap();
        let created = skill::create(
            &conn,
            SkillCreate {
                name: "S".into(),
                ..Default::default()
            },
        )
        .unwrap();
        let err = sync(&conn, &created.id, repo.path()).unwrap_err();
        assert_eq!(err.code, ErrorCode::NotFound);
    }

    // --- path confinement (security — Req 11.8) ----------------------------

    #[test]
    fn parent_traversal_is_rejected_with_no_mutation() {
        let base = TempDir::new().unwrap();
        let repo = base.path().join("repo");
        fs::create_dir_all(&repo).unwrap();
        // An outside file the attacker would try to clobber.
        let outside = base.path().join("outside.txt");
        fs::write(&outside, "SECRET").unwrap();

        let err = write(&repo, "../outside.txt", "HACKED").unwrap_err();
        assert_eq!(err.code, ErrorCode::Validation);
        // The outside file is untouched (no filesystem mutation).
        assert_eq!(fs::read_to_string(&outside).unwrap(), "SECRET");
    }

    #[test]
    fn absolute_path_is_rejected_with_no_mutation() {
        let base = TempDir::new().unwrap();
        let repo = base.path().join("repo");
        fs::create_dir_all(&repo).unwrap();
        let outside = base.path().join("abs-target.txt");
        fs::write(&outside, "SECRET").unwrap();

        // Use the real absolute path of the outside file.
        let abs = outside.to_str().unwrap();
        let err = write(&repo, abs, "HACKED").unwrap_err();
        assert_eq!(err.code, ErrorCode::Validation);
        assert_eq!(fs::read_to_string(&outside).unwrap(), "SECRET");
    }

    #[test]
    fn nested_escape_is_rejected_with_no_mutation() {
        let base = TempDir::new().unwrap();
        let repo = base.path().join("repo");
        fs::create_dir_all(&repo).unwrap();
        let outside = base.path().join("nested-target.txt");
        fs::write(&outside, "SECRET").unwrap();

        // `sub/../../nested-target.txt` escapes after collapsing `..`.
        let err = write(&repo, "sub/../../nested-target.txt", "HACKED").unwrap_err();
        assert_eq!(err.code, ErrorCode::Validation);
        assert_eq!(fs::read_to_string(&outside).unwrap(), "SECRET");
        // No partial `sub` directory was created either.
        assert!(!repo.join("sub").exists());
    }

    #[test]
    fn escaping_read_and_delete_are_rejected_before_io() {
        let base = TempDir::new().unwrap();
        let repo = base.path().join("repo");
        fs::create_dir_all(&repo).unwrap();
        let outside = base.path().join("ro.txt");
        fs::write(&outside, "SECRET").unwrap();

        assert_eq!(
            read(&repo, "../ro.txt").unwrap_err().code,
            ErrorCode::Validation
        );
        assert_eq!(
            delete(&repo, "../ro.txt").unwrap_err().code,
            ErrorCode::Validation
        );
        // Confinement (not NOT_FOUND) wins, and the file survives.
        assert_eq!(fs::read_to_string(&outside).unwrap(), "SECRET");
    }
}
