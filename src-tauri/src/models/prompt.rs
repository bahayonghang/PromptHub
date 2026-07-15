//! Prompt-related DTOs exchanged between the Command_Layer and the Frontend.
//!
//! Field names and value shapes mirror the existing TypeScript domain types
//! (Requirement 2.5). Timestamps cross the wire as ISO_8601 strings
//! (Requirement 4.9), so they are modeled as `String`.

use serde::{Deserialize, Serialize};

use super::enums::{PromptRevisionSource, PromptType, SortField, SortOrder};

/// A single prompt record (Requirement 6).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Prompt {
    /// Generated unique identifier.
    pub id: String,
    /// Prompt title (non-empty).
    pub title: String,
    /// Optional free-form description.
    pub description: Option<String>,
    /// Prompt kind; defaults to `text`.
    pub prompt_type: PromptType,
    /// Optional system prompt.
    pub system_prompt: Option<String>,
    /// User prompt (non-empty).
    pub user_prompt: String,
    /// Ordered chat messages. Empty for a simple text prompt.
    pub messages: Vec<PromptMessage>,
    /// Declared variables/placeholders.
    pub variables: Vec<Variable>,
    /// Free-form tags.
    pub tags: Vec<String>,
    /// Containing folder, or `None` when at the root.
    pub folder_id: Option<String>,
    /// Image file references.
    pub images: Vec<String>,
    /// Video file references.
    pub videos: Vec<String>,
    /// Favorite flag.
    pub is_favorite: bool,
    /// Pinned flag.
    pub is_pinned: bool,
    /// Whether content fields are encrypted at rest.
    pub is_private: bool,
    /// Whether private content is unavailable because no key is cached.
    pub is_locked: bool,
    /// Highest stored version number (0 when none).
    pub current_version: i64,
    /// Number of times the prompt has been used/copied.
    pub usage_count: i64,
    /// Optional source URL or reference.
    pub source: Option<String>,
    /// Optional personal notes.
    pub notes: Option<String>,
    /// Last AI test response, if any.
    pub last_ai_response: Option<String>,
    /// Creation time as an ISO_8601 string.
    pub created_at: String,
    /// Last-updated time as an ISO_8601 string.
    pub updated_at: String,
}

/// A template variable/placeholder declared on a prompt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Variable {
    /// Placeholder name (matches `{{name}}` in the prompt body).
    pub name: String,
    /// Input kind: `text` | `textarea` | `number` | `select`.
    pub r#type: String,
    /// Optional display label.
    pub label: Option<String>,
    /// Optional default value.
    pub default_value: Option<String>,
    /// Allowed options for `select` inputs.
    pub options: Option<Vec<String>>,
    /// Whether a value is required.
    pub required: bool,
}

/// One ordered message in a chat-style prompt definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptMessage {
    /// Message role: `system`, `user`, or `assistant`.
    pub role: String,
    /// Message content, including any declared `{{variable}}` placeholders.
    pub content: String,
}

/// A snapshot of a prompt captured as a version (Requirement 7).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptVersion {
    /// Generated unique identifier.
    pub id: String,
    /// Owning prompt identifier.
    pub prompt_id: String,
    /// Sequential version number (starts at 1).
    pub version: i64,
    /// Snapshot of the system prompt.
    pub system_prompt: Option<String>,
    /// Snapshot of the user prompt.
    pub user_prompt: String,
    /// Snapshot of ordered chat messages.
    pub messages: Vec<PromptMessage>,
    /// Snapshot of the variables.
    pub variables: Vec<Variable>,
    pub title: String,
    pub description: Option<String>,
    pub prompt_type: PromptType,
    pub tags: Vec<String>,
    pub folder_id: Option<String>,
    pub images: Vec<String>,
    pub videos: Vec<String>,
    pub is_favorite: bool,
    pub is_pinned: bool,
    pub is_private: bool,
    pub source: Option<String>,
    pub notes: Option<String>,
    /// Optional note (≤1000 characters).
    pub note: Option<String>,
    /// AI test response captured with this version, if any.
    pub ai_response: Option<String>,
    pub source_action: PromptRevisionSource,
    pub parent_revision_id: Option<String>,
    /// Creation time as an ISO_8601 string.
    pub created_at: String,
}

/// Search/filter query for prompts (Requirement 5).
///
/// All fields are optional; missing fields take their documented defaults in
/// the Prompt_Service (keyword: none, sort: `updatedAt` `desc`, limit 50,
/// offset 0).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchQuery {
    /// Full-text keyword.
    pub keyword: Option<String>,
    /// Tag filters (conjunctive).
    pub tags: Option<Vec<String>>,
    /// Folder filter.
    pub folder_id: Option<String>,
    /// Favorite filter.
    pub is_favorite: Option<bool>,
    /// Sort field; defaults to `updatedAt`.
    pub sort_by: Option<SortField>,
    /// Sort direction; defaults to `desc`.
    pub sort_order: Option<SortOrder>,
    /// Result limit; clamped to `1..=100`, default 50.
    pub limit: Option<u32>,
    /// Result offset; `>= 0`, default 0.
    pub offset: Option<u32>,
}

/// A deterministic page returned by `prompt.search`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptPage {
    pub items: Vec<Prompt>,
    pub total: u64,
    pub limit: u32,
    pub offset: u32,
    pub has_more: bool,
}

impl std::ops::Deref for PromptPage {
    type Target = [Prompt];

    fn deref(&self) -> &Self::Target {
        &self.items
    }
}
