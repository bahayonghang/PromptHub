//! Property-based test for the Data_Path_Manager (task 14.2).
//!
//! Runs as an **integration test** against the public `prompthub_lib` API
//! (`services::data_path::preview_change`), so it needs no edits to any
//! `mod.rs` — the same pattern used by the sibling `tests/*_properties.rs`
//! files. Each case builds a fresh random directory tree under a per-case
//! `tempfile` root and drives the *real* service function exactly as the
//! Command_Layer (task 17.1) will.
//!
//! Property implemented (design "Testing Strategy"):
//!   - Property 39: Data-path preview is read-only and well-formed
//!
//! *For any* target directory state — non-existent, empty, holding only
//! non-marker files, holding PromptHub data markers, or being the active data
//! directory itself — `preview_change` reports `exists`, `has_prompt_hub_data`,
//! `is_current`, and a `recommended_action` in {migrate, switch} consistent with
//! that state, and leaves both the target and the active data directory (indeed
//! the whole tree) byte-for-byte unmodified.
//!
//! **Validates: Requirements 19.4**

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use proptest::prelude::*;
use tempfile::TempDir;

use prompthub_lib::services::data_path::preview_change;

// ---------------------------------------------------------------------------
// Known PromptHub data markers
// ---------------------------------------------------------------------------

/// A representative set of the marker entries whose presence makes a directory
/// "hold PromptHub data" (mirrors `data_path::DATA_MARKERS`, which is private).
/// Seeding any one of these makes `has_prompt_hub_data` true; the test never
/// relies on the full list, only that these names are genuine markers.
const KNOWN_MARKERS: &[&str] = &[
    "data",
    "config",
    "backups",
    "logs",
    "images",
    "videos",
    "skills",
    "rules",
    "shortcuts.json",
    "shortcut-mode.json",
    "prompthub.db",
];

// ---------------------------------------------------------------------------
// Generators
// ---------------------------------------------------------------------------

/// A single "noise" file: a relative path built entirely from `noise_`-prefixed
/// segments (so it can never collide with a marker name) plus arbitrary content.
#[derive(Debug, Clone)]
struct NoiseFile {
    /// Relative path under the seeded directory, e.g. `noise_ab/noise_c.dat`.
    rel: String,
    content: Vec<u8>,
}

/// A relative file path of 1..3 `noise_` directory segments with a `.dat` leaf.
///
/// Every intermediate segment is a directory named `noise_*` (never carrying a
/// `.dat` suffix) and every leaf file ends in `.dat`, so seeding can never hit a
/// file/directory type clash, and no generated name equals a PromptHub marker.
fn noise_path() -> impl Strategy<Value = String> {
    prop::collection::vec("noise_[a-z]{1,4}".prop_map(String::from), 1..3)
        .prop_map(|segs| format!("{}.dat", segs.join("/")))
}

fn noise_file() -> impl Strategy<Value = NoiseFile> {
    (noise_path(), prop::collection::vec(any::<u8>(), 0..16))
        .prop_map(|(rel, content)| NoiseFile { rel, content })
}

/// A small random tree of non-marker files (possibly empty).
fn noise_tree() -> impl Strategy<Value = Vec<NoiseFile>> {
    prop::collection::vec(noise_file(), 0..4)
}

/// A non-empty subset of the known PromptHub markers, preserving list order.
fn markers_subset() -> impl Strategy<Value = Vec<String>> {
    let all: Vec<String> = KNOWN_MARKERS.iter().map(|s| s.to_string()).collect();
    prop::sample::subsequence(all, 1..=KNOWN_MARKERS.len())
}

/// The states a preview target can be in.
#[derive(Debug, Clone)]
enum TargetSpec {
    /// The target path does not exist on disk.
    NonExistent,
    /// The target exists but is an empty directory.
    Empty,
    /// The target exists and holds only non-marker files.
    NonMarker(Vec<NoiseFile>),
    /// The target exists and holds PromptHub data markers (plus optional noise).
    WithData(Vec<String>, Vec<NoiseFile>),
    /// The target *is* the active data directory.
    Active,
}

fn target_spec() -> impl Strategy<Value = TargetSpec> {
    prop_oneof![
        Just(TargetSpec::NonExistent),
        Just(TargetSpec::Empty),
        noise_tree().prop_map(TargetSpec::NonMarker),
        (markers_subset(), noise_tree()).prop_map(|(m, n)| TargetSpec::WithData(m, n)),
        Just(TargetSpec::Active),
    ]
}

/// A full scenario: how the active directory is seeded and what the target is.
#[derive(Debug, Clone)]
struct Scenario {
    /// Markers seeded into the active data directory (always non-empty, so the
    /// active directory genuinely holds PromptHub data).
    active_markers: Vec<String>,
    /// Extra non-marker files seeded into the active data directory.
    active_noise: Vec<NoiseFile>,
    target: TargetSpec,
}

fn scenario() -> impl Strategy<Value = Scenario> {
    (markers_subset(), noise_tree(), target_spec()).prop_map(
        |(active_markers, active_noise, target)| Scenario {
            active_markers,
            active_noise,
            target,
        },
    )
}

// ---------------------------------------------------------------------------
// Seeding + filesystem snapshot helpers
// ---------------------------------------------------------------------------

