//! Property-based tests for the Skill_Service (task 8.6).
//!
//! These run as an **integration test** against the public `prompthub_lib` API
//! (`services::skill_md`, `skill_local`, `skill_platform`, `skill_safety`,
//! `storage`, and `error`). Each case uses a fresh in-memory database and/or a
//! per-case `tempfile` tree, driving the same public service functions the
//! Command_Layer will call.
//!
//! Properties implemented (design "Testing Strategy"):
//!   - Property 25: SKILL.md round-trip
//!   - Property 26: SKILL.md rejection of malformed or incomplete documents
//!   - Property 27: Skill import sanitization
//!   - Property 28: SSRF host classification
//!   - Property 29: Skill path confinement
//!
//! **Validates: Requirements 10.1–10.6, 11.8, 12.8, 13.5**

use std::fs;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::path::Path;

use proptest::prelude::*;
use serde_json::{json, Value as JsonValue};
use tempfile::TempDir;

use prompthub_lib::error::ErrorCode;
use prompthub_lib::models::ParsedSkillMd;
use prompthub_lib::services::skill;
use prompthub_lib::services::skill_local;
use prompthub_lib::services::skill_md;
use prompthub_lib::services::skill_platform::{self, Platform, SkillFile};
use prompthub_lib::services::skill_safety::{self, fetch_content};
use prompthub_lib::storage::{create_memory_pool, init_schema, DbPool};

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Builds an in-memory pool with the schema initialized.
fn schema_pool() -> DbPool {
    let pool = create_memory_pool().expect("memory pool");
    init_schema(&pool.get().expect("conn")).expect("schema");
    pool
}

/// True when `dir` either does not exist or has no entries.
fn dir_empty_or_missing(dir: &Path) -> bool {
    match fs::read_dir(dir) {
        Ok(mut entries) => entries.next().is_none(),
        Err(_) => true,
    }
}

/// A single resolved platform whose skills dir lives under `base`.
fn one_platform(base: &Path, id: &str) -> Vec<Platform> {
    let root_dir = base.join(format!(".{id}"));
    let skills_dir = root_dir.join("skills");
    vec![Platform {
        id: id.to_string(),
        name: id.to_string(),
        is_custom: false,
        root_dir,
        skills_dir,
    }]
}

fn skill_file(path: &str, content: &str) -> SkillFile {
    SkillFile {
        relative_path: path.to_string(),
        content: content.to_string(),
    }
}

fn no_active_content(s: &str) -> bool {
    let lower = s.to_ascii_lowercase();
    !lower.contains("<script")
        && !lower.contains("onclick")
        && !lower.contains("onload")
        && !lower.contains("onerror")
        && !lower.contains("onmouseover")
}

// ---------------------------------------------------------------------------
// Strategies: SKILL.md parsing / serialization
// ---------------------------------------------------------------------------

fn yaml_scalar() -> impl Strategy<Value = JsonValue> {
    prop_oneof![
        proptest::string::string_regex("[a-zA-Z0-9 _.,:;!/?好世界-]{0,40}")
            .unwrap()
            .prop_map(JsonValue::String),
        any::<bool>().prop_map(JsonValue::Bool),
        (-10_000i64..=10_000i64).prop_map(|n| JsonValue::Number(n.into())),
    ]
}

fn yaml_value() -> impl Strategy<Value = JsonValue> {
    prop_oneof![
        yaml_scalar(),
        prop::collection::vec(
            proptest::string::string_regex("[a-zA-Z0-9 _.-]{0,20}").unwrap(),
            0..=5,
        )
        .prop_map(|xs| JsonValue::Array(xs.into_iter().map(JsonValue::String).collect())),
    ]
}

