//! Shared domain enums for the Command_Layer DTOs.
//!
//! Every enum derives `Serialize`/`Deserialize` and uses serde renaming so the
//! wire value spellings match the existing TypeScript domain types
//! (Requirement 2.5). The exact spellings are taken from the design's Data
//! Models / schema section.

use serde::{Deserialize, Serialize};

/// Prompt kind. Wire values: `text` | `image` | `video` (Requirement 6.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PromptType {
    /// Text/chat prompt — the default when none is supplied.
    #[default]
    Text,
    /// Image-generation prompt.
    Image,
    /// Video-generation prompt.
    Video,
}

/// Field a prompt search may sort by.
/// Wire values: `title` | `createdAt` | `updatedAt` | `usageCount` (Requirement 5.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SortField {
    /// Sort by prompt title.
    Title,
    /// Sort by creation timestamp.
    CreatedAt,
    /// Sort by last-updated timestamp — the default sort field.
    #[default]
    UpdatedAt,
    /// Sort by usage count.
    UsageCount,
}

/// Sort direction. Wire values: `asc` | `desc` (Requirement 5.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortOrder {
    /// Ascending order.
    Asc,
    /// Descending order — the default direction.
    #[default]
    Desc,
}

/// Rule file synchronization state relative to its target file.
/// Wire values: `synced` | `target-missing` | `out-of-sync` | `sync-error` (Requirement 14).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SyncStatus {
    /// Managed content matches the target file.
    Synced,
    /// Target file is missing.
    TargetMissing,
    /// Target file differs from managed content.
    OutOfSync,
    /// Sync could not be evaluated due to an error.
    SyncError,
}

/// Provenance for an immutable prompt revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PromptRevisionSource {
    Create,
    Save,
    Manual,
    Rollback,
    Import,
    Replace,
}
