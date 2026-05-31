//! Property-based tests for the Sync_Service (task 12.2).
//!
//! These run as an **integration test** against the public `prompthub_lib` API
//! (`services::sync::*`, `state::RuntimePaths`, `error::ErrorCode`), so they
//! need no edits to any `mod.rs` — the same pattern used by the sibling
//! `tests/*_properties.rs` files. The Sync_Service rules are written against
//! injected dependencies (explicit `&Path`s, plain config values, a borrowed
//! cancellation flag), so the two properties below exercise the *real* service
//! functions without a live window or a real remote server.
//!
//! Properties implemented (design "Testing Strategy"):
//!   - Property 36: Export archive matches selected scope
//!   - Property 37: Malformed sync configuration issues no request
//!
//! Property 37 is validated at the **config-validation boundary** — the pure
//! validators ([`validate_webdav_config`] / [`validate_s3_config`]) that every
//! outbound entry point calls *first*, before any client is built or any address
//! is contacted. A `VALIDATION` rejection at this single gate is exactly what
//! guarantees zero outbound requests for a malformed config (Req 17.13), so the
//! property needs no network mock.
//!
//! **Validates: Requirements 17.5, 17.13**

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use proptest::prelude::*;
use tempfile::TempDir;

use prompthub_lib::error::ErrorCode;
use prompthub_lib::services::sync::{
    export_zip, validate_s3_config, validate_webdav_config, ExportScope, S3Config, WebDavConfig,
};
use prompthub_lib::state::RuntimePaths;

// ---------------------------------------------------------------------------
// Property 36 helpers: data-set generation and archive inspection
// ---------------------------------------------------------------------------

/// A set of files for one export category: unique file names mapped to their
/// (possibly empty) byte content.
type FileSet = BTreeMap<String, Vec<u8>>;

/// Generates a small set of files with Windows/Unix-safe names (so the archive
/// round-trip is unambiguous) and arbitrary short byte content. The `BTreeMap`
/// key strategy guarantees the file names within a category are unique.
fn file_set() -> impl Strategy<Value = FileSet> {
    prop::collection::btree_map(
        "[a-z][a-z0-9_-]{0,7}".prop_map(|stem| format!("{stem}.bin")),
        prop::collection::vec(any::<u8>(), 0..24),
        0..4,
    )
}

/// Reads a produced ZIP archive into a `{ entry_name -> bytes }` map, so the
/// test can compare the archive's full contents against the expected scope.
fn read_archive(path: &Path) -> BTreeMap<String, Vec<u8>> {
    use std::io::Read;
    let file = std::fs::File::open(path).expect("open archive");
    let mut archive = zip::ZipArchive::new(file).expect("read archive");
    let mut map = BTreeMap::new();
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).expect("archive entry");
        if entry.is_file() {
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf).expect("read entry");
            map.insert(entry.name().to_string(), buf);
        }
    }
    map
}

// ---------------------------------------------------------------------------
// Property 36: Export archive matches selected scope
// ---------------------------------------------------------------------------

proptest! {
    // Each case performs filesystem IO (writes the data set, builds a ZIP), so
    // the case count is kept modest.
    #![proptest_config(ProptestConfig::with_cases(48))]

    /// **Property 36: Export archive matches selected scope.**
    ///
    /// For *any* data set written across the four export categories and *any*
    /// scope selection, the produced ZIP archive contains exactly the files of
    /// the selected categories — no more, no fewer — each under its fixed
    /// category prefix, and the operation returns the absolute path of the
    /// produced archive (17.5).
    ///
    /// Every category's files are written to disk regardless of selection, so
    /// the test also confirms that an *unselected* category is excluded even
    /// when its data exists on disk.
    ///
    /// **Validates: Requirements 17.5**
    #[test]
    fn export_archive_matches_selected_scope(
        data_sel in any::<bool>(),
        media_sel in any::<bool>(),
        skill_sel in any::<bool>(),
        rule_sel in any::<bool>(),
        data_files in file_set(),
        media_files in file_set(),
        skill_files in file_set(),
        rule_files in file_set(),
    ) {
        let tmp = TempDir::new().unwrap();
        let base = tmp.path();
        let paths = RuntimePaths {
            data: base.join("data"),
            media: base.join("media"),
            skill: base.join("skill"),
            rule: base.join("rule"),
            backup: base.join("backup"),
            log: base.join("log"),
        };

        let prefixes = ["data", "media", "skill", "rule"];
        let dirs: [&PathBuf; 4] = [&paths.data, &paths.media, &paths.skill, &paths.rule];
        let files: [&FileSet; 4] = [&data_files, &media_files, &skill_files, &rule_files];
        let sel = [data_sel, media_sel, skill_sel, rule_sel];

        // Materialize the data set on disk for all four categories.
        for i in 0..4 {
            std::fs::create_dir_all(dirs[i]).unwrap();
            for (name, content) in files[i].iter() {
                std::fs::write(dirs[i].join(name), content).unwrap();
            }
        }

        // The archive must contain exactly the files of the selected categories.
        let mut expected: BTreeMap<String, Vec<u8>> = BTreeMap::new();
        for i in 0..4 {
            if sel[i] {
                for (name, content) in files[i].iter() {
                    expected.insert(format!("{}/{}", prefixes[i], name), content.clone());
                }
            }
        }

        let scope = ExportScope {
            data: data_sel,
            media: media_sel,
            skill: skill_sel,
            rule: rule_sel,
        };
        let dest = base.join("export.zip");
        let cancel = AtomicBool::new(false);

        let result = export_zip(&paths, &scope, &dest, &cancel).unwrap();

        prop_assert!(!result.canceled, "export must not report cancellation");
        let file_path = result.file_path.expect("a non-cancelled export returns a path");
        prop_assert!(
            Path::new(&file_path).is_absolute(),
            "returned export path must be absolute, got {file_path}"
        );
        prop_assert!(dest.is_file(), "archive file must exist at the destination");

        let actual = read_archive(&dest);
        prop_assert_eq!(actual, expected);
    }
}