fn parsed_skill_md() -> impl Strategy<Value = ParsedSkillMd> {
    let optional_pairs = prop::collection::btree_map(
        prop_oneof![
            Just("author".to_string()),
            Just("version".to_string()),
            Just("tags".to_string()),
            Just("enabled".to_string()),
            Just("count".to_string()),
            Just("category".to_string()),
        ],
        yaml_value(),
        0..=6,
    );

    (
        proptest::string::string_regex("[a-zA-Z0-9 好世界_-]{1,32}").unwrap(),
        proptest::string::string_regex("[a-zA-Z0-9 好世界_.,;:!?/-]{1,80}").unwrap(),
        optional_pairs,
        prop_oneof![
            Just(String::new()),
            any::<String>(),
            proptest::string::string_regex("[#*`> _a-zA-Z0-9好世界\n\r\t./:-]{0,300}").unwrap(),
        ],
    )
        .prop_map(|(name, description, mut frontmatter, body)| {
            frontmatter.insert("name".to_string(), JsonValue::String(name));
            frontmatter.insert("description".to_string(), JsonValue::String(description));
            ParsedSkillMd { frontmatter, body }
        })
}

#[derive(Debug, Clone)]
enum BadSkillMd {
    MissingOpening(String),
    MissingClosing(String),
    MalformedYaml,
    MissingName,
    MissingDescription,
    EmptyName(String),
    EmptyDescription(String),
    NonStringName,
    NonMappingFrontmatter,
}

fn bad_skill_md() -> impl Strategy<Value = BadSkillMd> {
    prop_oneof![
        any::<String>().prop_map(BadSkillMd::MissingOpening),
        any::<String>().prop_map(BadSkillMd::MissingClosing),
        Just(BadSkillMd::MalformedYaml),
        Just(BadSkillMd::MissingName),
        Just(BadSkillMd::MissingDescription),
        proptest::string::string_regex("[ \t\r\n]{0,8}")
            .unwrap()
            .prop_map(BadSkillMd::EmptyName),
        proptest::string::string_regex("[ \t\r\n]{0,8}")
            .unwrap()
            .prop_map(BadSkillMd::EmptyDescription),
        Just(BadSkillMd::NonStringName),
        Just(BadSkillMd::NonMappingFrontmatter),
    ]
}

fn bad_skill_doc(case: BadSkillMd) -> String {
    match case {
        BadSkillMd::MissingOpening(body) => {
            format!("name: Skill\ndescription: desc\n---\n{body}")
        }
        BadSkillMd::MissingClosing(body) => {
            format!("---\nname: Skill\ndescription: desc\n{body}")
        }
        BadSkillMd::MalformedYaml => {
            "---\nname: Skill\ndescription: desc\ntags: [unterminated\n---\nbody".to_string()
        }
        BadSkillMd::MissingName => "---\ndescription: desc\n---\nbody".to_string(),
        BadSkillMd::MissingDescription => "---\nname: Skill\n---\nbody".to_string(),
        BadSkillMd::EmptyName(name) => {
            format!("---\nname: {:?}\ndescription: desc\n---\nbody", name)
        }
        BadSkillMd::EmptyDescription(description) => {
            format!(
                "---\nname: Skill\ndescription: {:?}\n---\nbody",
                description
            )
        }
        BadSkillMd::NonStringName => "---\nname: 42\ndescription: desc\n---\nbody".to_string(),
        BadSkillMd::NonMappingFrontmatter => "---\n- name\n- desc\n---\nbody".to_string(),
    }
}

// ---------------------------------------------------------------------------
// Property 25: SKILL.md round-trip
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    /// Feature: tauri-rewrite, Property 25: SKILL.md round-trip.
    ///
    /// Serializing any valid parsed SKILL.md and parsing it back preserves all
    /// frontmatter pairs (order-insensitive via BTreeMap) and the Markdown body
    /// exactly. This is the acceptance contract for Requirements 10.1–10.3.
    #[test]
    fn skill_md_round_trip_preserves_metadata_and_body(original in parsed_skill_md()) {
        let serialized = skill_md::serialize_md(&original).unwrap();
        let reparsed = skill_md::parse_md(&serialized).unwrap();
        prop_assert_eq!(reparsed, original, "serialized document:\\n{}", serialized);
    }
}

// ---------------------------------------------------------------------------
// Property 26: SKILL.md rejection of malformed/incomplete documents
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    /// Feature: tauri-rewrite, Property 26: SKILL.md rejection of malformed or
    /// incomplete documents.
    ///
    /// Documents with missing delimiters, malformed/non-mapping YAML, missing
    /// `name`/`description`, empty required fields, or non-string required
    /// fields are rejected with `VALIDATION` and do not produce a parsed value.
    ///
    /// **Validates: Requirement 10.4**
    #[test]
    fn skill_md_rejects_malformed_or_incomplete_documents(case in bad_skill_md()) {
        let doc = bad_skill_doc(case);
        let err = skill_md::parse_md(&doc).expect_err("invalid SKILL.md must be rejected");
        prop_assert_eq!(err.code, ErrorCode::Validation);
    }
}

