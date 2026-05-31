//! Rule file DTOs exchanged between the Command_Layer and the Frontend
//! (Requirement 14).
//!
//! `RuleFileContent` flattens the rule descriptor fields and adds the current
//! content plus its version history, mirroring the existing TypeScript
//! `RuleFileContent` type (Requirement 2.5).

use serde::{Deserialize, Serialize};

use super::enums::SyncStatus;

/// A managed rule file with its current content and version history.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleFileContent {
    /// Rule file identifier.
    pub id: String,
    /// Owning platform identifier.
    pub platform_id: String,
    /// Human-readable platform name.
    pub platform_name: String,
    /// Platform icon.
    pub platform_icon: String,
    /// Platform description.
    pub platform_description: String,
    /// Display name of the rule file.
    pub name: String,
    /// Rule file description.
    pub description: String,
    /// Display path of the rule file.
    pub path: String,
    /// Whether the target file currently exists.
    pub exists: bool,
    /// Managed (canonical) file path, if applicable.
    pub managed_path: Option<String>,
    /// Target file path, if applicable.
    pub target_path: Option<String>,
    /// Project root path for project-scoped rules.
    pub project_root_path: Option<String>,
    /// Synchronization state relative to the target file.
    pub sync_status: Option<SyncStatus>,
    /// Current rule file content.
    pub content: String,
    /// Version history snapshots, most-recent first.
    pub versions: Vec<RuleVersionSnapshot>,
}

/// A point-in-time snapshot of a rule file's content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleVersionSnapshot {
    /// Generated unique identifier.
    pub id: String,
    /// Save time as an ISO_8601 string.
    pub saved_at: String,
    /// Snapshot content.
    pub content: String,
    /// What triggered the snapshot: `manual-save` | `ai-rewrite` | `create`.
    pub source: String,
}
