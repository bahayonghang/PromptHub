//! SKILL.md parsing, serialization, and import (Requirement 10).
//!
//! A SKILL.md document is a YAML frontmatter block delimited by `---` lines
//! followed by a Markdown body. This module owns three operations, kept separate
//! from the skill CRUD/versioning rules in [`crate::services::skill`] so the
//! concerns don't churn each other:
//!
//! - [`parse_md`] (10.1, 10.4): extract the frontmatter, parse it with
//!   `serde_yaml`, require a non-empty `name` and `description`, and preserve the
//!   body verbatim. Malformed YAML, a missing/empty `name`/`description`, or a
//!   missing delimiter are rejected with a `VALIDATION` error identifying the
//!   offending field or syntax problem; nothing is persisted.
//! - [`serialize_md`] (10.2): emit `---\n{yaml}\n---\n{body}` where `{yaml}` is
//!   the frontmatter mapping serialized by `serde_yaml`. Together with
//!   [`parse_md`] this satisfies the round-trip property (10.3): because the
//!   frontmatter is a [`BTreeMap`] (deterministic key order) and the body is
//!   re-emitted unchanged, `parse_md(serialize_md(x))` recovers the same metadata
//!   pairs and body.
//! - [`import`] (10.5, 10.6): validate a JSON skill object, sanitize active
//!   content (`<script>` blocks and `on*` HTML event-handler attributes) out of
//!   the metadata and body, and persist via [`crate::services::skill::create`].
//!   Bad JSON, a non-conforming shape, or content that cannot be fully sanitized
//!   are rejected with a structured error and nothing is persisted.
//!
//! ## Frontmatter value type
//!
//! [`crate::models::ParsedSkillMd`] keeps frontmatter as
//! `BTreeMap<String, serde_json::Value>`. `serde_yaml` parses into
//! [`serde_yaml::Value`], which [`yaml_to_json`] converts to the stored
//! `serde_json::Value` shape; serialization goes the other way for free because
//! `serde_json::Value` implements `Serialize`, so `serde_yaml` can emit it
//! directly. The public model shape is unchanged.
//!
//! ## Sanitization scope (10.5, 10.6)
//!
//! [`sanitize_active_content`] is intentionally **not** a full HTML sanitizer. It
//! removes complete `<script>...</script>` blocks (case-insensitive) and HTML
//! `on*` event-handler attributes (`onclick=...`, `onload='...'`, etc.), applied
//! to a fixed point so cascading removals settle. After sanitizing, [`import`]
//! re-scans for residual `<script` openings or `on*` handlers (for example an
//! unclosed `<script>` the block pattern cannot match); if any survive, the
//! import is rejected as unsanitizable (10.6).
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::sync::OnceLock;

use regex::Regex;
use rusqlite::Connection;
use serde::Deserialize;
use serde_json::Value as JsonValue;
use serde_yaml::Value as YamlValue;

use crate::error::AppError;
use crate::models::{ParsedSkillMd, Skill};
use crate::services::skill::{self, SkillCreate};

