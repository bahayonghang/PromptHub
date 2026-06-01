//! Property-based test for the Media_Service (task 13.2).
//!
//! Runs as an **integration test** against the public `prompthub_lib` API
//! (`services::media::*`, `error::*`), so it needs no edits to any `mod.rs`.
//! Each case uses a fresh per-case `tempfile` directory and drives the service
//! through its public functions exactly as the Command_Layer (task 17.1) will.
//!
//! Property implemented (design "Testing Strategy"):
//!   - Property 38: Media format and decoding validation
//!
//! *For any* image save input, the operation stores a file only when the data
//! decodes to one of JPEG/PNG/GIF/WebP and (for file-path saves) the path was
//! selected in the current session; otherwise — non-session paths, unsupported
//! formats, and undecodable base64 — the operation is rejected with a VALIDATION
//! error and writes no file.
//!
//! **Validates: Requirements 18.8**

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine as _;
use proptest::prelude::*;
use tempfile::TempDir;

use prompthub_lib::error::ErrorCode;
use prompthub_lib::services::media::{
    self, detect_image_format, images_dir, save_image_base64, save_image_buffer, save_images,
    IMAGE_EXTENSIONS,
};

// ---------------------------------------------------------------------------
// Generators
// ---------------------------------------------------------------------------

/// A byte payload whose leading magic bytes make it one of the four supported
/// image formats (JPEG, PNG, GIF, WebP), followed by an arbitrary tail. By
/// construction [`detect_image_format`] returns `Some` for every value.
fn supported_image() -> impl Strategy<Value = Vec<u8>> {
    let tail = || proptest::collection::vec(any::<u8>(), 0..32);
    prop_oneof![
        // JPEG — FF D8 FF + tail.
        tail().prop_map(|t| {
            let mut v = vec![0xFF, 0xD8, 0xFF];
            v.extend(t);
            v
        }),
        // PNG — 8-byte signature + tail.
        tail().prop_map(|t| {
            let mut v = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
            v.extend(t);
            v
        }),
        // GIF — "GIF87a" / "GIF89a" + tail.
        (any::<bool>(), tail()).prop_map(|(is89, t)| {
            let mut v = if is89 {
                b"GIF89a".to_vec()
            } else {
                b"GIF87a".to_vec()
            };
            v.extend(t);
            v
        }),
        // WebP — "RIFF" + 4-byte size + "WEBP" + tail.
        (proptest::array::uniform4(any::<u8>()), tail()).prop_map(|(size, t)| {
            let mut v = b"RIFF".to_vec();
            v.extend_from_slice(&size);
            v.extend_from_slice(b"WEBP");
            v.extend(t);
            v
        }),
    ]
}

/// The canonical extension a successfully stored image is expected to carry,
/// derived from the (guaranteed-present) format of a [`supported_image`] value.
fn expected_ext(bytes: &[u8]) -> &'static str {
    detect_image_format(bytes)
        .expect("supported_image() must produce a detectable format")
        .extension()
}

// ---------------------------------------------------------------------------
// Filesystem assertions
// ---------------------------------------------------------------------------

/// True when `dir` holds no files at all — the "wrote nothing" guarantee for a
/// rejected save (covers files of any extension, not just image ones).
fn nothing_written(dir: &Path) -> bool {
    match std::fs::read_dir(dir) {
        Ok(mut entries) => entries.next().is_none(),
        // A directory that was never created also means nothing was written.
        Err(_) => true,
    }
}

// ---------------------------------------------------------------------------
// Property 38a — buffer save accepts supported images and round-trips
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// A buffer whose magic bytes are a supported image is stored under a name
    /// carrying the detected extension, and the stored bytes equal the input.
    #[test]
    fn buffer_save_accepts_supported_image(bytes in supported_image()) {
        let base = TempDir::new().unwrap();
        let dir = images_dir(base.path());

        let name = save_image_buffer(&dir, &bytes)
            .expect("a supported image must be accepted");

        let suffix = format!(".{}", expected_ext(&bytes));
        prop_assert!(
            name.ends_with(&suffix),
            "name `{name}` should carry the detected extension"
        );
        prop_assert_eq!(media::read(&dir, &name).unwrap(), bytes);
        // Exactly one file was written.
        prop_assert_eq!(media::list(&dir, IMAGE_EXTENSIONS).unwrap(), vec![name]);
    }
}

// ---------------------------------------------------------------------------
// Property 38b — buffer save rejects non-image bytes, writing nothing
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// Arbitrary byte data that is not a supported image is rejected with a
    /// VALIDATION error and no file is written.
    #[test]
    fn buffer_save_rejects_non_image(bytes in proptest::collection::vec(any::<u8>(), 0..64)) {
        // Only consider inputs that are genuinely not a supported image.
        prop_assume!(detect_image_format(&bytes).is_none());

        let base = TempDir::new().unwrap();
        let dir = images_dir(base.path());

        let err = save_image_buffer(&dir, &bytes)
            .expect_err("non-image bytes must be rejected");
        prop_assert_eq!(err.code, ErrorCode::Validation);
        prop_assert!(nothing_written(&dir), "rejected save must write no file");
    }
}