/// Seeds the named markers into `dir`. Names with an extension become files;
/// the rest become directories — either form is recognized as a marker.
fn seed_markers(dir: &Path, markers: &[String]) {
    fs::create_dir_all(dir).unwrap();
    for name in markers {
        let path = dir.join(name);
        if name.contains('.') {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(&path, b"x").unwrap();
        } else {
            fs::create_dir_all(&path).unwrap();
        }
    }
}

/// Seeds a tree of non-marker files into `dir`.
fn seed_noise(dir: &Path, tree: &[NoiseFile]) {
    fs::create_dir_all(dir).unwrap();
    for file in tree {
        let path = dir.join(&file.rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&path, &file.content).unwrap();
    }
}

/// A captured filesystem node: a directory, or a file with its exact bytes.
#[derive(Debug, PartialEq, Eq)]
enum Node {
    Dir,
    File(Vec<u8>),
}

/// Recursively captures every entry under `root` as a `{ relative_path -> Node }`
/// map, recording file contents so the snapshot detects any modification — a new
/// or removed entry, or a single changed byte.
fn snapshot(root: &Path) -> BTreeMap<String, Node> {
    fn walk(base: &Path, dir: &Path, map: &mut BTreeMap<String, Node>) {
        let entries = match fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(_) => return,
        };
        for entry in entries {
            let entry = entry.unwrap();
            let path = entry.path();
            let rel = path
                .strip_prefix(base)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            let file_type = entry.file_type().unwrap();
            if file_type.is_dir() {
                map.insert(rel, Node::Dir);
                walk(base, &path, map);
            } else {
                map.insert(rel, Node::File(fs::read(&path).unwrap()));
            }
        }
    }
    let mut map = BTreeMap::new();
    walk(root, root, &mut map);
    map
}

// ---------------------------------------------------------------------------
// Property 39: Data-path preview is read-only and well-formed
// ---------------------------------------------------------------------------

proptest! {
    // Each case performs filesystem IO (builds a tree, snapshots it twice), so
    // the case count is kept modest.
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// **Property 39: Data-path preview is read-only and well-formed.**
    ///
    /// For *any* target directory state, `preview_change`:
    ///   1. leaves the entire filesystem tree (target + active directory)
    ///      byte-for-byte unchanged — in particular a non-existent target is not
    ///      created (read-only);
    ///   2. reports `exists` matching the target's real existence;
    ///   3. reports `has_prompt_hub_data` exactly when markers were found
    ///      (`has_prompt_hub_data == !markers.is_empty()`) and matching whether
    ///      the target was seeded with PromptHub data;
    ///   4. reports `is_current` true exactly when the target resolves to the
    ///      active data directory;
    ///   5. returns `recommended_action` that is exactly `switch` when the target
    ///      holds PromptHub data and exactly `migrate` otherwise;
    ///   6. echoes the previewed `target_path`.
    ///
    /// **Validates: Requirements 19.4**
    #[test]
    fn preview_is_read_only_and_well_formed(scenario in scenario()) {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        // Seed the active data directory — always holds PromptHub data.
        let active = root.join("active");
        seed_markers(&active, &scenario.active_markers);
        seed_noise(&active, &scenario.active_noise);

        // Materialize the target and derive the expected, independently-known
        // observations for it.
        let (target, expected_exists, expected_has_data, expected_is_current) =
            match &scenario.target {
                TargetSpec::NonExistent => {
                    (root.join("target_missing"), false, false, false)
                }
                TargetSpec::Empty => {
                    let target = root.join("target");
                    fs::create_dir_all(&target).unwrap();
                    (target, true, false, false)
                }
                TargetSpec::NonMarker(noise) => {
                    let target = root.join("target");
                    seed_noise(&target, noise);
                    (target, true, false, false)
                }
                TargetSpec::WithData(markers, noise) => {
                    let target = root.join("target");
                    seed_markers(&target, markers);
                    seed_noise(&target, noise);
                    (target, true, true, false)
                }
                TargetSpec::Active => {
                    // Target is the active directory itself: it exists, holds
                    // data, and is the current directory.
                    (active.clone(), true, true, true)
                }
            };

        // Snapshot the whole tree, run the preview, snapshot again.
        let before = snapshot(root);
        let result = preview_change(&active, &target).unwrap();
        let after = snapshot(root);

        // (1) Read-only: nothing on disk changed.
        prop_assert!(
            before == after,
            "preview_change must not modify the filesystem"
        );

        // (6) The previewed path is echoed back verbatim.
        prop_assert_eq!(&result.target_path, &target.to_string_lossy().to_string());

        // (2) Existence is reported correctly.
        prop_assert_eq!(result.exists, expected_exists);

        // (3) has_prompt_hub_data matches both the seeded state and the returned
        // marker list.
        prop_assert_eq!(result.has_prompt_hub_data, expected_has_data);
        prop_assert_eq!(result.has_prompt_hub_data, !result.markers.is_empty());

        // (4) is_current is true exactly when the target is the active directory.
        prop_assert_eq!(result.is_current, expected_is_current);

        // (5) recommended_action is well-formed and consistent with the data
        // presence: exactly "switch" when data is present, else "migrate".
        prop_assert!(
            result.recommended_action == "switch" || result.recommended_action == "migrate",
            "recommended_action `{}` must be exactly switch or migrate",
            result.recommended_action
        );
        let expected_action = if result.has_prompt_hub_data { "switch" } else { "migrate" };
        prop_assert_eq!(&result.recommended_action, expected_action);
    }
}