/// Parses a SKILL.md document into a [`ParsedSkillMd`] (Req 10.1, 10.4).
///
/// The document must begin with a `---` delimiter line; the YAML frontmatter runs
/// up to the next `---` line and everything after that line (less the single
/// newline terminating it) is preserved verbatim as the body. The frontmatter is
/// parsed as a YAML mapping and stored as name-value pairs; a non-empty `name`
/// and `description` are required. Any structural or YAML problem, or a
/// missing/empty required field, yields a `VALIDATION` error and no persistence.
pub fn parse_md(input: &str) -> Result<ParsedSkillMd, AppError> {
    // The document must open with a `---` delimiter line (Req 10.4).
    let after_open = strip_opening_delimiter(input).ok_or_else(|| {
        AppError::validation("SKILL.md must begin with a YAML frontmatter delimiter line `---`")
    })?;

    // The frontmatter ends at the next `---` line; the rest is the body verbatim.
    let (yaml_src, body) = split_at_closing_delimiter(after_open).ok_or_else(|| {
        AppError::validation("SKILL.md frontmatter is missing a closing `---` delimiter")
    })?;

    // Parse the frontmatter as YAML; malformed YAML is a VALIDATION error (Req 10.4).
    let yaml_value: YamlValue = serde_yaml::from_str(yaml_src).map_err(|e| {
        AppError::validation(format!("SKILL.md frontmatter is not valid YAML: {e}"))
    })?;

    let mapping = match yaml_value {
        YamlValue::Mapping(mapping) => mapping,
        // Empty frontmatter parses to Null; treat it as "missing name" below.
        YamlValue::Null => serde_yaml::Mapping::new(),
        _ => {
            return Err(AppError::validation(
                "SKILL.md frontmatter must be a YAML mapping of fields",
            ))
        }
    };

    // Preserve every field as a name-value pair (Req 10.1).
    let mut frontmatter: BTreeMap<String, JsonValue> = BTreeMap::new();
    for (key, value) in mapping {
        frontmatter.insert(yaml_key_to_string(key)?, yaml_to_json(value));
    }

    // Require a non-empty `name` and `description`, identifying the offending
    // field on failure (Req 10.4).
    require_non_empty_string(&frontmatter, "name")?;
    require_non_empty_string(&frontmatter, "description")?;

    Ok(ParsedSkillMd {
        frontmatter,
        body: body.to_string(),
    })
}

/// Serializes a [`ParsedSkillMd`] back to a SKILL.md document (Req 10.2).
///
/// Emits `---\n{yaml}\n---\n{body}` where `{yaml}` is the frontmatter mapping
/// serialized by `serde_yaml` (deterministic order from the [`BTreeMap`]). The
/// trailing newline `serde_yaml` appends is trimmed so exactly one newline
/// precedes the closing delimiter, which keeps [`parse_md`] round-tripping (10.3).
pub fn serialize_md(parsed: &ParsedSkillMd) -> Result<String, AppError> {
    let yaml = serde_yaml::to_string(&parsed.frontmatter).map_err(|e| {
        AppError::internal(format!("failed to serialize SKILL.md frontmatter: {e}"))
    })?;
    let yaml = yaml.trim_end_matches('\n');
    Ok(format!("---\n{yaml}\n---\n{body}", body = parsed.body))
}

/// JSON shape accepted by [`import`] (Req 10.5).
///
/// `name` is required; every other field is optional. Deserialization fails for
/// non-object JSON or a missing `name`, both of which surface as `VALIDATION`
/// (Req 10.6). Unknown fields are tolerated so exported skills with extra
/// metadata still import.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SkillImport {
    /// Skill name (required).
    name: String,
    /// Optional description.
    #[serde(default)]
    description: Option<String>,
    /// Optional SKILL.md content / body.
    #[serde(default)]
    content: Option<String>,
    /// Optional tags.
    #[serde(default)]
    tags: Option<Vec<String>>,
    /// Optional author.
    #[serde(default)]
    author: Option<String>,
    /// Optional version label.
    #[serde(default)]
    version: Option<String>,
    /// Optional source URL.
    #[serde(default)]
    source_url: Option<String>,
}