// ---------------------------------------------------------------------------
// Property 27: Skill import sanitization
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
enum ImportCase {
    Sanitizable {
        name_suffix: String,
        description: String,
        body: String,
        tag: String,
    },
    MalformedJson(String),
    WrongShape(String),
    MissingName,
    EmptyName(String),
    UnsanitizableScript,
}

fn import_case() -> impl Strategy<Value = ImportCase> {
    prop_oneof![
        (
            proptest::string::string_regex("[a-zA-Z0-9_-]{1,20}").unwrap(),
            proptest::string::string_regex("[a-zA-Z0-9 _.-]{0,40}").unwrap(),
            proptest::string::string_regex("[a-zA-Z0-9 _./#*-]{0,80}").unwrap(),
            proptest::string::string_regex("[a-zA-Z0-9_-]{0,16}").unwrap(),
        )
            .prop_map(
                |(name_suffix, description, body, tag)| ImportCase::Sanitizable {
                    name_suffix,
                    description,
                    body,
                    tag,
                }
            ),
        proptest::string::string_regex("[!@#%&*();:,.?]{1,40}")
            .unwrap()
            .prop_map(ImportCase::MalformedJson),
        prop_oneof![
            Just("[]".to_string()),
            Just("42".to_string()),
            Just("null".to_string())
        ]
        .prop_map(ImportCase::WrongShape),
        Just(ImportCase::MissingName),
        proptest::string::string_regex("[ \t\r\n]{0,8}")
            .unwrap()
            .prop_map(ImportCase::EmptyName),
        Just(ImportCase::UnsanitizableScript),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    /// Feature: tauri-rewrite, Property 27: Skill import sanitization.
    ///
    /// Malformed JSON, non-conforming import shapes, missing/empty names, and
    /// content that cannot be fully sanitized are rejected without persistence;
    /// sanitizable `<script>` blocks and `on*` handlers are removed before the
    /// skill is persisted.
    ///
    /// **Validates: Requirements 10.5, 10.6**
    #[test]
    fn skill_import_sanitizes_or_rejects_without_partial_persistence(case in import_case()) {
        let pool = schema_pool();
        let conn = pool.get().unwrap();

        match case {
            ImportCase::Sanitizable { name_suffix, description, body, tag } => {
                let raw = json!({
                    "name": format!("Skill-{name_suffix}"),
                    "description": format!("before <script>alert(1)</script> {description} after"),
                    "content": format!("<div onclick=\"evil()\" onload='go()'>{body}</div><script>bad()</script>"),
                    "tags": [format!("tag-{tag}"), "x onerror=bad()"],
                    "author": "safe onmouseover='bad()' author"
                });
                let created = skill_md::import(&conn, &raw.to_string()).unwrap();

                prop_assert_eq!(skill::list(&conn).unwrap().len(), 1);
                prop_assert!(no_active_content(&created.name));
                if let Some(description) = &created.description {
                    prop_assert!(no_active_content(description), "description: {description}");
                    prop_assert!(description.contains("before"));
                    prop_assert!(description.contains("after"));
                }
                if let Some(content) = &created.content {
                    prop_assert!(no_active_content(content), "content: {content}");
                    prop_assert!(content.contains(&body));
                }
                for tag in &created.tags {
                    prop_assert!(no_active_content(tag), "tag: {tag}");
                }
                if let Some(author) = &created.author {
                    prop_assert!(no_active_content(author), "author: {author}");
                }
            }
            ImportCase::MalformedJson(payload) => {
                let err = skill_md::import(&conn, &payload).unwrap_err();
                prop_assert_eq!(err.code, ErrorCode::Validation);
                prop_assert!(skill::list(&conn).unwrap().is_empty());
            }
            ImportCase::WrongShape(payload) => {
                let err = skill_md::import(&conn, &payload).unwrap_err();
                prop_assert_eq!(err.code, ErrorCode::Validation);
                prop_assert!(skill::list(&conn).unwrap().is_empty());
            }
            ImportCase::MissingName => {
                let err = skill_md::import(&conn, r#"{"description":"x"}"#).unwrap_err();
                prop_assert_eq!(err.code, ErrorCode::Validation);
                prop_assert!(skill::list(&conn).unwrap().is_empty());
            }
            ImportCase::EmptyName(name) => {
                let raw = json!({ "name": name, "description": "x" });
                let err = skill_md::import(&conn, &raw.to_string()).unwrap_err();
                prop_assert_eq!(err.code, ErrorCode::Validation);
                prop_assert!(skill::list(&conn).unwrap().is_empty());
            }
            ImportCase::UnsanitizableScript => {
                let raw = json!({
                    "name": "Bad",
                    "description": "ok",
                    "content": "prefix <script>evil() no closing tag"
                });
                let err = skill_md::import(&conn, &raw.to_string()).unwrap_err();
                prop_assert_eq!(err.code, ErrorCode::Validation);
                prop_assert!(skill::list(&conn).unwrap().is_empty());
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Property 28: SSRF host classification
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
enum IpCase {
    V4([u8; 4], bool),
    V6([u16; 8], bool),
}

fn ip_case() -> impl Strategy<Value = IpCase> {
    prop_oneof![
        // IPv4 rejected ranges: loopback, link-local, private, CGNAT,
        // documentation/reserved/multicast/broadcast.
        (0u8..=255, 0u8..=255, 0u8..=255).prop_map(|(b, c, d)| IpCase::V4([0, b, c, d], false)),
        (0u8..=255, 0u8..=255, 0u8..=255).prop_map(|(b, c, d)| IpCase::V4([10, b, c, d], false)),
        (0u8..=255, 0u8..=255, 0u8..=255).prop_map(|(b, c, d)| IpCase::V4([127, b, c, d], false)),
        (64u8..=127, 0u8..=255, 0u8..=255).prop_map(|(b, c, d)| IpCase::V4([100, b, c, d], false)),
        (0u8..=255, 0u8..=255).prop_map(|(c, d)| IpCase::V4([169, 254, c, d], false)),
        (16u8..=31, 0u8..=255, 0u8..=255).prop_map(|(b, c, d)| IpCase::V4([172, b, c, d], false)),
        (0u8..=255, 0u8..=255).prop_map(|(c, d)| IpCase::V4([192, 168, c, d], false)),
        (2u8..=2, 0u8..=255).prop_map(|(c, d)| IpCase::V4([192, 0, c, d], false)),
        (18u8..=19, 0u8..=255, 0u8..=255).prop_map(|(b, c, d)| IpCase::V4([198, b, c, d], false)),
        (0u8..=255).prop_map(|d| IpCase::V4([198, 51, 100, d], false)),
        (0u8..=255).prop_map(|d| IpCase::V4([203, 0, 113, d], false)),
        (224u8..=255, 0u8..=255, 0u8..=255, 0u8..=255)
            .prop_map(|(a, b, c, d)| IpCase::V4([a, b, c, d], false)),
        // IPv4 public examples, including boundaries around private/CGNAT.
        prop_oneof![
            Just([8, 8, 8, 8]),
            Just([1, 1, 1, 1]),
            Just([140, 82, 121, 3]),
            Just([172, 15, 255, 255]),
            Just([172, 32, 0, 1]),
            Just([100, 63, 255, 255]),
            Just([100, 128, 0, 1]),
        ]
        .prop_map(|octets| IpCase::V4(octets, true)),
        // IPv6 rejected ranges.
        Just(IpCase::V6([0, 0, 0, 0, 0, 0, 0, 0], false)),
        Just(IpCase::V6([0, 0, 0, 0, 0, 0, 0, 1], false)),
        (0xfe80u16..=0xfebfu16).prop_map(|first| IpCase::V6([first, 0, 0, 0, 0, 0, 0, 1], false)),
        (0xfc00u16..=0xfdffu16).prop_map(|first| IpCase::V6([first, 0, 0, 0, 0, 0, 0, 1], false)),
        (0xff00u16..=0xffffu16).prop_map(|first| IpCase::V6([first, 0, 0, 0, 0, 0, 0, 1], false)),
        Just(IpCase::V6([0x2001, 0x0db8, 0, 0, 0, 0, 0, 1], false)),
        Just(IpCase::V6([0x0100, 0, 0, 0, 0, 0, 0, 1], false)),
        Just(IpCase::V6([0, 0, 0, 0, 0, 0xffff, 0x7f00, 0x0001], false)),
        Just(IpCase::V6([0, 0, 0, 0, 0, 0xffff, 0x0808, 0x0808], true)),
        Just(IpCase::V6(
            [0x2606, 0x4700, 0x4700, 0, 0, 0, 0, 0x1111],
            true
        )),
        Just(IpCase::V6(
            [0x2001, 0x4860, 0x4860, 0, 0, 0, 0, 0x8888],
            true
        )),
    ]
}

#[derive(Debug, Clone)]
enum BlockedUrlCase {
    NonHttpsPublicV4([u8; 4]),
    HttpsBlockedV4([u8; 4]),
    HttpsBlockedV6([u16; 8]),
    Localhost,
}

fn blocked_url_case() -> impl Strategy<Value = BlockedUrlCase> {
    prop_oneof![
        prop_oneof![Just([8, 8, 8, 8]), Just([1, 1, 1, 1])]
            .prop_map(BlockedUrlCase::NonHttpsPublicV4),
        prop_oneof![
            Just([127, 0, 0, 1]),
            Just([10, 0, 0, 5]),
            Just([169, 254, 1, 1]),
            Just([192, 168, 1, 1])
        ]
        .prop_map(BlockedUrlCase::HttpsBlockedV4),
        prop_oneof![
            Just([0, 0, 0, 0, 0, 0, 0, 1]),
            Just([0xfe80, 0, 0, 0, 0, 0, 0, 1]),
            Just([0xfc00, 0, 0, 0, 0, 0, 0, 1]),
        ]
        .prop_map(BlockedUrlCase::HttpsBlockedV6),
        Just(BlockedUrlCase::Localhost),
    ]
}

fn ip_from_case(case: IpCase) -> (IpAddr, bool) {
    match case {
        IpCase::V4(octets, expected) => (IpAddr::V4(Ipv4Addr::from(octets)), expected),
        IpCase::V6(segments, expected) => (IpAddr::V6(Ipv6Addr::from(segments)), expected),
    }
}

fn blocked_url(case: BlockedUrlCase) -> String {
    match case {
        BlockedUrlCase::NonHttpsPublicV4(octets) => {
            let ip = Ipv4Addr::from(octets);
            format!("http://{ip}/skill.md")
        }
        BlockedUrlCase::HttpsBlockedV4(octets) => {
            let ip = Ipv4Addr::from(octets);
            format!("https://{ip}/skill.md")
        }
        BlockedUrlCase::HttpsBlockedV6(segments) => {
            let ip = Ipv6Addr::from(segments);
            format!("https://[{ip}]/skill.md")
        }
        BlockedUrlCase::Localhost => "https://localhost/skill.md".to_string(),
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    /// Feature: tauri-rewrite, Property 28: SSRF host classification.
    ///
    /// The pure address classifier accepts public IPv4/IPv6 examples and rejects
    /// loopback, link-local, private, documentation, multicast, CGNAT,
    /// unspecified, and reserved ranges.
    ///
    /// **Validates: Requirement 13.5**
    #[test]
    fn ssrf_address_classifier_matches_public_matrix(case in ip_case()) {
        let (ip, expected_public) = ip_from_case(case);
        prop_assert_eq!(skill_safety::is_public_ip(ip), expected_public, "ip={}", ip);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(96))]

    /// Feature: tauri-rewrite, Property 28: SSRF host classification.
    ///
    /// URLs that fail the skill-fetch SSRF policy during synchronous preflight
    /// (non-HTTPS scheme, localhost, or non-public IP literal) are rejected with
    /// `SSRF_BLOCKED`, so no outbound request is attempted.
    ///
    /// **Validates: Requirement 13.5**
    #[test]
    fn skill_fetch_precheck_rejects_blocked_urls_without_network(case in blocked_url_case()) {
        let url = blocked_url(case);
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let err = rt.block_on(fetch_content(&url)).unwrap_err();
        prop_assert_eq!(err.code, ErrorCode::SsrfBlocked, "url={}", url);
    }
}

// ---------------------------------------------------------------------------
// Property 29: Skill path confinement
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
enum EscapingPathCase {
    Parent,
    NestedParent,
    BackslashParent,
    AbsoluteOutside,
    DriveLike,
    SchemeLike,
    NullByte,
}

fn escaping_path_case() -> impl Strategy<Value = EscapingPathCase> {
    prop_oneof![
        Just(EscapingPathCase::Parent),
        Just(EscapingPathCase::NestedParent),
        Just(EscapingPathCase::BackslashParent),
        Just(EscapingPathCase::AbsoluteOutside),
        Just(EscapingPathCase::DriveLike),
        Just(EscapingPathCase::SchemeLike),
        Just(EscapingPathCase::NullByte),
    ]
}

fn local_escape_payload(case: EscapingPathCase, outside: &Path) -> String {
    match case {
        EscapingPathCase::Parent => "../outside.txt".to_string(),
        EscapingPathCase::NestedParent => "sub/../../outside.txt".to_string(),
        EscapingPathCase::BackslashParent => "..\\outside.txt".to_string(),
        EscapingPathCase::AbsoluteOutside => outside.to_string_lossy().to_string(),
        EscapingPathCase::DriveLike => "C:\\outside.txt".to_string(),
        EscapingPathCase::SchemeLike => "file:outside.txt".to_string(),
        EscapingPathCase::NullByte => "safe\0name.txt".to_string(),
    }
}

#[derive(Debug, Clone)]
enum PlatformEscapeCase {
    SkillNameParent,
    SkillNameNested,
    SkillNameBackslash,
    SkillNameDotDot,
    SkillNameAbsolute,
    SkillFileParent,
    SkillFileNestedParent,
    SkillFileBackslashParent,
    SkillFileEmpty,
    SkillFileNullByte,
}

fn platform_escape_case() -> impl Strategy<Value = PlatformEscapeCase> {
    prop_oneof![
        Just(PlatformEscapeCase::SkillNameParent),
        Just(PlatformEscapeCase::SkillNameNested),
        Just(PlatformEscapeCase::SkillNameBackslash),
        Just(PlatformEscapeCase::SkillNameDotDot),
        Just(PlatformEscapeCase::SkillNameAbsolute),
        Just(PlatformEscapeCase::SkillFileParent),
        Just(PlatformEscapeCase::SkillFileNestedParent),
        Just(PlatformEscapeCase::SkillFileBackslashParent),
        Just(PlatformEscapeCase::SkillFileEmpty),
        Just(PlatformEscapeCase::SkillFileNullByte),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    /// Feature: tauri-rewrite, Property 29: Skill path confinement.
    ///
    /// Escaping local-repository paths are rejected with `VALIDATION` before
    /// read/write/delete/mkdir side effects; an outside sentinel file remains
    /// unchanged and no partial repository path is created.
    ///
    /// **Validates: Requirement 11.8**
    #[test]
    fn local_repo_operations_reject_escaping_paths_without_io(case in escaping_path_case()) {
        let base = TempDir::new().unwrap();
        let repo = base.path().join("repo");
        fs::create_dir_all(&repo).unwrap();
        let outside = base.path().join("outside.txt");
        fs::write(&outside, "SECRET").unwrap();

        let rel = local_escape_payload(case, &outside);

        let write_err = skill_local::write(&repo, &rel, "HACKED").unwrap_err();
        prop_assert_eq!(write_err.code, ErrorCode::Validation, "write rel={:?}", rel);
        prop_assert_eq!(fs::read_to_string(&outside).unwrap(), "SECRET");
        prop_assert!(!repo.join("sub").exists(), "write must not create partial dirs");

        let read_err = skill_local::read(&repo, &rel).unwrap_err();
        prop_assert_eq!(read_err.code, ErrorCode::Validation, "read rel={:?}", rel);

        let delete_err = skill_local::delete(&repo, &rel).unwrap_err();
        prop_assert_eq!(delete_err.code, ErrorCode::Validation, "delete rel={:?}", rel);
        prop_assert_eq!(fs::read_to_string(&outside).unwrap(), "SECRET");

        let mkdir_err = skill_local::mkdir(&repo, &rel).unwrap_err();
        prop_assert_eq!(mkdir_err.code, ErrorCode::Validation, "mkdir rel={:?}", rel);
        prop_assert_eq!(fs::read_to_string(&outside).unwrap(), "SECRET");
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    /// Feature: tauri-rewrite, Property 29: Skill path confinement.
    ///
    /// Escaping skill names and escaping file-set paths for platform operations
    /// are rejected with `VALIDATION` before install/uninstall/status side
    /// effects; the platform skills directory is not created and no outside file
    /// is modified.
    ///
    /// **Validates: Requirement 12.8**
    #[test]
    fn platform_operations_reject_escaping_names_and_file_paths_without_io(case in platform_escape_case()) {
        let base = TempDir::new().unwrap();
        let platforms = one_platform(base.path(), "claude");
        let outside = base.path().join("outside.txt");
        fs::write(&outside, "SECRET").unwrap();

        match case {
            PlatformEscapeCase::SkillNameParent => {
                let err = skill_platform::install(&platforms, "claude", "../evil", &[skill_file("SKILL.md", "x")]).unwrap_err();
                prop_assert_eq!(err.code, ErrorCode::Validation);
                prop_assert_eq!(skill_platform::uninstall(&platforms, "claude", "../evil").unwrap_err().code, ErrorCode::Validation);
                prop_assert_eq!(skill_platform::status(&platforms, "../evil").unwrap_err().code, ErrorCode::Validation);
            }
            PlatformEscapeCase::SkillNameNested => {
                let err = skill_platform::install(&platforms, "claude", "nested/evil", &[skill_file("SKILL.md", "x")]).unwrap_err();
                prop_assert_eq!(err.code, ErrorCode::Validation);
            }
            PlatformEscapeCase::SkillNameBackslash => {
                let err = skill_platform::install(&platforms, "claude", "nested\\evil", &[skill_file("SKILL.md", "x")]).unwrap_err();
                prop_assert_eq!(err.code, ErrorCode::Validation);
            }
            PlatformEscapeCase::SkillNameDotDot => {
                let err = skill_platform::install(&platforms, "claude", "..", &[skill_file("SKILL.md", "x")]).unwrap_err();
                prop_assert_eq!(err.code, ErrorCode::Validation);
            }
            PlatformEscapeCase::SkillNameAbsolute => {
                let name = outside.to_string_lossy().to_string();
                let err = skill_platform::install(&platforms, "claude", &name, &[skill_file("SKILL.md", "x")]).unwrap_err();
                prop_assert_eq!(err.code, ErrorCode::Validation);
            }
            PlatformEscapeCase::SkillFileParent => {
                let err = skill_platform::install(&platforms, "claude", "safe-skill", &[skill_file("../outside.txt", "HACKED")]).unwrap_err();
                prop_assert_eq!(err.code, ErrorCode::Validation);
            }
            PlatformEscapeCase::SkillFileNestedParent => {
                let err = skill_platform::install(&platforms, "claude", "safe-skill", &[skill_file("docs/../../outside.txt", "HACKED")]).unwrap_err();
                prop_assert_eq!(err.code, ErrorCode::Validation);
            }
            PlatformEscapeCase::SkillFileBackslashParent => {
                let err = skill_platform::install(&platforms, "claude", "safe-skill", &[skill_file("..\\outside.txt", "HACKED")]).unwrap_err();
                prop_assert_eq!(err.code, ErrorCode::Validation);
            }
            PlatformEscapeCase::SkillFileEmpty => {
                let err = skill_platform::install(&platforms, "claude", "safe-skill", &[skill_file("", "x")]).unwrap_err();
                prop_assert_eq!(err.code, ErrorCode::Validation);
            }
            PlatformEscapeCase::SkillFileNullByte => {
                let err = skill_platform::install(&platforms, "claude", "safe-skill", &[skill_file("safe\0name.txt", "x")]).unwrap_err();
                prop_assert_eq!(err.code, ErrorCode::Validation);
            }
        }

        prop_assert_eq!(fs::read_to_string(&outside).unwrap(), "SECRET");
        prop_assert!(dir_empty_or_missing(&platforms[0].skills_dir));
        prop_assert!(!platforms[0].skills_dir.join("safe-skill").exists());
    }
}
