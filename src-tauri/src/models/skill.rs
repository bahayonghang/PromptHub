//! Skill-related DTOs exchanged between the Command_Layer and the Frontend
//! (Requirements 9–13).
//!
//! Fields follow the `skills`/`skill_versions` schema columns; timestamps cross
//! the wire as ISO_8601 strings (Requirement 4.9). Unions that are not part of
//! the required shared enums (e.g. category, protocol type) are modeled as
//! `String` to keep this task minimal.

use serde::{Deserialize, Serialize};

use super::enums::SafetyLevel;

/// A reusable skill record defined by a SKILL.md document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Skill {
    /// Generated unique identifier.
    pub id: String,
    /// Skill name (non-empty).
    pub name: String,
    /// Optional description.
    pub description: Option<String>,
    /// SKILL.md content / instructions.
    pub content: Option<String>,
    /// Protocol type; defaults to `skill`.
    pub protocol_type: String,
    /// Optional skill version label.
    pub version: Option<String>,
    /// Optional author.
    pub author: Option<String>,
    /// Free-form tags.
    pub tags: Vec<String>,
    /// Favorite flag.
    pub is_favorite: bool,
    /// Source URL when imported.
    pub source_url: Option<String>,
    /// Stable source identity for same-name variants.
    pub source_id: Option<String>,
    /// Human-readable source label.
    pub source_label: Option<String>,
    /// Source branch when imported from a git-like store.
    pub source_branch: Option<String>,
    /// Source directory when imported from a nested store.
    pub source_directory: Option<String>,
    /// Canonical skill path inside the source.
    pub canonical_skill_path: Option<String>,
    /// Absolute path to the local repository directory.
    pub local_repo_path: Option<String>,
    /// Stable fingerprint of the full skill directory.
    pub directory_fingerprint: Option<String>,
    /// Skill icon URL.
    pub icon_url: Option<String>,
    /// Emoji icon fallback.
    pub icon_emoji: Option<String>,
    /// Icon background color.
    pub icon_background: Option<String>,
    /// Skill category; defaults to `general`.
    pub category: String,
    /// Whether this is a built-in skill.
    pub is_builtin: bool,
    /// Unique slug in the registry.
    pub registry_slug: Option<String>,
    /// Remote SKILL.md URL.
    pub content_url: Option<String>,
    /// Latest safety classification.
    pub safety_level: Option<SafetyLevel>,
    /// Numeric safety score (0–100, higher is safer).
    pub safety_score: Option<i64>,
    /// Full safety report payload (structured JSON).
    pub safety_report: Option<serde_json::Value>,
    /// Time of the last safety scan as an ISO_8601 string.
    pub safety_scanned_at: Option<String>,
    /// Highest stored version number (0 when none).
    pub current_version: i64,
    /// Whether version tracking is enabled.
    pub version_tracking_enabled: bool,
    /// Creation time as an ISO_8601 string.
    pub created_at: String,
    /// Last-updated time as an ISO_8601 string.
    pub updated_at: String,
}

/// A snapshot of a skill captured as a version (Requirement 9.7).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillVersion {
    /// Generated unique identifier.
    pub id: String,
    /// Owning skill identifier.
    pub skill_id: String,
    /// Sequential version number.
    pub version: i64,
    /// Snapshot of the skill content.
    pub content: Option<String>,
    /// Snapshot of the multi-file file set.
    pub files_snapshot: Option<Vec<SkillFileSnapshot>>,
    /// Optional note.
    pub note: Option<String>,
    /// Creation time as an ISO_8601 string.
    pub created_at: String,
}

/// A single file captured in a multi-file skill version snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillFileSnapshot {
    /// Path relative to the skill repository root.
    pub relative_path: String,
    /// File content.
    pub content: String,
}