/// Validates, sanitizes, and persists an imported skill from JSON (Req 10.5, 10.6).
///
/// The JSON must conform to the skill object shape (at minimum a string `name`);
/// malformed JSON or a non-conforming shape is rejected with `VALIDATION`. Active
/// content (`<script>` blocks and `on*` event-handler attributes) is removed from
/// the metadata and body before persisting; if dangerous content cannot be fully
/// removed, the import is rejected. On success the sanitized skill is created via
/// [`crate::services::skill::create`] and the persisted record is returned.
pub fn import(conn: &Connection, json: &str) -> Result<Skill, AppError> {
    let raw: SkillImport = serde_json::from_str(json).map_err(|e| {
        AppError::validation(format!("skill import is not a valid skill object: {e}"))
    })?;

    if raw.name.trim().is_empty() {
        return Err(AppError::validation(
            "skill import `name` must not be empty",
        ));
    }

    // Sanitize every metadata field and the body (Req 10.5).
    let name = sanitize_active_content(&raw.name);
    let description = raw.description.as_deref().map(sanitize_active_content);
    let content = raw.content.as_deref().map(sanitize_active_content);
    let author = raw.author.as_deref().map(sanitize_active_content);
    let tags = raw.tags.map(|tags| {
        tags.iter()
            .map(|t| sanitize_active_content(t))
            .collect::<Vec<_>>()
    });

    // Reject if active content survived sanitization (Req 10.6).
    let mut combined = name.clone();
    for field in [
        description.as_deref(),
        content.as_deref(),
        author.as_deref(),
    ]
    .into_iter()
    .flatten()
    {
        combined.push('\n');
        combined.push_str(field);
    }
    if let Some(tags) = &tags {
        for tag in tags {
            combined.push('\n');
            combined.push_str(tag);
        }
    }
    if !is_fully_sanitized(&combined) {
        return Err(AppError::validation(
            "skill import contains active content that could not be sanitized",
        ));
    }

    // The name must still be present after removing active content.
    if name.trim().is_empty() {
        return Err(AppError::validation(
            "skill import `name` is empty after removing active content",
        ));
    }

    let create = SkillCreate {
        name,
        description,
        content,
        author,
        tags,
        version: raw.version,
        source_url: raw.source_url,
        ..Default::default()
    };
    skill::create(conn, create)
}

// --- frontmatter delimiter handling ----------------------------------------

/// Returns the slice after the opening `---` delimiter line, or `None` when the
/// input does not start with one.
///
/// The first line (trailing whitespace/`\r` tolerated) must be exactly `---`.
fn strip_opening_delimiter(input: &str) -> Option<&str> {
    match input.find('\n') {
        Some(nl) if input[..nl].trim_end() == "---" => Some(&input[nl + 1..]),
        // A document that is only `---` has no frontmatter body and no closing
        // delimiter; treat it as having no closing delimiter (handled by caller).
        None if input.trim_end() == "---" => Some(""),
        _ => None,
    }
}

/// Splits the post-opening text at the next `---` line into `(yaml, body)`.
///
/// `yaml` is the frontmatter text before the closing delimiter line; `body` is
/// everything after that line's terminating newline, preserved verbatim. Returns
/// `None` when no closing `---` line exists.
fn split_at_closing_delimiter(after_open: &str) -> Option<(&str, &str)> {
    let mut cursor = 0;
    loop {
        let (line, next) = match after_open[cursor..].find('\n') {
            Some(rel) => (&after_open[cursor..cursor + rel], cursor + rel + 1),
            None => (&after_open[cursor..], after_open.len()),
        };
        if line.trim_end() == "---" {
            return Some((&after_open[..cursor], &after_open[next..]));
        }
        if next >= after_open.len() {
            return None;
        }
        cursor = next;
    }
}

// --- YAML <-> JSON conversion ----------------------------------------------

/// Converts a [`serde_yaml::Value`] into the [`serde_json::Value`] stored in
/// [`ParsedSkillMd`]. Tagged nodes keep their value and drop the tag.
fn yaml_to_json(value: YamlValue) -> JsonValue {
    match value {
        YamlValue::Null => JsonValue::Null,
        YamlValue::Bool(b) => JsonValue::Bool(b),
        YamlValue::Number(n) => yaml_number_to_json(n),
        YamlValue::String(s) => JsonValue::String(s),
        YamlValue::Sequence(seq) => JsonValue::Array(seq.into_iter().map(yaml_to_json).collect()),
        YamlValue::Mapping(map) => {
            let mut obj = serde_json::Map::with_capacity(map.len());
            for (k, v) in map {
                // Non-scalar nested keys collapse to their string form; this only
                // matters for unusual nested mappings, never for the top-level
                // SKILL.md fields.
                let key = yaml_key_to_string(k).unwrap_or_else(|_| "?".to_string());
                obj.insert(key, yaml_to_json(v));
            }
            JsonValue::Object(obj)
        }
        YamlValue::Tagged(tagged) => yaml_to_json(tagged.value),
    }
}