// ---------------------------------------------------------------------------
// Property 37 helpers: malformed configuration generators
// ---------------------------------------------------------------------------

/// Strings that are empty or whitespace-only — rejected by the validators after
/// trimming as a missing required value.
fn whitespace_only() -> impl Strategy<Value = String> {
    prop::sample::select(vec!["", " ", "   ", "\t", "\n", "  \t\n "]).prop_map(str::to_string)
}

/// URLs that parse successfully but carry a scheme other than http/https. The
/// validators accept only http/https, so every value here is rejected with
/// `VALIDATION` regardless of host details.
fn non_http_scheme_url() -> impl Strategy<Value = String> {
    (
        prop::sample::select(vec![
            "ftp", "file", "ws", "wss", "data", "gopher", "ssh", "mailto",
        ]),
        "[a-z]{1,8}",
    )
        .prop_map(|(scheme, host)| format!("{scheme}://{host}"))
}

/// Strings that `Url::parse` rejects outright (no scheme, empty host, etc.), so
/// the validators reject them with `VALIDATION` at the parse step.
fn unparseable_url() -> impl Strategy<Value = String> {
    prop::sample::select(vec![
        "not a url",
        "http://",
        "https://",
        "::::",
        "http//no-colon",
        "%%%",
        "###",
        "://nohost",
    ])
    .prop_map(str::to_string)
}

/// Any URL string that is malformed for the sync validators: empty/whitespace,
/// a non-http(s) scheme, or unparseable.
fn malformed_url() -> impl Strategy<Value = String> {
    prop_oneof![whitespace_only(), non_http_scheme_url(), unparseable_url()]
}

/// A baseline S3 configuration that passes validation; individual fields are
/// then mutated into a malformed state by [`apply_s3_malformation`].
fn valid_s3_config() -> S3Config {
    S3Config {
        endpoint: "https://s3.example.com".into(),
        region: "us-east-1".into(),
        bucket: "my-bucket".into(),
        access_key_id: "AKIAEXAMPLE".into(),
        secret_access_key: "secret".into(),
    }
}

/// The ways an S3 configuration can be made malformed: blanking any one of the
/// five required fields, or supplying a non-http(s)/unparseable endpoint.
#[derive(Clone, Debug)]
enum S3Malformation {
    /// Blank the field at the given index (0..5) with the supplied blank string.
    BlankField(usize, String),
    /// Replace the endpoint with a non-http(s)-scheme or unparseable URL.
    BadEndpoint(String),
}

fn s3_malformation() -> impl Strategy<Value = S3Malformation> {
    prop_oneof![
        (0usize..5, whitespace_only())
            .prop_map(|(idx, blank)| S3Malformation::BlankField(idx, blank)),
        prop_oneof![non_http_scheme_url(), unparseable_url()].prop_map(S3Malformation::BadEndpoint),
    ]
}

/// Applies a malformation to a baseline-valid config, yielding a config the
/// validator must reject.
fn apply_s3_malformation(mut config: S3Config, malformation: S3Malformation) -> S3Config {
    match malformation {
        S3Malformation::BlankField(idx, blank) => match idx {
            0 => config.endpoint = blank,
            1 => config.region = blank,
            2 => config.bucket = blank,
            3 => config.access_key_id = blank,
            _ => config.secret_access_key = blank,
        },
        S3Malformation::BadEndpoint(url) => config.endpoint = url,
    }
    config
}

// ---------------------------------------------------------------------------
// Property 37: Malformed sync configuration issues no request
// ---------------------------------------------------------------------------

proptest! {
    // Pure validation only (no IO, no network), so a high case count is cheap.
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// **Property 37 (WebDAV): Malformed configuration issues no request.**
    ///
    /// For *any* malformed WebDAV URL, the pre-request validation gate
    /// ([`validate_webdav_config`]) rejects the configuration with a
    /// `VALIDATION` error. Because every WebDAV transport entry point calls this
    /// pure validator before constructing a client or contacting any address, a
    /// rejection here means zero outbound requests are issued (17.13).
    ///
    /// **Validates: Requirements 17.13**
    #[test]
    fn malformed_webdav_config_rejected_with_validation(
        url in malformed_url(),
        username in "[a-z]{0,8}",
        password in "[a-z]{0,8}",
    ) {
        let config = WebDavConfig { url, username, password };
        let err = validate_webdav_config(&config).unwrap_err();
        prop_assert_eq!(err.code, ErrorCode::Validation);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// **Property 37 (S3): Malformed configuration issues no request.**
    ///
    /// For *any* malformed S3 configuration (a blank required field or a
    /// non-http(s)/unparseable endpoint), the pre-request validation gate
    /// ([`validate_s3_config`]) rejects it with a `VALIDATION` error. As that
    /// validator is the single gate every S3 transport entry point calls before
    /// signing or issuing a request, a rejection here means zero outbound
    /// requests are issued (17.13).
    ///
    /// **Validates: Requirements 17.13**
    #[test]
    fn malformed_s3_config_rejected_with_validation(malformation in s3_malformation()) {
        let config = apply_s3_malformation(valid_s3_config(), malformation);
        let err = validate_s3_config(&config).unwrap_err();
        prop_assert_eq!(err.code, ErrorCode::Validation);
    }
}
