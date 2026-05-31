//! Property-based test for runtime directory resolution (task 23.2).
//!
//! Runs as an **integration test** against the public `prompthub_lib` API
//! (`services::data_path::{resolve_runtime_paths, database_path,
//! ensure_directories}` and `services::window::get_runtime_paths`), so it needs
//! no edits to any `mod.rs` — the same pattern used by the sibling
//! `tests/*_properties.rs` files. Each case derives a fresh random
//! application-data root beneath a per-case `tempfile` directory and drives the
//! *real* resolution + creation functions exactly as the startup sequence
//! (task 17.x) and the Window_Manager's runtime-paths report do.
//!
//! Property implemented (design "Testing Strategy"):
//!   - Property 46: Runtime directory resolution
//!
//! *For any* supported platform, resolving the runtime paths SHALL yield six
//! directories (data, media, skill, rule, backup, log) that are pairwise
//! distinct, absolute, located under the platform's per-user application-data
//! root, and present on disk after initialization. The resolved set matches what
//! `ensure_directories` creates and what the Window_Manager's `get_runtime_paths`
//! report exposes, and every created directory is writable.
//!
//! **Validates: Requirements 23.2, 20.9**

use std::fs;

use proptest::prelude::*;
use tempfile::TempDir;

use prompthub_lib::services::data_path::{
    database_path, ensure_directories, resolve_runtime_paths,
};
use prompthub_lib::services::window::get_runtime_paths;

// ---------------------------------------------------------------------------
// Generators
// ---------------------------------------------------------------------------

/// An arbitrary application-data root *suffix*: 0..3 portable path segments that
/// are appended to a per-case temp directory to form the root. Segments use only
/// `[a-z0-9_]` so the root is creatable on every Target_Platform (no reserved
/// characters, no separators) while still exercising roots nested several levels
/// deep — the platform's `app_data_dir()` is likewise a deep, absolute path.
fn base_suffix() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec("[a-z0-9_]{1,8}".prop_map(String::from), 0..3)
}

// ---------------------------------------------------------------------------
// Property 46: Runtime directory resolution
// ---------------------------------------------------------------------------

proptest! {
    // Each case performs filesystem IO (creates the six directories and probes
    // each for writability), so the case count is kept modest.
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// **Property 46: Runtime directory resolution.**
    ///
    /// For *any* application-data root, `resolve_runtime_paths`:
    ///   1. yields six directories (data, media, skill, rule, backup, log) that
    ///      are pairwise distinct;
    ///   2. each of which is absolute and located directly under the given root
    ///      (its parent is the root);
    ///   3. with the SQLite database located under the resolved data directory;
    ///   4. such that the Window_Manager's `get_runtime_paths` report exposes
    ///      exactly that resolved set (the six directories plus the database);
    ///   5. and after `ensure_directories` runs, every one of the six exists on
    ///      disk, is writable, and the root's immediate children are exactly
    ///      those six directories — nothing more, nothing less.
    ///
    /// **Validates: Requirements 23.2, 20.9**
    #[test]
    fn runtime_directory_resolution(suffix in base_suffix()) {
        let tmp = TempDir::new().unwrap();
        let mut base = tmp.path().to_path_buf();
        for segment in &suffix {
            base = base.join(segment);
        }

        let paths = resolve_runtime_paths(&base);

        let dirs = [
            ("data", &paths.data),
            ("media", &paths.media),
            ("skill", &paths.skill),
            ("rule", &paths.rule),
            ("backup", &paths.backup),
            ("log", &paths.log),
        ];

        // (1) The six directories are pairwise distinct.
        for (i, (label_a, a)) in dirs.iter().enumerate() {
            for (label_b, b) in &dirs[i + 1..] {
                prop_assert_ne!(a, b, "{} and {} must be distinct", label_a, label_b);
            }
        }

        // (2) Each directory is absolute and a direct child of the root.
        for (label, dir) in &dirs {
            prop_assert!(dir.is_absolute(), "{} directory must be absolute", label);
            prop_assert!(
                dir.starts_with(&base),
                "{} directory must be located under the root",
                label
            );
            prop_assert_eq!(
                dir.parent(),
                Some(base.as_path()),
                "{} directory must be a direct child of the root",
                label
            );
        }

        // (3) The database lives under the resolved data directory.
        let db = database_path(&paths);
        prop_assert!(
            db.starts_with(&paths.data),
            "database path must be under the data directory"
        );

        // (4) The runtime-paths report exposes exactly the resolved set.
        let report = get_runtime_paths(&paths);
        prop_assert_eq!(&report.data, &paths.data.to_string_lossy().to_string());
        prop_assert_eq!(&report.media, &paths.media.to_string_lossy().to_string());
        prop_assert_eq!(&report.skill, &paths.skill.to_string_lossy().to_string());
        prop_assert_eq!(&report.rule, &paths.rule.to_string_lossy().to_string());
        prop_assert_eq!(&report.backup, &paths.backup.to_string_lossy().to_string());
        prop_assert_eq!(&report.log, &paths.log.to_string_lossy().to_string());
        prop_assert_eq!(&report.database, &db.to_string_lossy().to_string());

        // (5) After initialization every directory exists, is writable, and the
        // root holds exactly the six runtime directories.
        let ensured = ensure_directories(&paths);
        prop_assert!(
            ensured.is_ok(),
            "ensure_directories must succeed for a writable root: {:?}",
            ensured.err()
        );

        for (label, dir) in &dirs {
            prop_assert!(dir.is_dir(), "{} directory should exist after init", label);
            // Confirm writability by writing then removing a probe file.
            let probe = dir.join(".pbt-write-probe");
            let wrote = fs::write(&probe, b"x");
            prop_assert!(
                wrote.is_ok(),
                "{} directory must be writable after init: {:?}",
                label,
                wrote.err()
            );
            let _ = fs::remove_file(&probe);
        }

        let mut children: Vec<String> = fs::read_dir(&base)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        children.sort();
        let expected: Vec<String> = ["backup", "data", "log", "media", "rule", "skill"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        prop_assert_eq!(
            children,
            expected,
            "the root's children must be exactly the six runtime directories"
        );
    }
}