/// Converts a YAML number to a JSON number, preferring integer representations.
fn yaml_number_to_json(n: serde_yaml::Number) -> JsonValue {
    if let Some(i) = n.as_i64() {
        JsonValue::Number(i.into())
    } else if let Some(u) = n.as_u64() {
        JsonValue::Number(u.into())
    } else if let Some(f) = n.as_f64() {
        serde_json::Number::from_f64(f)
            .map(JsonValue::Number)
            .unwrap_or(JsonValue::Null)
    } else {
        JsonValue::Null
    }
}

/// Renders a YAML mapping key as a string (frontmatter keys are scalars).
fn yaml_key_to_string(key: YamlValue) -> Result<String, AppError> {
    match key {
        YamlValue::String(s) => Ok(s),
        YamlValue::Bool(b) => Ok(b.to_string()),
        YamlValue::Number(n) => Ok(n.to_string()),
        YamlValue::Null => Ok("null".to_string()),
        _ => Err(AppError::validation(
            "SKILL.md frontmatter keys must be scalar values",
        )),
    }
}

/// Asserts that `field` is present in the frontmatter as a non-empty string,
/// returning a `VALIDATION` error that names the offending field otherwise.
fn require_non_empty_string(
    frontmatter: &BTreeMap<String, JsonValue>,
    field: &str,
) -> Result<(), AppError> {
    match frontmatter.get(field) {
        Some(JsonValue::String(s)) if !s.trim().is_empty() => Ok(()),
        Some(JsonValue::String(_)) => Err(AppError::validation(format!(
            "SKILL.md frontmatter field `{field}` must not be empty"
        ))),
        Some(_) => Err(AppError::validation(format!(
            "SKILL.md frontmatter field `{field}` must be a non-empty string"
        ))),
        None => Err(AppError::validation(format!(
            "SKILL.md frontmatter is missing required field `{field}`"
        ))),
    }
}

// --- sanitization (Req 10.5, 10.6) -----------------------------------------

/// Matches a complete `<script ...>...</script>` block (case-insensitive, dotall).
fn script_block_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?is)<script\b[^>]*>.*?</script\s*>").unwrap())
}

/// Matches an HTML `on*` event-handler attribute (`onclick="..."`, `onload=x`, …).
///
/// A word boundary precedes `on` so the attribute is recognized after spaces,
/// quotes, or `>`; the value may be double-quoted, single-quoted, or unquoted.
fn on_attr_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"(?i)\bon[a-z]+\s*=\s*("[^"]*"|'[^']*'|[^\s>]+)"#).unwrap())
}

/// Matches a residual `<script` opening tag the block pattern could not pair.
fn residual_script_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)<script\b").unwrap())
}

/// Removes `<script>` blocks and `on*` event-handler attributes from `input`.
///
/// Removal is applied to a fixed point so cascading matches (an attribute exposed
/// only after an adjacent one is removed) are all eliminated. Each iteration only
/// deletes text, so the process strictly shrinks the string and terminates.
fn sanitize_active_content(input: &str) -> String {
    let mut current = input.to_string();
    loop {
        let without_scripts = script_block_re().replace_all(&current, "");
        let without_handlers = on_attr_re().replace_all(&without_scripts, "").into_owned();
        if without_handlers == current {
            return without_handlers;
        }
        current = without_handlers;
    }
}

