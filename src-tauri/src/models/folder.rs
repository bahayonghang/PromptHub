//! Folder DTO exchanged between the Command_Layer and the Frontend (Requirement 8).
//!
//! Fields follow the `folders` schema columns; timestamps cross the wire as
//! ISO_8601 strings (Requirement 4.9).

use serde::{Deserialize, Serialize};

/// A hierarchical folder record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Folder {
    /// Generated unique identifier.
    pub id: String,
    /// Folder name (1–255 characters after trimming).
    pub name: String,
    /// Optional icon (e.g. an emoji).
    pub icon: Option<String>,
    /// Parent folder, or `None` for a root-level folder.
    pub parent_id: Option<String>,
    /// Sort order among siblings (zero-based).
    pub sort_order: i64,
    /// Creation time as an ISO_8601 string.
    pub created_at: String,
    /// Last-updated time as an ISO_8601 string, if set.
    pub updated_at: Option<String>,
}
