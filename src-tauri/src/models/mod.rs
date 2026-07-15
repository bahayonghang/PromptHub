//! Domain DTOs and shared types for the Command_Layer (Requirements 2.5, 4.9).
//!
//! Every struct derives `serde::{Serialize, Deserialize}` with
//! `#[serde(rename_all = "camelCase")]` so field names map to the existing
//! TypeScript domain types. Timestamps cross the wire as ISO_8601 strings and
//! are therefore modeled as `String` (Requirement 4.9).
//!
//! These types are not all referenced yet (services are implemented in later
//! tasks), so the module is allowed to carry currently-unused definitions.
#![allow(dead_code)]

mod enums;
mod evaluation;
mod folder;
mod prompt;
mod rules;
mod security;
mod settings;

pub use enums::{PromptRevisionSource, PromptType, SortField, SortOrder, SyncStatus};
pub use evaluation::*;
pub use folder::Folder;
pub use prompt::{Prompt, PromptMessage, PromptPage, PromptVersion, SearchQuery, Variable};
pub use rules::{RuleFileContent, RuleVersionSnapshot};
pub use security::StoredMasterPassword;
pub use settings::{SecuritySettings, Settings, SyncSettings};

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn enum_value_spellings_match_the_wire_contract() {
        assert_eq!(
            serde_json::to_value(PromptType::Text).unwrap(),
            json!("text")
        );
        assert_eq!(
            serde_json::to_value(PromptType::Image).unwrap(),
            json!("image")
        );
        assert_eq!(
            serde_json::to_value(PromptType::Video).unwrap(),
            json!("video")
        );

        assert_eq!(
            serde_json::to_value(SortField::Title).unwrap(),
            json!("title")
        );
        assert_eq!(
            serde_json::to_value(SortField::CreatedAt).unwrap(),
            json!("createdAt")
        );
        assert_eq!(
            serde_json::to_value(SortField::UpdatedAt).unwrap(),
            json!("updatedAt")
        );
        assert_eq!(
            serde_json::to_value(SortField::UsageCount).unwrap(),
            json!("usageCount")
        );

        assert_eq!(serde_json::to_value(SortOrder::Asc).unwrap(), json!("asc"));
        assert_eq!(
            serde_json::to_value(SortOrder::Desc).unwrap(),
            json!("desc")
        );

        assert_eq!(
            serde_json::to_value(SyncStatus::Synced).unwrap(),
            json!("synced")
        );
        assert_eq!(
            serde_json::to_value(SyncStatus::TargetMissing).unwrap(),
            json!("target-missing")
        );
        assert_eq!(
            serde_json::to_value(SyncStatus::OutOfSync).unwrap(),
            json!("out-of-sync")
        );
        assert_eq!(
            serde_json::to_value(SyncStatus::SyncError).unwrap(),
            json!("sync-error")
        );
    }

    #[test]
    fn prompt_type_defaults_to_text() {
        assert_eq!(PromptType::default(), PromptType::Text);
    }

    #[test]
    fn search_query_defaults_to_updated_at_desc() {
        assert_eq!(SortField::default(), SortField::UpdatedAt);
        assert_eq!(SortOrder::default(), SortOrder::Desc);
    }

    #[test]
    fn prompt_serializes_with_camel_case_fields() {
        let prompt = Prompt {
            id: "p1".into(),
            title: "Title".into(),
            description: None,
            prompt_type: PromptType::Text,
            system_prompt: None,
            user_prompt: "Hello {{name}}".into(),
            messages: vec![],
            variables: vec![Variable {
                name: "name".into(),
                r#type: "text".into(),
                label: None,
                default_value: None,
                options: None,
                required: true,
            }],
            tags: vec!["a".into()],
            folder_id: None,
            images: vec![],
            videos: vec![],
            is_favorite: false,
            is_pinned: false,
            is_private: false,
            is_locked: false,
            current_version: 0,
            usage_count: 0,
            source: None,
            notes: None,
            last_ai_response: None,
            created_at: "2024-01-01T00:00:00.000Z".into(),
            updated_at: "2024-01-01T00:00:00.000Z".into(),
        };
        let value = serde_json::to_value(&prompt).unwrap();
        // camelCase keys present.
        for key in [
            "promptType",
            "systemPrompt",
            "userPrompt",
            "folderId",
            "isFavorite",
            "isPinned",
            "currentVersion",
            "usageCount",
            "lastAiResponse",
            "createdAt",
            "updatedAt",
        ] {
            assert!(value.get(key).is_some(), "missing key {key}");
        }
        // Nested Variable also uses camelCase.
        let var = &value["variables"][0];
        assert!(var.get("defaultValue").is_some());

        // Round-trips back to an equal struct.
        let back: Prompt = serde_json::from_value(value).unwrap();
        assert_eq!(back, prompt);
    }

    #[test]
    fn search_query_round_trips_from_camel_case() {
        let value = json!({
            "keyword": "hi",
            "tags": ["x"],
            "folderId": "f1",
            "isFavorite": true,
            "sortBy": "usageCount",
            "sortOrder": "asc",
            "limit": 10,
            "offset": 5
        });
        let q: SearchQuery = serde_json::from_value(value).unwrap();
        assert_eq!(q.folder_id.as_deref(), Some("f1"));
        assert_eq!(q.sort_by, Some(SortField::UsageCount));
        assert_eq!(q.sort_order, Some(SortOrder::Asc));
        assert_eq!(q.limit, Some(10));
    }
}