/// Reports whether `text` is free of residual active content after sanitization.
fn is_fully_sanitized(text: &str) -> bool {
    !residual_script_re().is_match(text) && !on_attr_re().is_match(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorCode;
    use crate::storage::{create_memory_pool, init_schema, DbPool};
    use serde_json::json;

    fn schema_pool() -> DbPool {
        let pool = create_memory_pool().expect("memory pool");
        init_schema(&pool.get().expect("conn")).expect("schema");
        pool
    }

    // --- parse_md (Req 10.1) ----------------------------------------------

    #[test]
    fn parse_preserves_frontmatter_pairs_and_body_verbatim() {
        let doc = "---\nname: My Skill\ndescription: Does things\ntags:\n  - a\n  - b\nversion: \"1.0\"\n---\n# Heading\n\nBody **text** here.\n";
        let parsed = parse_md(doc).unwrap();

        assert_eq!(parsed.frontmatter.get("name").unwrap(), &json!("My Skill"));
        assert_eq!(
            parsed.frontmatter.get("description").unwrap(),
            &json!("Does things")
        );
        assert_eq!(parsed.frontmatter.get("tags").unwrap(), &json!(["a", "b"]));
        assert_eq!(parsed.frontmatter.get("version").unwrap(), &json!("1.0"));
        // Body preserved exactly (the single newline after `---` is consumed).
        assert_eq!(parsed.body, "# Heading\n\nBody **text** here.\n");
    }

    #[test]
    fn parse_preserves_body_containing_delimiter_lines() {
        let doc = "---\nname: X\ndescription: Y\n---\nabove\n---\nbelow";
        let parsed = parse_md(doc).unwrap();
        // Only the first `---` after the frontmatter closes it; later `---`
        // stays in the body.
        assert_eq!(parsed.body, "above\n---\nbelow");
    }

    // --- round-trip (Req 10.2, 10.3) --------------------------------------

    fn fm(pairs: &[(&str, JsonValue)]) -> ParsedSkillMd {
        let mut frontmatter = BTreeMap::new();
        for (k, v) in pairs {
            frontmatter.insert((*k).to_string(), v.clone());
        }
        ParsedSkillMd {
            frontmatter,
            body: String::new(),
        }
    }

    #[test]
    fn round_trip_preserves_metadata_and_body() {
        let cases = vec![
            ParsedSkillMd {
                body: "# Title\n\nSome body.\n".to_string(),
                ..fm(&[
                    ("name", json!("Alpha")),
                    ("description", json!("first skill")),
                ])
            },
            ParsedSkillMd {
                body: "body with trailing text".to_string(),
                ..fm(&[
                    ("name", json!("Beta")),
                    ("description", json!("second")),
                    ("tags", json!(["x", "y", "z"])),
                    ("author", json!("alice")),
                    ("enabled", json!(true)),
                    ("count", json!(3)),
                ])
            },
            ParsedSkillMd {
                body: String::new(),
                ..fm(&[
                    ("name", json!("Gamma")),
                    ("description", json!("empty body")),
                ])
            },
            ParsedSkillMd {
                body: "leading blank line below\n\nmore".to_string(),
                ..fm(&[
                    ("name", json!("Delta")),
                    ("description", json!("multi line body")),
                ])
            },
        ];

        for original in cases {
            let serialized = serialize_md(&original).unwrap();
            let reparsed = parse_md(&serialized).unwrap();
            assert_eq!(
                reparsed, original,
                "round-trip mismatch for serialized:\n{serialized}"
            );
        }
    }

    // --- rejection (Req 10.4) ---------------------------------------------

    #[test]
    fn parse_rejects_missing_opening_delimiter() {
        let err = parse_md("name: X\ndescription: Y\n").unwrap_err();
        assert_eq!(err.code, ErrorCode::Validation);
    }

    #[test]
    fn parse_rejects_missing_closing_delimiter() {
        let err = parse_md("---\nname: X\ndescription: Y\n").unwrap_err();
        assert_eq!(err.code, ErrorCode::Validation);
    }

    #[test]
    fn parse_rejects_malformed_yaml() {
        // A tab in indentation / unbalanced bracket makes the YAML invalid.
        let doc = "---\nname: X\ndescription: Y\ntags: [unterminated\n---\nbody";
        let err = parse_md(doc).unwrap_err();
        assert_eq!(err.code, ErrorCode::Validation);
    }

    #[test]
    fn parse_rejects_missing_name() {
        let err = parse_md("---\ndescription: Y\n---\nbody").unwrap_err();
        assert_eq!(err.code, ErrorCode::Validation);
        assert!(err.message.contains("name"), "message: {}", err.message);
    }

    #[test]
    fn parse_rejects_missing_description() {
        let err = parse_md("---\nname: X\n---\nbody").unwrap_err();
        assert_eq!(err.code, ErrorCode::Validation);
        assert!(
            err.message.contains("description"),
            "message: {}",
            err.message
        );
    }

    #[test]
    fn parse_rejects_empty_name_and_empty_description() {
        let empty_name = parse_md("---\nname: \"  \"\ndescription: Y\n---\nb").unwrap_err();
        assert_eq!(empty_name.code, ErrorCode::Validation);
        assert!(empty_name.message.contains("name"));

        let empty_desc = parse_md("---\nname: X\ndescription: \"\"\n---\nb").unwrap_err();
        assert_eq!(empty_desc.code, ErrorCode::Validation);
        assert!(empty_desc.message.contains("description"));
    }

    #[test]
    fn parse_rejects_non_string_name() {
        let err = parse_md("---\nname: 42\ndescription: Y\n---\nb").unwrap_err();
        assert_eq!(err.code, ErrorCode::Validation);
        assert!(err.message.contains("name"));
    }

    // --- import (Req 10.5, 10.6) ------------------------------------------

    #[test]
    fn import_rejects_malformed_json() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let err = import(&conn, "{ not json").unwrap_err();
        assert_eq!(err.code, ErrorCode::Validation);
    }

    #[test]
    fn import_rejects_wrong_shape() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();

        // A JSON array is not a skill object.
        let arr = import(&conn, "[1, 2, 3]").unwrap_err();
        assert_eq!(arr.code, ErrorCode::Validation);

        // An object missing the required `name` field.
        let no_name = import(&conn, r#"{"description": "x"}"#).unwrap_err();
        assert_eq!(no_name.code, ErrorCode::Validation);

        // Nothing was persisted.
        assert!(skill::list(&conn).unwrap().is_empty());
    }

    #[test]
    fn import_sanitizes_script_blocks_and_event_handlers() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();

        let json = r#"{
            "name": "Clean Skill",
            "description": "safe <script>alert('x')</script>desc",
            "content": "<div onclick=\"steal()\" onload='go()'>hello</div><script>evil()</script>"
        }"#;
        let skill = import(&conn, json).unwrap();

        let description = skill.description.unwrap();
        let content = skill.content.unwrap();
        assert!(!description.to_lowercase().contains("<script"));
        assert!(!content.to_lowercase().contains("<script"));
        assert!(!content.to_lowercase().contains("onclick"));
        assert!(!content.to_lowercase().contains("onload"));
        // Surrounding, non-active text is preserved.
        assert!(description.contains("safe"));
        assert!(description.contains("desc"));
        assert!(content.contains("hello"));
    }

    #[test]
    fn import_rejects_content_that_cannot_be_sanitized() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();

        // An unclosed `<script>` cannot be removed as a block, so a residual
        // opening tag survives and the import is rejected (Req 10.6).
        let json =
            r#"{"name": "Bad", "description": "ok", "content": "before <script>evil() no close"}"#;
        let err = import(&conn, json).unwrap_err();
        assert_eq!(err.code, ErrorCode::Validation);

        // Nothing was persisted.
        assert!(skill::list(&conn).unwrap().is_empty());
    }

    #[test]
    fn import_persists_and_returns_skill() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();

        let json = r##"{
            "name": "Persisted",
            "description": "a skill",
            "content": "# Body",
            "tags": ["one", "two"],
            "author": "bob"
        }"##;
        let created = import(&conn, json).unwrap();

        assert!(!created.id.is_empty());
        assert_eq!(created.name, "Persisted");
        assert_eq!(created.tags, vec!["one".to_string(), "two".to_string()]);

        // The skill is readable back from storage.
        let fetched = skill::get(&conn, &created.id).unwrap();
        assert_eq!(fetched, created);
    }
}