// ---------------------------------------------------------------------------
// Property 38c — base64 save: decode + format gate
// ---------------------------------------------------------------------------

/// The three input shapes a base64 image save can take.
#[derive(Debug, Clone)]
enum B64Case {
    /// Valid base64 of a supported image — must be accepted.
    ValidImage(Vec<u8>),
    /// Valid base64 of bytes that are not a supported image — must be rejected.
    DecodableNonImage(Vec<u8>),
    /// A string that cannot be base64-decoded — must be rejected.
    Undecodable(String),
}

fn b64_case() -> impl Strategy<Value = B64Case> {
    prop_oneof![
        supported_image().prop_map(B64Case::ValidImage),
        proptest::collection::vec(any::<u8>(), 0..48).prop_map(B64Case::DecodableNonImage),
        // None of these characters are in the standard base64 alphabet, none are
        // whitespace, and they cannot form a `base64,` data-URL prefix, so the
        // payload is guaranteed to fail decoding.
        "[!@#%&*();:,.?]{1,40}".prop_map(B64Case::Undecodable),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// A base64 image save succeeds only when the payload decodes to a supported
    /// image; an undecodable payload or a decodable non-image is rejected with a
    /// VALIDATION error and writes no file.
    #[test]
    fn base64_save_enforces_decode_and_format(case in b64_case()) {
        let base = TempDir::new().unwrap();
        let dir = images_dir(base.path());

        match case {
            B64Case::ValidImage(bytes) => {
                let data = BASE64.encode(&bytes);
                let name = save_image_base64(&dir, &data)
                    .expect("base64 of a supported image must be accepted");
                let suffix = format!(".{}", expected_ext(&bytes));
                prop_assert!(name.ends_with(&suffix));
                prop_assert_eq!(media::read(&dir, &name).unwrap(), bytes);
            }
            B64Case::DecodableNonImage(bytes) => {
                prop_assume!(detect_image_format(&bytes).is_none());
                let data = BASE64.encode(&bytes);
                let err = save_image_base64(&dir, &data)
                    .expect_err("decodable non-image must be rejected");
                prop_assert_eq!(err.code, ErrorCode::Validation);
                prop_assert!(nothing_written(&dir));
            }
            B64Case::Undecodable(data) => {
                let err = save_image_base64(&dir, &data)
                    .expect_err("undecodable base64 must be rejected");
                prop_assert_eq!(err.code, ErrorCode::Validation);
                prop_assert!(nothing_written(&dir));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Property 38d — file-path save: session selection + format gate
// ---------------------------------------------------------------------------

/// The three input shapes a session-path image save can take.
#[derive(Debug, Clone)]
#[allow(clippy::enum_variant_names)]
enum PathCase {
    /// A supported image whose source path was selected this session.
    SessionImage(Vec<u8>),
    /// A supported image whose source path was NOT selected this session.
    NonSessionImage(Vec<u8>),
    /// A non-image source whose path was selected this session.
    SessionNonImage(Vec<u8>),
}

fn path_case() -> impl Strategy<Value = PathCase> {
    prop_oneof![
        supported_image().prop_map(PathCase::SessionImage),
        supported_image().prop_map(PathCase::NonSessionImage),
        proptest::collection::vec(any::<u8>(), 0..48).prop_map(PathCase::SessionNonImage),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    /// A file-path image save stores the file only when the path was selected
    /// this session and its content is a supported image; a non-session path or
    /// a non-image source is rejected with VALIDATION and writes no file.
    #[test]
    fn path_save_enforces_session_and_format(case in path_case()) {
        let base = TempDir::new().unwrap();
        let dir = images_dir(base.path());
        let src = base.path().join("source.bin");

        match case {
            PathCase::SessionImage(bytes) => {
                std::fs::write(&src, &bytes).unwrap();
                let allowed: HashSet<PathBuf> = [src.clone()].into_iter().collect();

                let names = save_images(&dir, &allowed, &[src])
                    .expect("a session-selected supported image must be accepted");
                prop_assert_eq!(names.len(), 1);
                let suffix = format!(".{}", expected_ext(&bytes));
                prop_assert!(names[0].ends_with(&suffix));
                prop_assert_eq!(media::read(&dir, &names[0]).unwrap(), bytes);
            }
            PathCase::NonSessionImage(bytes) => {
                std::fs::write(&src, &bytes).unwrap();
                // Empty allow-set: the path was never selected this session.
                let allowed: HashSet<PathBuf> = HashSet::new();

                let err = save_images(&dir, &allowed, &[src])
                    .expect_err("a non-session path must be rejected");
                prop_assert_eq!(err.code, ErrorCode::Validation);
                prop_assert!(nothing_written(&dir));
            }
            PathCase::SessionNonImage(bytes) => {
                prop_assume!(detect_image_format(&bytes).is_none());
                std::fs::write(&src, &bytes).unwrap();
                let allowed: HashSet<PathBuf> = [src.clone()].into_iter().collect();

                let err = save_images(&dir, &allowed, &[src])
                    .expect_err("a non-image source must be rejected");
                prop_assert_eq!(err.code, ErrorCode::Validation);
                prop_assert!(nothing_written(&dir));
            }
        }
    }
}
