//! Prompt_Service: prompt CRUD operations (Requirement 6).
//!
//! This module owns the create/read/list/update/delete business rules for
//! prompts. The functions are written against a borrowed [`rusqlite::Connection`]
//! rather than reaching into global [`crate::state::AppState`], so they are
//! directly unit-testable with an in-memory pool (`storage::create_memory_pool`
//! + `storage::init_schema`) and so the Command_Layer (task 17.1) can hand them a
//!   pooled connection.
//!
//! ## Validation (no mutation on error — Req 2.3)
//!
//! Every validation runs *before* any database write, so a rejected request
//! never mutates persistent data:
//!
//! - `title` and `userPrompt` must be non-empty after trimming (Req 6.13).
//! - `promptType`, when supplied, must be one of `text` / `image` / `video`
//!   (Req 6.14); it is validated by parsing the string through the
//!   [`PromptType`] enum via serde so the wire spellings stay authoritative.
//!   When omitted on create it defaults to `text` (Req 6.6).
//!
//! ## Partial update strategy (Req 6.4)
//!
//! [`update`] uses a typed [`PromptUpdate`] patch where each field is an
//! `Option<T>`: `Some` replaces the field, `None` leaves it unchanged. The
//! implementation reads the existing prompt (returning `NOT_FOUND` when absent),
//! overlays the supplied fields, refreshes `updatedAt`, and writes the full row
//! back. Because it starts from the stored values, unsupplied fields are
//! preserved by construction (Property 7). Nullable text fields (description,
//! systemPrompt, folderId, source, notes) cannot be reset to NULL through this
//! patch shape; that is an accepted limitation for this task's scope.
//!
//! Timestamps are stored as epoch milliseconds and read back as ISO_8601 strings
//! through [`crate::storage::mapping::prompt_from_row`] (Requirement 4.9).
#![allow(dead_code)]

use std::sync::Mutex;

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::models::{
    Prompt, PromptMessage, PromptPage, PromptRevisionSource, PromptType, PromptVersion, Variable,
};
use crate::models::{SearchQuery, SortField, SortOrder};
use crate::state::EncryptionState;
use crate::storage::mapping::prompt_from_row;
use crate::storage::time::now_millis;

/// Arguments for creating a prompt (Req 6.1, 6.6, 6.7).
///
/// `title` and `userPrompt` are required; every other field is optional and
/// takes its documented default when omitted. `promptType` is accepted as a raw
/// string so an out-of-domain value can be rejected with a structured
/// `VALIDATION` error (Req 6.14) rather than failing deserialization opaquely.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PromptCreate {
    /// Prompt title (required, non-empty after trimming).
    pub title: String,
    /// User prompt body (required, non-empty after trimming).
    pub user_prompt: String,
    /// Optional free-form description.
    pub description: Option<String>,
    /// Prompt kind; defaults to `text` when omitted, validated against the domain.
    pub prompt_type: Option<String>,
    /// Optional custom organizational type whose base kind must match `promptType`.
    pub type_definition_id: Option<String>,
    /// Optional system prompt.
    pub system_prompt: Option<String>,
    /// Ordered chat messages. Empty or omitted for a simple text prompt.
    pub messages: Option<Vec<PromptMessage>>,
    /// Declared variables/placeholders.
    pub variables: Option<Vec<Variable>>,
    /// Free-form tags.
    pub tags: Option<Vec<String>>,
    /// Containing folder, or `None` for the root level.
    pub folder_id: Option<String>,
    /// Image file references.
    pub images: Option<Vec<String>>,
    /// Video file references.
    pub videos: Option<Vec<String>>,
    /// Favorite flag (default `false`).
    pub is_favorite: Option<bool>,
    /// Pinned flag (default `false`).
    pub is_pinned: Option<bool>,
    /// Encrypt content fields at rest (requires an unlocked master key).
    pub is_private: Option<bool>,
    /// Initial usage count (default `0`).
    pub usage_count: Option<i64>,
    /// Optional source URL or reference.
    pub source: Option<String>,
    /// Optional personal notes.
    pub notes: Option<String>,
}

/// Partial-update patch for a prompt (Req 6.4).
///
/// Each `Some` field replaces the stored value; each `None` field is left
/// unchanged. `promptType`, when supplied, is validated against the domain
/// (Req 6.14).
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PromptUpdate {
    /// Replacement title.
    pub title: Option<String>,
    /// Replacement user prompt.
    pub user_prompt: Option<String>,
    /// Replacement description.
    pub description: Option<String>,
    /// Replacement prompt kind (validated against the domain).
    pub prompt_type: Option<String>,
    /// Omitted preserves the reference; null clears it; a string assigns it.
    pub type_definition_id: Option<Option<String>>,
    /// Replacement system prompt.
    pub system_prompt: Option<String>,
    /// Replacement ordered chat messages.
    pub messages: Option<Vec<PromptMessage>>,
    /// Replacement variables.
    pub variables: Option<Vec<Variable>>,
    /// Replacement tags.
    pub tags: Option<Vec<String>>,
    /// Replacement folder association.
    pub folder_id: Option<String>,
    /// Replacement image references.
    pub images: Option<Vec<String>>,
    /// Replacement video references.
    pub videos: Option<Vec<String>>,
    /// Replacement favorite flag.
    pub is_favorite: Option<bool>,
    /// Replacement pinned flag.
    pub is_pinned: Option<bool>,
    /// Replacement private-content flag.
    pub is_private: Option<bool>,
    /// Replacement usage count.
    pub usage_count: Option<i64>,
    /// Replacement source.
    pub source: Option<String>,
    /// Replacement notes.
    pub notes: Option<String>,
    /// Replacement last-AI-response.
    pub last_ai_response: Option<String>,
}

/// Maps a raw rusqlite error into an `INTERNAL` [`AppError`].
fn db_err(context: &str, e: rusqlite::Error) -> AppError {
    AppError::internal(format!("{context}: {e}"))
}

/// Serializes a slice to a JSON array TEXT column value (`[]` when empty).
fn json_array<T: Serialize>(items: &[T]) -> String {
    serde_json::to_string(items).unwrap_or_else(|_| "[]".to_string())
}

/// Validates and resolves a supplied `promptType` string against the domain.
///
/// Returns a `VALIDATION` error when the value is not one of `text`, `image`,
/// or `video` (Req 6.14). Parsing goes through serde so the wire spellings in
/// [`PromptType`] remain the single source of truth.
fn parse_prompt_type(raw: &str) -> Result<PromptType, AppError> {
    serde_json::from_value::<PromptType>(serde_json::Value::String(raw.to_string())).map_err(|_| {
        AppError::validation(format!(
            "invalid promptType `{raw}`; expected one of `text`, `image`, `video`"
        ))
    })
}

/// Returns the wire-spelling stored in the `prompt_type` column for a [`PromptType`].
fn prompt_type_wire(prompt_type: PromptType) -> &'static str {
    match prompt_type {
        PromptType::Text => "text",
        PromptType::Image => "image",
        PromptType::Video => "video",
    }
}

fn resolve_prompt_type_reference(
    conn: &Connection,
    requested_prompt_type: Option<PromptType>,
    type_definition_id: Option<String>,
    fallback_prompt_type: PromptType,
) -> Result<(PromptType, Option<String>), AppError> {
    let Some(id) = type_definition_id else {
        return Ok((requested_prompt_type.unwrap_or(fallback_prompt_type), None));
    };
    let definition = crate::services::prompt_type::get(conn, &id)?;
    if requested_prompt_type.is_some_and(|prompt_type| prompt_type != definition.base_kind) {
        return Err(AppError::validation(format!(
            "promptType does not match prompt type definition `{id}`"
        )));
    }
    Ok((definition.base_kind, Some(id)))
}

fn validate_messages(messages: &[PromptMessage]) -> Result<(), AppError> {
    for (index, message) in messages.iter().enumerate() {
        if !matches!(message.role.as_str(), "system" | "user" | "assistant") {
            return Err(AppError::validation(format!(
                "messages[{index}].role must be system, user, or assistant"
            )));
        }
        if message.content.trim().is_empty() {
            return Err(AppError::validation(format!(
                "messages[{index}].content is required"
            )));
        }
    }
    Ok(())
}

/// Creates a prompt and returns the stored record (Req 6.1, 6.6, 6.7).
///
/// Validates non-empty `title`/`userPrompt` (Req 6.13) and the optional
/// `promptType` (Req 6.14) before writing. Generates a UUID identifier, sets
/// `createdAt` equal to `updatedAt` at creation (Req 6.1), defaults `promptType`
/// to `text`, and persists variables, tags, image/video references, the
/// favorite/pinned flags, usage count, source, and notes (Req 6.7).
pub fn create(conn: &Connection, input: PromptCreate) -> Result<Prompt, AppError> {
    let scan = crate::services::reference::ReferenceScan::from_create(&input);
    create_inner(conn, input, scan)
}

pub fn create_inner(
    conn: &Connection,
    input: PromptCreate,
    scan: crate::services::reference::ReferenceScan,
) -> Result<Prompt, AppError> {
    if input.title.trim().is_empty() {
        return Err(AppError::validation("title is required"));
    }
    let messages = input.messages.unwrap_or_default();
    validate_messages(&messages)?;
    if input.user_prompt.trim().is_empty() && messages.is_empty() {
        return Err(AppError::validation(
            "userPrompt or at least one chat message is required",
        ));
    }
    let requested_prompt_type = match input.prompt_type.as_deref() {
        Some(raw) => parse_prompt_type(raw)?,
        None => PromptType::Text,
    };
    let (prompt_type, type_definition_id) = resolve_prompt_type_reference(
        conn,
        input
            .prompt_type
            .as_deref()
            .map(parse_prompt_type)
            .transpose()?,
        input.type_definition_id,
        requested_prompt_type,
    )?;

    let id = uuid::Uuid::new_v4().to_string();
    let now = now_millis();
    let variables = json_array(&input.variables.unwrap_or_default());
    let messages = json_array(&messages);
    let tags = json_array(&input.tags.unwrap_or_default());
    let images = json_array(&input.images.unwrap_or_default());
    let videos = json_array(&input.videos.unwrap_or_default());

    let tx = conn
        .unchecked_transaction()
        .map_err(|e| db_err("failed to begin create-prompt transaction", e))?;
    tx.execute(
        "INSERT INTO prompts \
         (id, title, description, prompt_type, type_definition_id, system_prompt, user_prompt, messages, variables, tags, \
          folder_id, images, videos, is_favorite, is_pinned, is_private, current_version, usage_count, \
          source, notes, last_ai_response, created_at, updated_at) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23)",
        params![
            id,
            input.title,
            input.description,
            prompt_type_wire(prompt_type),
            type_definition_id,
            input.system_prompt,
            input.user_prompt,
            messages,
            variables,
            tags,
            input.folder_id,
            images,
            videos,
            input.is_favorite.unwrap_or(false),
            input.is_pinned.unwrap_or(false),
            input.is_private.unwrap_or(false),
            0_i64,
            input.usage_count.unwrap_or(0),
            input.source,
            input.notes,
            None::<String>,
            now,
            now,
        ],
    )
    .map_err(|e| db_err("failed to insert prompt", e))?;
    let created = get(&tx, &id)?;
    crate::services::version::append_snapshot(
        &tx,
        &created,
        None,
        PromptRevisionSource::Create,
        None,
    )?;
    crate::services::reference::resolve_and_store(&tx, &id, &scan)?;
    tx.commit()
        .map_err(|e| db_err("failed to commit create-prompt transaction", e))?;
    get(conn, &id)
}

/// Fetches a prompt by identifier (Req 6.2), returning `NOT_FOUND` when absent
/// (Req 6.12).
pub fn get(conn: &Connection, id: &str) -> Result<Prompt, AppError> {
    conn.query_row("SELECT * FROM prompts WHERE id = ?1", [id], prompt_from_row)
        .optional()
        .map_err(|e| db_err("failed to read prompt", e))?
        .ok_or_else(|| AppError::not_found(format!("prompt `{id}` not found")))
}

/// Returns all stored prompts (Req 6.3), or an empty vector when none exist.
///
/// Ordered by creation time ascending for a stable, intuitive listing.
pub fn list(conn: &Connection) -> Result<Vec<Prompt>, AppError> {
    let mut stmt = conn
        .prepare("SELECT * FROM prompts ORDER BY created_at ASC, id ASC")
        .map_err(|e| db_err("failed to prepare prompt list", e))?;
    let rows = stmt
        .query_map([], prompt_from_row)
        .map_err(|e| db_err("failed to query prompts", e))?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| db_err("failed to map prompt rows", e))
}

/// Applies a partial update to an existing prompt and returns it (Req 6.4).
///
/// Supplied fields replace their stored values; unsupplied fields are preserved.
/// `updatedAt` is always refreshed to the current time. Returns `NOT_FOUND` when
/// the prompt does not exist (Req 6.12); a supplied invalid `promptType` is
/// rejected with `VALIDATION` before any write (Req 6.14).
pub fn update(conn: &Connection, id: &str, patch: PromptUpdate) -> Result<Prompt, AppError> {
    let existing = get(conn, id)?;
    let scan = crate::services::reference::ReferenceScan::from_update(&patch, &existing);
    update_inner(conn, id, patch, scan)
}

pub fn update_inner(
    conn: &Connection,
    id: &str,
    patch: PromptUpdate,
    scan: crate::services::reference::ReferenceScan,
) -> Result<Prompt, AppError> {
    // Validate the optional promptType before touching the database so a rejected
    // request never mutates stored data (Req 2.3).
    let new_prompt_type = match patch.prompt_type.as_deref() {
        Some(raw) => Some(parse_prompt_type(raw)?),
        None => None,
    };

    // NOT_FOUND when the prompt does not exist; also the base for preserved fields.
    let existing = get(conn, id)?;

    let title = patch.title.unwrap_or_else(|| existing.title.clone());
    let description = patch.description.or_else(|| existing.description.clone());
    let type_definition_id = patch
        .type_definition_id
        .unwrap_or_else(|| existing.type_definition_id.clone());
    let (prompt_type, type_definition_id) = resolve_prompt_type_reference(
        conn,
        new_prompt_type,
        type_definition_id,
        existing.prompt_type,
    )?;
    let system_prompt = patch
        .system_prompt
        .or_else(|| existing.system_prompt.clone());
    let user_prompt = patch
        .user_prompt
        .unwrap_or_else(|| existing.user_prompt.clone());
    let messages = patch.messages.unwrap_or_else(|| existing.messages.clone());
    validate_messages(&messages)?;
    if user_prompt.trim().is_empty() && messages.is_empty() {
        return Err(AppError::validation(
            "userPrompt or at least one chat message is required",
        ));
    }
    let variables = patch
        .variables
        .unwrap_or_else(|| existing.variables.clone());
    let tags = patch.tags.unwrap_or_else(|| existing.tags.clone());
    let folder_id = patch.folder_id.or_else(|| existing.folder_id.clone());
    let images = patch.images.unwrap_or_else(|| existing.images.clone());
    let videos = patch.videos.unwrap_or_else(|| existing.videos.clone());
    let is_favorite = patch.is_favorite.unwrap_or(existing.is_favorite);
    let is_pinned = patch.is_pinned.unwrap_or(existing.is_pinned);
    let is_private = patch.is_private.unwrap_or(existing.is_private);
    let usage_count = patch.usage_count.unwrap_or(existing.usage_count);
    let source = patch.source.or_else(|| existing.source.clone());
    let notes = patch.notes.or_else(|| existing.notes.clone());
    let last_ai_response = patch
        .last_ai_response
        .or_else(|| existing.last_ai_response.clone());

    let revision_changed = title != existing.title
        || description != existing.description
        || prompt_type != existing.prompt_type
        || type_definition_id != existing.type_definition_id
        || system_prompt != existing.system_prompt
        || user_prompt != existing.user_prompt
        || messages != existing.messages
        || variables != existing.variables
        || tags != existing.tags
        || folder_id != existing.folder_id
        || images != existing.images
        || videos != existing.videos
        || is_favorite != existing.is_favorite
        || is_pinned != existing.is_pinned
        || is_private != existing.is_private
        || source != existing.source
        || notes != existing.notes
        || last_ai_response != existing.last_ai_response;
    if !revision_changed && usage_count == existing.usage_count {
        return Ok(existing);
    }

    let now = now_millis();
    let tx = conn
        .unchecked_transaction()
        .map_err(|e| db_err("failed to begin update-prompt transaction", e))?;
    tx.execute(
        "UPDATE prompts SET \
         title=?1, description=?2, prompt_type=?3, type_definition_id=?4, system_prompt=?5, user_prompt=?6, messages=?7, \
         variables=?8, tags=?9, folder_id=?10, images=?11, videos=?12, is_favorite=?13, \
         is_pinned=?14, is_private=?15, usage_count=?16, source=?17, notes=?18, last_ai_response=?19, \
         updated_at=?20 \
         WHERE id=?21",
        params![
            title,
            description,
            prompt_type_wire(prompt_type),
            type_definition_id,
            system_prompt,
            user_prompt,
            json_array(&messages),
            json_array(&variables),
            json_array(&tags),
            folder_id,
            json_array(&images),
            json_array(&videos),
            is_favorite,
            is_pinned,
            is_private,
            usage_count,
            source,
            notes,
            last_ai_response,
            now,
            id,
        ],
    )
    .map_err(|e| db_err("failed to update prompt", e))?;
    let updated = get(&tx, id)?;
    if revision_changed {
        crate::services::version::append_snapshot(
            &tx,
            &updated,
            None,
            PromptRevisionSource::Save,
            None,
        )?;
    }
    crate::services::reference::resolve_and_store(&tx, id, &scan)?;
    tx.commit()
        .map_err(|e| db_err("failed to commit update-prompt transaction", e))?;
    get(conn, id)
}

fn required_private_key(encryption: &Mutex<EncryptionState>) -> Result<Vec<u8>, AppError> {
    crate::services::security::unlocked_key(encryption)?.ok_or_else(|| {
        AppError::unauthorized("unlock the prompt library to access private content")
    })
}

fn encrypt_optional(value: Option<String>, key: &[u8]) -> Result<Option<String>, AppError> {
    value
        .map(|value| crate::services::security::encrypt(&value, key))
        .transpose()
}

fn decrypt_optional(value: Option<String>, key: &[u8]) -> Result<Option<String>, AppError> {
    value
        .map(|value| crate::services::security::decrypt(&value, key))
        .transpose()
}

fn encrypt_messages(
    messages: Vec<PromptMessage>,
    key: &[u8],
) -> Result<Vec<PromptMessage>, AppError> {
    messages
        .into_iter()
        .map(|mut message| {
            message.content = crate::services::security::encrypt(&message.content, key)?;
            Ok(message)
        })
        .collect()
}

fn decrypt_messages(
    messages: Vec<PromptMessage>,
    key: &[u8],
) -> Result<Vec<PromptMessage>, AppError> {
    messages
        .into_iter()
        .map(|mut message| {
            message.content = crate::services::security::decrypt(&message.content, key)?;
            Ok(message)
        })
        .collect()
}

pub(crate) fn present_prompt(mut prompt: Prompt, key: Option<&[u8]>) -> Result<Prompt, AppError> {
    if !prompt.is_private {
        return Ok(prompt);
    }
    let Some(key) = key else {
        prompt.description = None;
        prompt.system_prompt = None;
        prompt.user_prompt.clear();
        prompt.messages.clear();
        prompt.source = None;
        prompt.notes = None;
        prompt.last_ai_response = None;
        prompt.is_locked = true;
        return Ok(prompt);
    };

    prompt.description = decrypt_optional(prompt.description, key)?;
    prompt.system_prompt = decrypt_optional(prompt.system_prompt, key)?;
    prompt.user_prompt = crate::services::security::decrypt(&prompt.user_prompt, key)?;
    prompt.messages = decrypt_messages(prompt.messages, key)?;
    prompt.source = decrypt_optional(prompt.source, key)?;
    prompt.notes = decrypt_optional(prompt.notes, key)?;
    prompt.last_ai_response = decrypt_optional(prompt.last_ai_response, key)?;
    prompt.is_locked = false;
    Ok(prompt)
}

pub(crate) fn present_version(
    mut version: PromptVersion,
    key: Option<&[u8]>,
) -> Result<PromptVersion, AppError> {
    if !version.is_private {
        return Ok(version);
    }
    let Some(key) = key else {
        version.description = None;
        version.system_prompt = None;
        version.user_prompt.clear();
        version.messages.clear();
        version.source = None;
        version.notes = None;
        version.ai_response = None;
        return Ok(version);
    };

    version.description = decrypt_optional(version.description, key)?;
    version.system_prompt = decrypt_optional(version.system_prompt, key)?;
    version.user_prompt = crate::services::security::decrypt(&version.user_prompt, key)?;
    version.messages = decrypt_messages(version.messages, key)?;
    version.source = decrypt_optional(version.source, key)?;
    version.notes = decrypt_optional(version.notes, key)?;
    version.ai_response = decrypt_optional(version.ai_response, key)?;
    Ok(version)
}

pub fn create_secure(
    conn: &Connection,
    encryption: &Mutex<EncryptionState>,
    mut input: PromptCreate,
) -> Result<Prompt, AppError> {
    let scan = crate::services::reference::ReferenceScan::from_create(&input);
    if input.is_private.unwrap_or(false) {
        let key = required_private_key(encryption)?;
        input.description = encrypt_optional(input.description, &key)?;
        input.system_prompt = encrypt_optional(input.system_prompt, &key)?;
        input.user_prompt = crate::services::security::encrypt(&input.user_prompt, &key)?;
        input.messages = Some(encrypt_messages(input.messages.unwrap_or_default(), &key)?);
        input.source = encrypt_optional(input.source, &key)?;
        input.notes = encrypt_optional(input.notes, &key)?;
    }
    let stored = create_inner(conn, input, scan)?;
    let key = crate::services::security::unlocked_key(encryption)?;
    present_prompt(stored, key.as_deref())
}

pub fn update_secure(
    conn: &Connection,
    encryption: &Mutex<EncryptionState>,
    id: &str,
    mut patch: PromptUpdate,
) -> Result<Prompt, AppError> {
    let existing = get_secure(conn, encryption, id)?;
    let scan = crate::services::reference::ReferenceScan::from_update(&patch, &existing);
    let existing = get(conn, id)?;
    let desired_private = patch.is_private.unwrap_or(existing.is_private);

    match (existing.is_private, desired_private) {
        (false, true) => {
            let key = required_private_key(encryption)?;
            patch.description = encrypt_optional(
                patch.description.take().or(existing.description.clone()),
                &key,
            )?;
            patch.system_prompt = encrypt_optional(
                patch
                    .system_prompt
                    .take()
                    .or(existing.system_prompt.clone()),
                &key,
            )?;
            patch.user_prompt = Some(crate::services::security::encrypt(
                patch
                    .user_prompt
                    .as_deref()
                    .unwrap_or(&existing.user_prompt),
                &key,
            )?);
            patch.messages = Some(encrypt_messages(
                patch.messages.take().unwrap_or(existing.messages.clone()),
                &key,
            )?);
            patch.source = encrypt_optional(patch.source.take().or(existing.source.clone()), &key)?;
            patch.notes = encrypt_optional(patch.notes.take().or(existing.notes.clone()), &key)?;
            patch.last_ai_response = encrypt_optional(
                patch
                    .last_ai_response
                    .take()
                    .or(existing.last_ai_response.clone()),
                &key,
            )?;
        }
        (true, false) => {
            let key = required_private_key(encryption)?;
            let current = present_prompt(existing.clone(), Some(&key))?;
            patch.description = patch.description.or(current.description);
            patch.system_prompt = patch.system_prompt.or(current.system_prompt);
            patch.user_prompt = Some(patch.user_prompt.unwrap_or(current.user_prompt));
            patch.messages = Some(patch.messages.unwrap_or(current.messages));
            patch.source = patch.source.or(current.source);
            patch.notes = patch.notes.or(current.notes);
            patch.last_ai_response = patch.last_ai_response.or(current.last_ai_response);
        }
        (true, true) => {
            let touches_content = patch.description.is_some()
                || patch.system_prompt.is_some()
                || patch.user_prompt.is_some()
                || patch.messages.is_some()
                || patch.source.is_some()
                || patch.notes.is_some()
                || patch.last_ai_response.is_some();
            if touches_content {
                let key = required_private_key(encryption)?;
                patch.description = encrypt_optional(patch.description, &key)?;
                patch.system_prompt = encrypt_optional(patch.system_prompt, &key)?;
                patch.user_prompt = patch
                    .user_prompt
                    .map(|value| crate::services::security::encrypt(&value, &key))
                    .transpose()?;
                patch.messages = patch
                    .messages
                    .map(|messages| encrypt_messages(messages, &key))
                    .transpose()?;
                patch.source = encrypt_optional(patch.source, &key)?;
                patch.notes = encrypt_optional(patch.notes, &key)?;
                patch.last_ai_response = encrypt_optional(patch.last_ai_response, &key)?;
            }
        }
        (false, false) => {}
    }

    let stored = update_inner(conn, id, patch, scan)?;
    let key = crate::services::security::unlocked_key(encryption)?;
    present_prompt(stored, key.as_deref())
}

pub fn get_secure(
    conn: &Connection,
    encryption: &Mutex<EncryptionState>,
    id: &str,
) -> Result<Prompt, AppError> {
    let key = crate::services::security::unlocked_key(encryption)?;
    present_prompt(get(conn, id)?, key.as_deref())
}

pub fn list_secure(
    conn: &Connection,
    encryption: &Mutex<EncryptionState>,
) -> Result<Vec<Prompt>, AppError> {
    let key = crate::services::security::unlocked_key(encryption)?;
    list(conn)?
        .into_iter()
        .map(|prompt| present_prompt(prompt, key.as_deref()))
        .collect()
}

pub fn search_secure(
    conn: &Connection,
    encryption: &Mutex<EncryptionState>,
    query: SearchQuery,
) -> Result<PromptPage, AppError> {
    let key = crate::services::security::unlocked_key(encryption)?;
    let mut page = search(conn, query)?;
    page.items = page
        .items
        .into_iter()
        .map(|prompt| present_prompt(prompt, key.as_deref()))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(page)
}

/// Deletes a prompt by identifier (Req 6.5).
///
/// The `ON DELETE CASCADE` foreign key on `prompt_versions` removes the prompt's
/// version history as part of the same delete (Req 4.4). Returns `NOT_FOUND`
/// when the prompt does not exist (Req 6.12).
pub fn delete(conn: &Connection, id: &str) -> Result<(), AppError> {
    get(conn, id)?;
    let tx = conn
        .unchecked_transaction()
        .map_err(|e| db_err("failed to begin delete-prompt transaction", e))?;
    crate::services::reference::mark_incoming_missing(&tx, id)?;
    let affected = tx
        .execute("DELETE FROM prompts WHERE id = ?1", [id])
        .map_err(|e| db_err("failed to delete prompt", e))?;
    if affected == 0 {
        return Err(AppError::not_found(format!("prompt `{id}` not found")));
    }
    tx.commit()
        .map_err(|e| db_err("failed to commit delete-prompt transaction", e))?;
    Ok(())
}

/// Clones a prompt, preserving private ciphertext and starting new revision history.
pub fn duplicate(conn: &Connection, id: &str) -> Result<Prompt, AppError> {
    let source = get(conn, id)?;
    let duplicate_id = uuid::Uuid::new_v4().to_string();
    let now = now_millis();
    let title = format!("{} copy", source.title);
    let tx = conn
        .unchecked_transaction()
        .map_err(|error| db_err("failed to begin duplicate transaction", error))?;
    tx.execute(
        "INSERT INTO prompts (id,title,description,prompt_type,type_definition_id,system_prompt,user_prompt,messages,variables,tags,folder_id,images,videos,is_favorite,is_pinned,is_private,current_version,usage_count,source,notes,last_ai_response,created_at,updated_at) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,0,0,?17,?18,?19,?20,?20)",
        params![
            duplicate_id,
            title,
            source.description,
            prompt_type_wire(source.prompt_type),
            source.type_definition_id,
            source.system_prompt,
            source.user_prompt,
            json_array(&source.messages),
            json_array(&source.variables),
            json_array(&source.tags),
            source.folder_id,
            json_array(&source.images),
            json_array(&source.videos),
            source.is_favorite,
            source.is_pinned,
            source.is_private,
            source.source,
            source.notes,
            source.last_ai_response,
            now,
        ],
    )
    .map_err(|error| db_err("failed to duplicate prompt", error))?;
    let duplicated = get(&tx, &duplicate_id)?;
    crate::services::version::append_snapshot(
        &tx,
        &duplicated,
        Some(format!("Duplicated from {}", source.id)),
        PromptRevisionSource::Create,
        None,
    )?;
    if source.is_private {
        crate::services::reference::copy_outgoing(&tx, &source.id, &duplicate_id)?;
    } else {
        crate::services::reference::resolve_and_store(
            &tx,
            &duplicate_id,
            &crate::services::reference::ReferenceScan::from_prompt(&source),
        )?;
    }
    tx.commit()
        .map_err(|error| db_err("failed to commit duplicate transaction", error))?;
    get(conn, &duplicate_id)
}

fn validate_batch(conn: &Connection, ids: &[String]) -> Result<Vec<Prompt>, AppError> {
    let mut unique = std::collections::HashSet::new();
    let mut prompts = Vec::new();
    for id in ids {
        if unique.insert(id) {
            prompts.push(get(conn, id)?);
        }
    }
    Ok(prompts)
}

pub fn batch_move(
    conn: &Connection,
    ids: &[String],
    folder_id: Option<&str>,
) -> Result<(), AppError> {
    let prompts = validate_batch(conn, ids)?;
    let tx = conn
        .unchecked_transaction()
        .map_err(|error| db_err("failed to begin batch move", error))?;
    for prompt in prompts {
        if prompt.folder_id.as_deref() == folder_id {
            continue;
        }
        tx.execute(
            "UPDATE prompts SET folder_id = ?1, updated_at = ?2 WHERE id = ?3",
            params![folder_id, now_millis(), prompt.id],
        )
        .map_err(|error| db_err("failed to move prompt", error))?;
        let updated = get(&tx, &prompt.id)?;
        crate::services::version::append_snapshot(
            &tx,
            &updated,
            Some("Batch folder move".into()),
            PromptRevisionSource::Save,
            None,
        )?;
    }
    tx.commit()
        .map_err(|error| db_err("failed to commit batch move", error))
}

pub fn batch_tag(conn: &Connection, ids: &[String], tags: &[String]) -> Result<(), AppError> {
    let prompts = validate_batch(conn, ids)?;
    let tx = conn
        .unchecked_transaction()
        .map_err(|error| db_err("failed to begin batch tag", error))?;
    for prompt in prompts {
        let mut next = prompt.tags.clone();
        for tag in tags
            .iter()
            .map(|tag| tag.trim())
            .filter(|tag| !tag.is_empty())
        {
            if !next.iter().any(|existing| existing == tag) {
                next.push(tag.to_string());
            }
        }
        if next == prompt.tags {
            continue;
        }
        tx.execute(
            "UPDATE prompts SET tags = ?1, updated_at = ?2 WHERE id = ?3",
            params![json_array(&next), now_millis(), prompt.id],
        )
        .map_err(|error| db_err("failed to tag prompt", error))?;
        let updated = get(&tx, &prompt.id)?;
        crate::services::version::append_snapshot(
            &tx,
            &updated,
            Some("Batch tag update".into()),
            PromptRevisionSource::Save,
            None,
        )?;
    }
    tx.commit()
        .map_err(|error| db_err("failed to commit batch tag", error))
}

pub fn batch_delete(conn: &Connection, ids: &[String]) -> Result<(), AppError> {
    let prompts = validate_batch(conn, ids)?;
    let tx = conn
        .unchecked_transaction()
        .map_err(|error| db_err("failed to begin batch delete", error))?;
    for prompt in prompts {
        crate::services::reference::mark_incoming_missing(&tx, &prompt.id)?;
        tx.execute("DELETE FROM prompts WHERE id = ?1", [&prompt.id])
            .map_err(|error| db_err("failed to delete prompt", error))?;
    }
    tx.commit()
        .map_err(|error| db_err("failed to commit batch delete", error))
}

// --- search (task 4.3; Req 5.3–5.10) ---------------------------------------

/// Builds a safe FTS5 `MATCH` expression from a raw keyword (Req 5.3, 5.7).
///
/// The keyword is split on whitespace and each token is wrapped in double quotes
/// (with any embedded double quote doubled, the FTS5 escape) so it is treated as
/// a literal phrase rather than an operator. The quoted phrases are joined with a
/// space, which FTS5 interprets as implicit AND, so every token must be present.
/// This neutralizes query operators and special characters (`*`, `:`, `(`, `^`,
/// `OR`, etc.) by quoting them.
///
/// Returns `None` when the keyword is empty or whitespace-only, signalling that no
/// FTS constraint should be applied. The resulting expression is still bound as a
/// SQL parameter (never interpolated); if FTS5 cannot parse it (for example a
/// phrase that tokenizes to nothing), [`search`] catches the step error and
/// returns a structured parse error instead of panicking (Req 5.7).
fn build_fts_match(keyword: &str) -> Option<String> {
    let tokens: Vec<String> = keyword
        .split_whitespace()
        .map(|token| format!("\"{}\"", token.replace('"', "\"\"")))
        .collect();
    if tokens.is_empty() {
        None
    } else {
        Some(tokens.join(" "))
    }
}

/// Maps the SQL column a [`SortField`] sorts by. The value is a fixed string from
/// a closed whitelist, so it is safe to assemble into the `ORDER BY` clause.
fn sort_column(field: SortField) -> &'static str {
    match field {
        SortField::Title => "prompts.title",
        SortField::CreatedAt => "prompts.created_at",
        SortField::UpdatedAt => "prompts.updated_at",
        SortField::UsageCount => "prompts.usage_count",
    }
}

/// Maps a [`SortOrder`] to its SQL direction keyword (fixed whitelist).
fn sort_direction(order: SortOrder) -> &'static str {
    match order {
        SortOrder::Asc => "ASC",
        SortOrder::Desc => "DESC",
    }
}

/// Searches prompts with full-text keyword, conjunctive filters, sorting, and
/// pagination (Requirements 5.3–5.10).
///
/// - **Keyword (5.3, 5.7):** a non-empty keyword is turned into a quoted FTS5
///   `MATCH` expression by [`build_fts_match`] and bound as a parameter against
///   the `prompts_fts` index; matching is case-insensitive (the FTS5 tokenizer
///   lowercases). An empty/whitespace keyword applies no FTS constraint. If FTS5
///   cannot parse the expression, a structured `PARSE` error is returned rather
///   than panicking.
/// - **Filters (5.4):** `tags`, `folderId`, and `isFavorite` combine with any
///   keyword using conjunctive `AND` logic. Each required tag must be present in
///   the prompt's JSON tags array (exact membership via `json_each`, so a tag is
///   not matched as a substring of another).
/// - **Sorting (5.5, 5.8):** orders by the requested field/direction; a missing
///   sort field or order falls back to `updatedAt` descending.
/// - **Pagination (5.6, 5.9):** the limit is clamped to `1..=100` (default 50)
///   and the offset defaults to 0.
///
/// All values are bound as SQL parameters; only the `ORDER BY` column/direction
/// (from a closed whitelist) are assembled into the statement text.
pub fn search(conn: &Connection, query: SearchQuery) -> Result<PromptPage, AppError> {
    let match_expr = query.keyword.as_deref().and_then(build_fts_match);
    let has_keyword = match_expr.is_some();

    let limit = query.limit.map(|l| l.clamp(1, 100)).unwrap_or(50);
    let offset = query.offset.unwrap_or(0);

    let mut from_sql = String::from(" FROM prompts");
    if has_keyword {
        from_sql.push_str(" JOIN prompts_fts ON prompts_fts.rowid = prompts.rowid");
    }

    let mut clauses: Vec<&str> = Vec::new();
    let mut params: Vec<rusqlite::types::Value> = Vec::new();

    if let Some(expr) = match_expr {
        clauses.push("prompts_fts MATCH ?");
        params.push(rusqlite::types::Value::Text(expr));
    }
    if let Some(tags) = &query.tags {
        for tag in tags {
            // Exact membership: a tag matches only when it equals an element of
            // the JSON tags array, never as a substring of another tag.
            clauses.push("EXISTS (SELECT 1 FROM json_each(prompts.tags) WHERE value = ?)");
            params.push(rusqlite::types::Value::Text(tag.clone()));
        }
    }
    if let Some(folder_id) = &query.folder_id {
        clauses.push("prompts.folder_id = ?");
        params.push(rusqlite::types::Value::Text(folder_id.clone()));
    }
    if let Some(is_favorite) = query.is_favorite {
        clauses.push("prompts.is_favorite = ?");
        params.push(rusqlite::types::Value::Integer(i64::from(is_favorite)));
    }
    if !clauses.is_empty() {
        from_sql.push_str(" WHERE ");
        from_sql.push_str(&clauses.join(" AND "));
    }

    let count_sql = format!("SELECT COUNT(*){from_sql}");
    let total: i64 = conn
        .query_row(
            &count_sql,
            rusqlite::params_from_iter(params.iter()),
            |row| row.get(0),
        )
        .map_err(|e| {
            if has_keyword {
                AppError::parse(format!("search keyword could not be parsed: {e}"))
            } else {
                db_err("failed to count search results", e)
            }
        })?;

    let field = query.sort_by.unwrap_or_default();
    let order = query.sort_order.unwrap_or_default();
    let mut sql = format!("SELECT prompts.*{from_sql}");
    // `prompts.id` is a stable secondary key so equal sort values order
    // deterministically.
    sql.push_str(&format!(
        " ORDER BY {} {}, prompts.id ASC LIMIT ? OFFSET ?",
        sort_column(field),
        sort_direction(order)
    ));
    params.push(rusqlite::types::Value::Integer(i64::from(limit)));
    params.push(rusqlite::types::Value::Integer(i64::from(offset)));

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| db_err("failed to prepare prompt search", e))?;
    let mut rows = stmt
        .query(rusqlite::params_from_iter(params.iter()))
        .map_err(|e| db_err("failed to execute prompt search", e))?;

    let mut out = Vec::new();
    loop {
        // The FTS5 expression is parsed lazily on the first step, so a malformed
        // keyword surfaces here as a step error — classified as a structured parse
        // error rather than allowed to panic (Req 5.7).
        match rows.next() {
            Ok(Some(row)) => {
                out.push(prompt_from_row(row).map_err(|e| db_err("failed to map prompt row", e))?)
            }
            Ok(None) => break,
            Err(e) if has_keyword => {
                return Err(AppError::parse(format!(
                    "search keyword could not be parsed: {e}"
                )))
            }
            Err(e) => return Err(db_err("failed to read search results", e)),
        }
    }
    let total =
        u64::try_from(total).map_err(|_| AppError::internal("search result count was negative"))?;
    let has_more = u64::from(offset) + (out.len() as u64) < total;
    Ok(PromptPage {
        items: out,
        total,
        limit,
        offset,
        has_more,
    })
}

// --- copy + tag operations (task 4.2; Req 6.8–6.11) ------------------------

/// Substituted prompt text returned by [`copy`] (Req 6.11).
///
/// Mirrors the Reference_App's resolved-content shape: the system prompt is
/// optional, the user prompt is always present, and placeholders have been
/// substituted in each. The Frontend decides how to combine them for the
/// clipboard.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptCopy {
    /// System prompt with placeholders substituted, when the prompt has one.
    pub system_prompt: Option<String>,
    /// User prompt with placeholders substituted.
    pub user_prompt: String,
    /// Chat messages after expansion then substitution.
    #[serde(default)]
    pub messages: Vec<crate::models::PromptMessage>,
    /// Tokens that could not be expanded.
    #[serde(default)]
    pub unexpanded: Vec<crate::services::reference::UnexpandedReference>,
}

/// Substitutes `{{name}}` placeholders in `text` using `values` (Req 6.11).
///
/// A placeholder is a `{{ ... }}` span; the part before an optional `:default`
/// segment, trimmed of surrounding whitespace, is treated as the variable name
/// (so `{{name}}`, `{{ name }}`, and `{{name:default}}` all resolve to `name`,
/// matching the Reference_App's substitution regex). When the name has a value
/// in `values`, the whole placeholder is replaced by that value; otherwise the
/// placeholder is copied through unchanged. An unterminated `{{` is also left
/// intact.
pub(crate) fn substitute_placeholders(
    text: &str,
    values: &std::collections::HashMap<String, String>,
) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(open) = rest.find("{{") {
        out.push_str(&rest[..open]);
        let after_open = &rest[open + 2..];
        match after_open.find("}}") {
            Some(close) => {
                let inner = &after_open[..close];
                let name = inner.split(':').next().unwrap_or("").trim();
                match values.get(name) {
                    Some(value) => out.push_str(value),
                    // No supplied value: leave the placeholder exactly as written.
                    None => {
                        out.push_str("{{");
                        out.push_str(inner);
                        out.push_str("}}");
                    }
                }
                rest = &after_open[close + 2..];
            }
            // Unterminated `{{`: copy it literally and stop scanning for more.
            None => {
                out.push_str("{{");
                out.push_str(after_open);
                rest = "";
                break;
            }
        }
    }
    out.push_str(rest);
    out
}

/// Returns the substituted system/user prompt text for a prompt (Req 6.11).
///
/// Fetches the prompt (returning `NOT_FOUND` when absent — Req 6.12) and applies
/// [`substitute_placeholders`] to both its system and user prompt, leaving any
/// placeholder without a supplied value unchanged. This is a read-only operation
/// and never mutates stored data.
pub fn copy(
    conn: &Connection,
    id: &str,
    values: &std::collections::HashMap<String, String>,
) -> Result<PromptCopy, AppError> {
    let encryption = Mutex::new(EncryptionState {
        derived_key: None,
        locked: false,
    });
    copy_secure(conn, &encryption, id, values)
}

pub fn copy_secure(
    conn: &Connection,
    encryption: &Mutex<EncryptionState>,
    id: &str,
    values: &std::collections::HashMap<String, String>,
) -> Result<PromptCopy, AppError> {
    let prompt = get_secure(conn, encryption, id)?;
    if prompt.is_locked {
        return Err(AppError::unauthorized(
            "unlock the prompt library to copy private content",
        ));
    }
    // Expand @@references first, then substitute {{placeholders}} once over the
    // assembled text, including chat messages. Expanded bodies use this prompt's
    // values, not the referenced prompt's declared defaults.
    let (expanded, unexpanded) =
        crate::services::reference::expand_copy(conn, encryption, &prompt)?;
    Ok(PromptCopy {
        system_prompt: expanded
            .system_prompt
            .as_deref()
            .map(|text| substitute_placeholders(text, values)),
        user_prompt: substitute_placeholders(&expanded.user_prompt, values),
        messages: expanded
            .messages
            .into_iter()
            .map(|message| crate::models::PromptMessage {
                role: message.role,
                content: substitute_placeholders(&message.content, values),
            })
            .collect(),
        unexpanded,
    })
}

/// Reads every prompt's `(id, tags)` pair, parsing the stored JSON tag arrays.
///
/// Tag columns are written as JSON arrays by [`json_array`]; a value that fails
/// to parse is treated as an empty tag list so a single malformed row cannot
/// break tag aggregation.
fn tag_rows(conn: &Connection) -> Result<Vec<(String, Vec<String>)>, AppError> {
    let mut stmt = conn
        .prepare("SELECT id, tags FROM prompts")
        .map_err(|e| db_err("failed to prepare tag query", e))?;
    let rows = stmt
        .query_map([], |row| {
            let id: String = row.get(0)?;
            let tags_json: String = row.get(1)?;
            Ok((id, tags_json))
        })
        .map_err(|e| db_err("failed to query tags", e))?;

    let mut out = Vec::new();
    for row in rows {
        let (id, tags_json) = row.map_err(|e| db_err("failed to read tag row", e))?;
        let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
        out.push((id, tags));
    }
    Ok(out)
}

/// Returns the distinct set of tags across all prompts, sorted (Req 6.8).
///
/// Flattens every prompt's tag array, deduplicates, and returns the result in
/// sorted order for a stable listing; returns an empty vector when no tags exist.
pub fn tag_list(conn: &Connection) -> Result<Vec<String>, AppError> {
    use std::collections::HashSet;

    let mut distinct: HashSet<String> = HashSet::new();
    for (_, tags) in tag_rows(conn)? {
        distinct.extend(tags);
    }
    let mut result: Vec<String> = distinct.into_iter().collect();
    result.sort();
    Ok(result)
}

/// Replaces tag `old` with `new` on every prompt that carries `old`, without
/// creating a duplicate on prompts that already carry `new` (Req 6.9).
///
/// Only prompts that actually carry `old` are rewritten, so the operation is
/// idempotent (a second call finds no `old` tag and changes nothing) and bumps
/// `updatedAt` only on prompts it modifies. When `old == new` the call is a
/// no-op. All writes run in a single transaction so a mid-run failure rolls back
/// fully (Req 4.8).
pub fn tag_rename(conn: &Connection, old: &str, new: &str) -> Result<(), AppError> {
    use std::collections::HashSet;

    if old == new {
        return Ok(());
    }

    let rows = tag_rows(conn)?;
    let tx = conn
        .unchecked_transaction()
        .map_err(|e| db_err("failed to begin tag rename transaction", e))?;
    let now = now_millis();

    for (id, tags) in rows {
        if !tags.iter().any(|t| t == old) {
            continue;
        }
        // Map old -> new and deduplicate while preserving first-seen order, so a
        // prompt that already carries `new` ends up with a single `new` tag.
        let mut seen: HashSet<String> = HashSet::new();
        let mut updated: Vec<String> = Vec::with_capacity(tags.len());
        for tag in tags {
            let mapped = if tag == old { new.to_string() } else { tag };
            if seen.insert(mapped.clone()) {
                updated.push(mapped);
            }
        }
        tx.execute(
            "UPDATE prompts SET tags = ?1, updated_at = ?2 WHERE id = ?3",
            params![json_array(&updated), now, id],
        )
        .map_err(|e| db_err("failed to rename tag on prompt", e))?;
        let prompt = get(&tx, &id)?;
        crate::services::version::append_snapshot(
            &tx,
            &prompt,
            Some(format!("Renamed tag {old} to {new}")),
            PromptRevisionSource::Save,
            None,
        )?;
    }

    tx.commit()
        .map_err(|e| db_err("failed to commit tag rename transaction", e))?;
    Ok(())
}

/// Removes tag `tag` from every prompt that carries it (Req 6.10).
///
/// Only prompts that carry the tag are rewritten. All writes run in a single
/// transaction so a mid-run failure rolls back fully (Req 4.8).
pub fn tag_delete(conn: &Connection, tag: &str) -> Result<(), AppError> {
    let rows = tag_rows(conn)?;
    let tx = conn
        .unchecked_transaction()
        .map_err(|e| db_err("failed to begin tag delete transaction", e))?;
    let now = now_millis();

    for (id, tags) in rows {
        if !tags.iter().any(|t| t == tag) {
            continue;
        }
        let updated: Vec<String> = tags.into_iter().filter(|t| t != tag).collect();
        tx.execute(
            "UPDATE prompts SET tags = ?1, updated_at = ?2 WHERE id = ?3",
            params![json_array(&updated), now, id],
        )
        .map_err(|e| db_err("failed to delete tag on prompt", e))?;
        let prompt = get(&tx, &id)?;
        crate::services::version::append_snapshot(
            &tx,
            &prompt,
            Some(format!("Deleted tag {tag}")),
            PromptRevisionSource::Save,
            None,
        )?;
    }

    tx.commit()
        .map_err(|e| db_err("failed to commit tag delete transaction", e))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorCode;
    use crate::storage::{create_memory_pool, init_schema, DbPool};
    use rusqlite::params;

    /// Builds an in-memory pool with the schema initialized.
    fn schema_pool() -> DbPool {
        let pool = create_memory_pool().expect("memory pool");
        init_schema(&pool.get().expect("conn")).expect("schema");
        pool
    }

    fn sample_create() -> PromptCreate {
        PromptCreate {
            title: "My Prompt".into(),
            user_prompt: "Hello {{name}}".into(),
            description: Some("a description".into()),
            prompt_type: None,
            type_definition_id: None,
            system_prompt: Some("be helpful".into()),
            messages: None,
            variables: Some(vec![Variable {
                name: "name".into(),
                r#type: "text".into(),
                label: Some("Name".into()),
                default_value: None,
                options: None,
                required: true,
            }]),
            tags: Some(vec!["a".into(), "b".into()]),
            folder_id: None,
            images: Some(vec!["img1.png".into()]),
            videos: Some(vec![]),
            is_favorite: Some(true),
            is_pinned: Some(false),
            is_private: Some(false),
            usage_count: Some(3),
            source: Some("https://example.com".into()),
            notes: Some("note".into()),
        }
    }

    #[test]
    fn create_then_get_round_trips_all_fields() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();

        let created = create(&conn, sample_create()).unwrap();

        // Generated id is a non-empty UUID; timestamps equal at creation (Req 6.1).
        assert!(!created.id.is_empty());
        assert_eq!(created.created_at, created.updated_at);
        assert!(created.created_at.ends_with('Z'));

        // Persisted fields (Req 6.7) survive the round-trip.
        assert_eq!(created.title, "My Prompt");
        assert_eq!(created.user_prompt, "Hello {{name}}");
        assert_eq!(created.description.as_deref(), Some("a description"));
        assert_eq!(created.prompt_type, PromptType::Text); // default (Req 6.6)
        assert_eq!(created.system_prompt.as_deref(), Some("be helpful"));
        assert_eq!(created.variables.len(), 1);
        assert_eq!(created.tags, vec!["a".to_string(), "b".to_string()]);
        assert_eq!(created.images, vec!["img1.png".to_string()]);
        assert!(created.videos.is_empty());
        assert!(created.is_favorite);
        assert!(!created.is_pinned);
        assert_eq!(created.usage_count, 3);
        assert_eq!(created.source.as_deref(), Some("https://example.com"));
        assert_eq!(created.notes.as_deref(), Some("note"));
        assert_eq!(created.current_version, 1);

        // get by id returns an equal record (Req 6.2).
        let fetched = get(&conn, &created.id).unwrap();
        assert_eq!(fetched, created);
    }

    #[test]
    fn create_defaults_prompt_type_to_text() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let created = create(&conn, sample_create()).unwrap();
        assert_eq!(created.prompt_type, PromptType::Text);
    }

    #[test]
    fn create_accepts_valid_explicit_prompt_type() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let input = PromptCreate {
            prompt_type: Some("image".into()),
            ..sample_create()
        };
        let created = create(&conn, input).unwrap();
        assert_eq!(created.prompt_type, PromptType::Image);
    }

    #[test]
    fn create_rejects_empty_title() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let input = PromptCreate {
            title: "   ".into(),
            ..sample_create()
        };
        let err = create(&conn, input).unwrap_err();
        assert_eq!(err.code, ErrorCode::Validation);
        // No record was created (Req 6.13).
        assert!(list(&conn).unwrap().is_empty());
    }

    #[test]
    fn create_rejects_empty_user_prompt() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let input = PromptCreate {
            user_prompt: "".into(),
            ..sample_create()
        };
        let err = create(&conn, input).unwrap_err();
        assert_eq!(err.code, ErrorCode::Validation);
        assert!(list(&conn).unwrap().is_empty());
    }

    #[test]
    fn create_rejects_invalid_prompt_type() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let input = PromptCreate {
            prompt_type: Some("audio".into()),
            ..sample_create()
        };
        let err = create(&conn, input).unwrap_err();
        assert_eq!(err.code, ErrorCode::Validation);
        // No record was created (Req 6.14).
        assert!(list(&conn).unwrap().is_empty());
    }

    #[test]
    fn list_returns_empty_when_no_prompts() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        assert!(list(&conn).unwrap().is_empty());
    }

    #[test]
    fn list_returns_all_prompts() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let a = create(&conn, sample_create()).unwrap();
        let b = create(
            &conn,
            PromptCreate {
                title: "Second".into(),
                user_prompt: "body".into(),
                ..Default::default()
            },
        )
        .unwrap();

        let all = list(&conn).unwrap();
        assert_eq!(all.len(), 2);
        let ids: Vec<&str> = all.iter().map(|p| p.id.as_str()).collect();
        assert!(ids.contains(&a.id.as_str()));
        assert!(ids.contains(&b.id.as_str()));
    }

    #[test]
    fn get_missing_id_returns_not_found() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let err = get(&conn, "does-not-exist").unwrap_err();
        assert_eq!(err.code, ErrorCode::NotFound);
    }

    #[test]
    fn update_partial_patch_preserves_other_fields_and_bumps_updated_at() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let created = create(&conn, sample_create()).unwrap();

        // Force a strictly later timestamp than `created`.
        conn.execute(
            "UPDATE prompts SET created_at = created_at - 1000, updated_at = updated_at - 1000 \
             WHERE id = ?1",
            params![created.id],
        )
        .unwrap();
        let baseline = get(&conn, &created.id).unwrap();

        let patch = PromptUpdate {
            title: Some("Renamed".into()),
            ..Default::default()
        };
        let updated = update(&conn, &created.id, patch).unwrap();

        // Supplied field changed.
        assert_eq!(updated.title, "Renamed");
        // Unsupplied fields preserved (Property 7 / Req 6.4).
        assert_eq!(updated.user_prompt, baseline.user_prompt);
        assert_eq!(updated.description, baseline.description);
        assert_eq!(updated.system_prompt, baseline.system_prompt);
        assert_eq!(updated.variables, baseline.variables);
        assert_eq!(updated.tags, baseline.tags);
        assert_eq!(updated.images, baseline.images);
        assert_eq!(updated.is_favorite, baseline.is_favorite);
        assert_eq!(updated.usage_count, baseline.usage_count);
        assert_eq!(updated.source, baseline.source);
        assert_eq!(updated.notes, baseline.notes);
        // createdAt unchanged, updatedAt refreshed forward.
        assert_eq!(updated.created_at, baseline.created_at);
        assert!(
            updated.updated_at > baseline.updated_at,
            "updatedAt should advance: {} !> {}",
            updated.updated_at,
            baseline.updated_at
        );
    }

    #[test]
    fn update_can_change_prompt_type_and_rejects_invalid() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let created = create(&conn, sample_create()).unwrap();

        let updated = update(
            &conn,
            &created.id,
            PromptUpdate {
                prompt_type: Some("video".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(updated.prompt_type, PromptType::Video);

        // Invalid promptType rejected with VALIDATION; stored value unchanged.
        let err = update(
            &conn,
            &created.id,
            PromptUpdate {
                prompt_type: Some("nope".into()),
                ..Default::default()
            },
        )
        .unwrap_err();
        assert_eq!(err.code, ErrorCode::Validation);
        assert_eq!(
            get(&conn, &created.id).unwrap().prompt_type,
            PromptType::Video
        );
    }

    #[test]
    fn update_missing_id_returns_not_found() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let err = update(&conn, "missing", PromptUpdate::default()).unwrap_err();
        assert_eq!(err.code, ErrorCode::NotFound);
    }

    #[test]
    fn delete_removes_prompt_and_returns_ok() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let created = create(&conn, sample_create()).unwrap();

        delete(&conn, &created.id).unwrap();
        assert_eq!(
            get(&conn, &created.id).unwrap_err().code,
            ErrorCode::NotFound
        );
        assert!(list(&conn).unwrap().is_empty());
    }

    #[test]
    fn delete_missing_id_returns_not_found() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let err = delete(&conn, "missing").unwrap_err();
        assert_eq!(err.code, ErrorCode::NotFound);
    }

    #[test]
    fn delete_cascades_version_history() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let created = create(&conn, sample_create()).unwrap();

        // Creation records the first immutable revision automatically.
        let before: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM prompt_versions WHERE prompt_id = ?1",
                params![created.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(before, 1);

        delete(&conn, &created.id).unwrap();

        let after: i64 = conn
            .query_row("SELECT COUNT(*) FROM prompt_versions", [], |row| row.get(0))
            .unwrap();
        assert_eq!(after, 0, "deleting a prompt should cascade to its versions");
    }

    #[test]
    fn duplicate_and_batch_operations_are_atomic_and_revisioned() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let first = create(&conn, sample_create()).unwrap();
        let second = create(
            &conn,
            PromptCreate {
                title: "Second".into(),
                user_prompt: "body".into(),
                ..Default::default()
            },
        )
        .unwrap();
        conn.execute(
            "INSERT INTO folders (id,name,created_at) VALUES ('folder','Folder',0)",
            [],
        )
        .unwrap();

        let copy = duplicate(&conn, &first.id).unwrap();
        assert_ne!(copy.id, first.id);
        assert_eq!(copy.title, format!("{} copy", first.title));
        assert_eq!(
            crate::services::version::list(&conn, &copy.id)
                .unwrap()
                .len(),
            1
        );

        let ids = vec![first.id.clone(), second.id.clone()];
        batch_move(&conn, &ids, Some("folder")).unwrap();
        batch_tag(&conn, &ids, &["shared".into()]).unwrap();
        for id in &ids {
            let prompt = get(&conn, id).unwrap();
            assert_eq!(prompt.folder_id.as_deref(), Some("folder"));
            assert!(prompt.tags.contains(&"shared".into()));
            assert_eq!(crate::services::version::list(&conn, id).unwrap().len(), 3);
        }

        let before = get(&conn, &first.id).unwrap();
        let err = batch_move(&conn, &[first.id.clone(), "missing".into()], None).unwrap_err();
        assert_eq!(err.code, ErrorCode::NotFound);
        assert_eq!(get(&conn, &first.id).unwrap(), before);

        batch_delete(&conn, &ids).unwrap();
        assert_eq!(get(&conn, &first.id).unwrap_err().code, ErrorCode::NotFound);
        assert_eq!(
            get(&conn, &second.id).unwrap_err().code,
            ErrorCode::NotFound
        );
    }

    // --- copy + tag operation tests (task 4.2; Req 6.8–6.11) ---------------

    use std::collections::HashMap;

    /// Creates a prompt with the given title, user prompt, and tags.
    fn create_tagged(conn: &Connection, title: &str, user_prompt: &str, tags: &[&str]) -> Prompt {
        create(
            conn,
            PromptCreate {
                title: title.into(),
                user_prompt: user_prompt.into(),
                tags: Some(tags.iter().map(|t| t.to_string()).collect()),
                ..Default::default()
            },
        )
        .unwrap()
    }

    fn values(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn copy_substitutes_matched_and_leaves_unmatched_intact() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let created = create(
            &conn,
            PromptCreate {
                title: "T".into(),
                user_prompt: "Hi {{name}}, from {{city}}".into(),
                system_prompt: Some("You are {{role}} for {{name}}".into()),
                ..Default::default()
            },
        )
        .unwrap();

        // Only `name` is supplied: it is substituted everywhere; `city` and
        // `role` placeholders are left untouched (Req 6.11).
        let result = copy(&conn, &created.id, &values(&[("name", "Ada")])).unwrap();
        assert_eq!(result.user_prompt, "Hi Ada, from {{city}}");
        assert_eq!(
            result.system_prompt.as_deref(),
            Some("You are {{role}} for Ada")
        );
    }

    #[test]
    fn copy_handles_whitespace_and_default_value_placeholder_forms() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let created = create(
            &conn,
            PromptCreate {
                title: "T".into(),
                user_prompt: "{{ name }} / {{name:Anonymous}} / {{name}}".into(),
                ..Default::default()
            },
        )
        .unwrap();

        // All three spellings resolve to the `name` variable and are replaced.
        let result = copy(&conn, &created.id, &values(&[("name", "Ada")])).unwrap();
        assert_eq!(result.user_prompt, "Ada / Ada / Ada");
    }

    #[test]
    fn copy_with_no_values_returns_text_unchanged() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let created = create(
            &conn,
            PromptCreate {
                title: "T".into(),
                user_prompt: "Hello {{name}}".into(),
                ..Default::default()
            },
        )
        .unwrap();

        let result = copy(&conn, &created.id, &HashMap::new()).unwrap();
        assert_eq!(result.user_prompt, "Hello {{name}}");
        assert_eq!(result.system_prompt, None);
    }

    #[test]
    fn copy_leaves_unterminated_placeholder_intact() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let created = create(
            &conn,
            PromptCreate {
                title: "T".into(),
                user_prompt: "Hello {{name".into(),
                ..Default::default()
            },
        )
        .unwrap();

        let result = copy(&conn, &created.id, &values(&[("name", "Ada")])).unwrap();
        assert_eq!(result.user_prompt, "Hello {{name");
    }

    #[test]
    fn copy_missing_id_returns_not_found() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let err = copy(&conn, "missing", &HashMap::new()).unwrap_err();
        assert_eq!(err.code, ErrorCode::NotFound);
    }

    #[test]
    fn copy_without_references_is_byte_for_byte_with_substitution() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let created = create(
            &conn,
            PromptCreate {
                title: "T".into(),
                user_prompt: "Hello {{name}}".into(),
                system_prompt: Some("Be {{role}}".into()),
                ..Default::default()
            },
        )
        .unwrap();
        let result = copy(&conn, &created.id, &values(&[("name", "Ada")])).unwrap();
        assert_eq!(result.user_prompt, "Hello Ada");
        assert_eq!(result.system_prompt.as_deref(), Some("Be {{role}}"));
        assert!(result.messages.is_empty());
        assert!(result.unexpanded.is_empty());
    }

    #[test]
    fn batch_delete_marks_incoming_references_missing() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let target = create_tagged(&conn, "A", "body-a", &[]);
        let source = create_tagged(&conn, "S", "see @@A", &[]);
        batch_delete(&conn, std::slice::from_ref(&target.id)).unwrap();
        let listed = crate::services::reference::list(&conn, &source.id).unwrap();
        assert_eq!(listed.outgoing[0].resolution, "missing");
        assert!(listed.outgoing[0].target_prompt_id.is_none());
    }

    #[test]
    fn tag_list_returns_empty_when_no_tags() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        // A prompt with no tags still yields an empty distinct set.
        create_tagged(&conn, "T", "body", &[]);
        assert!(tag_list(&conn).unwrap().is_empty());
    }

    #[test]
    fn tag_list_returns_distinct_sorted_tags() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        create_tagged(&conn, "A", "body", &["zebra", "apple"]);
        create_tagged(&conn, "B", "body", &["apple", "mango"]);

        // Deduplicated across prompts and sorted (Req 6.8).
        assert_eq!(
            tag_list(&conn).unwrap(),
            vec![
                "apple".to_string(),
                "mango".to_string(),
                "zebra".to_string()
            ]
        );
    }

    /// Reads a prompt's current tag list directly from the DB.
    fn tags_of(conn: &Connection, id: &str) -> Vec<String> {
        let json: String = conn
            .query_row("SELECT tags FROM prompts WHERE id = ?1", [id], |r| r.get(0))
            .unwrap();
        serde_json::from_str(&json).unwrap()
    }

    #[test]
    fn tag_rename_replaces_across_prompts_without_duplicating() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        // p1 carries only `old`; p2 already carries `new` alongside `old`.
        let p1 = create_tagged(&conn, "P1", "body", &["old", "keep"]);
        let p2 = create_tagged(&conn, "P2", "body", &["new", "old", "other"]);
        let p3 = create_tagged(&conn, "P3", "body", &["unrelated"]);

        tag_rename(&conn, "old", "new").unwrap();

        // p1: old -> new, keep preserved.
        assert_eq!(
            tags_of(&conn, &p1.id),
            vec!["new".to_string(), "keep".to_string()]
        );
        // p2: old removed, new not duplicated (single `new`), order preserved.
        assert_eq!(
            tags_of(&conn, &p2.id),
            vec!["new".to_string(), "other".to_string()]
        );
        // p3: untouched.
        assert_eq!(tags_of(&conn, &p3.id), vec!["unrelated".to_string()]);
    }

    #[test]
    fn tag_rename_is_idempotent() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let p1 = create_tagged(&conn, "P1", "body", &["old", "keep"]);

        tag_rename(&conn, "old", "new").unwrap();
        let after_first = tags_of(&conn, &p1.id);
        // A second rename of the now-absent `old` changes nothing.
        tag_rename(&conn, "old", "new").unwrap();
        let after_second = tags_of(&conn, &p1.id);

        assert_eq!(after_first, vec!["new".to_string(), "keep".to_string()]);
        assert_eq!(after_first, after_second);
    }

    #[test]
    fn tag_rename_only_touches_prompts_carrying_old_tag() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let untouched = create_tagged(&conn, "U", "body", &["other"]);
        // Force a known updatedAt baseline.
        conn.execute(
            "UPDATE prompts SET updated_at = 111 WHERE id = ?1",
            params![untouched.id],
        )
        .unwrap();

        tag_rename(&conn, "old", "new").unwrap();

        let updated_at: i64 = conn
            .query_row(
                "SELECT updated_at FROM prompts WHERE id = ?1",
                [&untouched.id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            updated_at, 111,
            "prompt without `old` must not be rewritten"
        );
    }

    #[test]
    fn tag_delete_removes_tag_everywhere() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let p1 = create_tagged(&conn, "P1", "body", &["drop", "keep"]);
        let p2 = create_tagged(&conn, "P2", "body", &["a", "drop", "b"]);
        let p3 = create_tagged(&conn, "P3", "body", &["keep"]);

        tag_delete(&conn, "drop").unwrap();

        assert_eq!(tags_of(&conn, &p1.id), vec!["keep".to_string()]);
        assert_eq!(
            tags_of(&conn, &p2.id),
            vec!["a".to_string(), "b".to_string()]
        );
        assert_eq!(tags_of(&conn, &p3.id), vec!["keep".to_string()]);
        // The tag is gone from the distinct set.
        assert!(!tag_list(&conn).unwrap().contains(&"drop".to_string()));
    }

    // --- search tests (task 4.3; Req 5.3–5.10) -----------------------------

    /// Builds an in-memory pool with both the base schema and the FTS index, so
    /// `prompts_fts MATCH` works in search tests (the FTS table is created by
    /// `init_fts`, separately from `init_schema`).
    fn search_pool() -> DbPool {
        let pool = create_memory_pool().expect("memory pool");
        let conn = pool.get().expect("conn");
        init_schema(&conn).expect("schema");
        crate::storage::fts::init_fts(&conn).expect("fts");
        pool
    }

    /// Creates a fully-specified prompt for ordering/filter tests.
    fn create_full(
        conn: &Connection,
        title: &str,
        tags: &[&str],
        folder_id: Option<&str>,
        is_favorite: bool,
        usage_count: i64,
    ) -> Prompt {
        create(
            conn,
            PromptCreate {
                title: title.into(),
                user_prompt: "body".into(),
                tags: Some(tags.iter().map(|t| t.to_string()).collect()),
                folder_id: folder_id.map(|f| f.to_string()),
                is_favorite: Some(is_favorite),
                usage_count: Some(usage_count),
                ..Default::default()
            },
        )
        .unwrap()
    }

    /// Forces a prompt's timestamps to known values so ordering by created/updated
    /// is deterministic.
    fn set_timestamps(conn: &Connection, id: &str, created: i64, updated: i64) {
        conn.execute(
            "UPDATE prompts SET created_at = ?1, updated_at = ?2 WHERE id = ?3",
            params![created, updated, id],
        )
        .unwrap();
    }

    fn titles(prompts: &[Prompt]) -> Vec<String> {
        prompts.iter().map(|p| p.title.clone()).collect()
    }

    #[test]
    fn search_keyword_matches_case_insensitively_and_empty_when_no_match() {
        let pool = search_pool();
        let conn = pool.get().unwrap();
        create_full(&conn, "Dragon Slayer", &[], None, false, 0);
        create_full(&conn, "Phoenix Rising", &[], None, false, 0);

        // Keyword matches indexed title content regardless of case (Req 5.3).
        let lower = search(
            &conn,
            SearchQuery {
                keyword: Some("dragon".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(titles(&lower), vec!["Dragon Slayer".to_string()]);

        let upper = search(
            &conn,
            SearchQuery {
                keyword: Some("DRAGON".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(titles(&upper), vec!["Dragon Slayer".to_string()]);

        // No prompt matches -> empty result set (Req 5.3).
        let none = search(
            &conn,
            SearchQuery {
                keyword: Some("wombat".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(none.is_empty());
    }

    #[test]
    fn search_empty_keyword_applies_no_fts_constraint() {
        let pool = search_pool();
        let conn = pool.get().unwrap();
        create_full(&conn, "A", &[], None, false, 0);
        create_full(&conn, "B", &[], None, false, 0);

        // A whitespace-only keyword imposes no FTS filter, so all prompts return.
        let results = search(
            &conn,
            SearchQuery {
                keyword: Some("   ".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn search_applies_conjunctive_filters() {
        let pool = search_pool();
        let conn = pool.get().unwrap();
        // folder_id is a FK (foreign_keys=ON), so create the referenced folders.
        conn.execute(
            "INSERT INTO folders (id, name, created_at) VALUES ('f1','F1',0),('f2','F2',0)",
            [],
        )
        .unwrap();
        // Only this prompt satisfies tag=red AND folder=f1 AND favorite.
        let target = create_full(&conn, "Target", &["red", "blue"], Some("f1"), true, 0);
        // Missing one of each condition.
        create_full(&conn, "WrongFolder", &["red"], Some("f2"), true, 0);
        create_full(&conn, "NotFavorite", &["red"], Some("f1"), false, 0);
        create_full(&conn, "WrongTag", &["green"], Some("f1"), true, 0);

        let results = search(
            &conn,
            SearchQuery {
                tags: Some(vec!["red".into(), "blue".into()]),
                folder_id: Some("f1".into()),
                is_favorite: Some(true),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(titles(&results), vec![target.title]);
    }

    #[test]
    fn search_tag_filter_matches_exact_membership_not_substring() {
        let pool = search_pool();
        let conn = pool.get().unwrap();
        create_full(&conn, "Exact", &["cat"], None, false, 0);
        create_full(&conn, "Superstring", &["category"], None, false, 0);

        // Filtering by `cat` must not match a prompt whose only tag is `category`.
        let results = search(
            &conn,
            SearchQuery {
                tags: Some(vec!["cat".into()]),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(titles(&results), vec!["Exact".to_string()]);
    }

    #[test]
    fn search_keyword_and_filter_combine() {
        let pool = search_pool();
        let conn = pool.get().unwrap();
        create_full(&conn, "Dragon Quest", &["game"], None, true, 0);
        create_full(&conn, "Dragon Diet", &["food"], None, false, 0);

        // Keyword matches both, but the favorite filter narrows to one (Req 5.4).
        let results = search(
            &conn,
            SearchQuery {
                keyword: Some("dragon".into()),
                is_favorite: Some(true),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(titles(&results), vec!["Dragon Quest".to_string()]);
    }

    #[test]
    fn search_orders_by_each_field_and_direction() {
        let pool = search_pool();
        let conn = pool.get().unwrap();
        let a = create_full(&conn, "Apple", &[], None, false, 5);
        let b = create_full(&conn, "Cherry", &[], None, false, 1);
        let c = create_full(&conn, "Banana", &[], None, false, 9);
        // created: a<b<c ; updated set distinctly below.
        set_timestamps(&conn, &a.id, 100, 300);
        set_timestamps(&conn, &b.id, 200, 100);
        set_timestamps(&conn, &c.id, 300, 200);

        let by = |field: SortField, order: SortOrder| {
            titles(
                &search(
                    &conn,
                    SearchQuery {
                        sort_by: Some(field),
                        sort_order: Some(order),
                        ..Default::default()
                    },
                )
                .unwrap(),
            )
        };

        // Title.
        assert_eq!(
            by(SortField::Title, SortOrder::Asc),
            vec!["Apple", "Banana", "Cherry"]
        );
        assert_eq!(
            by(SortField::Title, SortOrder::Desc),
            vec!["Cherry", "Banana", "Apple"]
        );
        // usageCount: a=5,b=1,c=9.
        assert_eq!(
            by(SortField::UsageCount, SortOrder::Asc),
            vec!["Cherry", "Apple", "Banana"]
        );
        assert_eq!(
            by(SortField::UsageCount, SortOrder::Desc),
            vec!["Banana", "Apple", "Cherry"]
        );
        // createdAt: a=100,b=200,c=300.
        assert_eq!(
            by(SortField::CreatedAt, SortOrder::Asc),
            vec!["Apple", "Cherry", "Banana"]
        );
        // updatedAt: a=300,b=100,c=200.
        assert_eq!(
            by(SortField::UpdatedAt, SortOrder::Asc),
            vec!["Cherry", "Banana", "Apple"]
        );
        assert_eq!(
            by(SortField::UpdatedAt, SortOrder::Desc),
            vec!["Apple", "Banana", "Cherry"]
        );
    }

    #[test]
    fn search_defaults_to_updated_at_desc_when_sort_omitted() {
        let pool = search_pool();
        let conn = pool.get().unwrap();
        let a = create_full(&conn, "A", &[], None, false, 0);
        let b = create_full(&conn, "B", &[], None, false, 0);
        let c = create_full(&conn, "C", &[], None, false, 0);
        set_timestamps(&conn, &a.id, 0, 100);
        set_timestamps(&conn, &b.id, 0, 300);
        set_timestamps(&conn, &c.id, 0, 200);

        // No sort specified -> updatedAt DESC (Req 5.8): b(300), c(200), a(100).
        let results = search(&conn, SearchQuery::default()).unwrap();
        assert_eq!(titles(&results), vec!["B", "C", "A"]);
    }

    #[test]
    fn search_clamps_limit_and_applies_offset() {
        let pool = search_pool();
        let conn = pool.get().unwrap();
        // 5 prompts ordered deterministically by updatedAt DESC: e,d,c,b,a.
        for (i, name) in ["a", "b", "c", "d", "e"].iter().enumerate() {
            let p = create_full(&conn, name, &[], None, false, 0);
            set_timestamps(&conn, &p.id, 0, (i as i64 + 1) * 100);
        }

        // limit 0 clamps up to 1 (Req 5.6).
        let one = search(
            &conn,
            SearchQuery {
                limit: Some(0),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(titles(&one), vec!["e"]);

        // limit 101 clamps down to 100, returning all 5 available rows.
        let all = search(
            &conn,
            SearchQuery {
                limit: Some(101),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(all.len(), 5);

        // offset skips into the ordered window (Req 5.6, 5.9).
        let offset = search(
            &conn,
            SearchQuery {
                limit: Some(2),
                offset: Some(2),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(titles(&offset), vec!["c", "b"]);
    }

    #[test]
    fn search_default_limit_is_50() {
        let pool = search_pool();
        let conn = pool.get().unwrap();
        // 51 prompts; the default limit returns at most 50 (Req 5.9).
        for i in 0..51 {
            create_full(&conn, &format!("P{i:02}"), &[], None, false, 0);
        }
        let results = search(&conn, SearchQuery::default()).unwrap();
        assert_eq!(results.len(), 50);
        assert_eq!(results.total, 51);
        assert!(results.has_more);
    }

    #[test]
    fn search_pages_reach_all_250_prompts_without_duplicates() {
        let pool = search_pool();
        let conn = pool.get().unwrap();
        for i in 0..250 {
            create_full(&conn, &format!("Prompt {i:03}"), &[], None, false, 0);
        }

        let mut ids = std::collections::HashSet::new();
        for offset in [0, 100, 200] {
            let page = search(
                &conn,
                SearchQuery {
                    sort_by: Some(SortField::Title),
                    sort_order: Some(SortOrder::Asc),
                    limit: Some(100),
                    offset: Some(offset),
                    ..Default::default()
                },
            )
            .unwrap();
            assert_eq!(page.total, 250);
            assert_eq!(page.offset, offset);
            assert_eq!(page.has_more, offset < 200);
            for prompt in page.items {
                assert!(ids.insert(prompt.id), "duplicate prompt across pages");
            }
        }
        assert_eq!(ids.len(), 250);
    }

    #[test]
    fn search_keyword_with_special_chars_does_not_panic() {
        let pool = search_pool();
        let conn = pool.get().unwrap();
        create_full(&conn, "Normal Prompt", &[], None, false, 0);

        // Each of these contains FTS5 operators/special characters. The contract
        // is: return a result set or a structured PARSE error, never panic (5.7).
        for keyword in [
            "dragon OR phoenix",
            "\"unbalanced",
            "(a AND b)",
            "* ^ : NEAR",
            "title:foo",
            "NOT something",
            "a-b c+d",
        ] {
            let result = search(
                &conn,
                SearchQuery {
                    keyword: Some(keyword.into()),
                    ..Default::default()
                },
            );
            match result {
                Ok(_) => {}
                Err(e) => assert_eq!(
                    e.code,
                    ErrorCode::Parse,
                    "special-char keyword `{keyword}` should yield a PARSE error, got {e}"
                ),
            }
        }
    }

    #[test]
    fn search_reflects_read_after_write() {
        let pool = search_pool();
        let conn = pool.get().unwrap();

        // Not findable before creation.
        let before = search(
            &conn,
            SearchQuery {
                keyword: Some("Zephyr".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(before.is_empty());

        // Create, then immediately search: the new prompt is findable (Req 5.2).
        let created = create_full(&conn, "Zephyr Engine", &[], None, false, 0);
        let after = search(
            &conn,
            SearchQuery {
                keyword: Some("Zephyr".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(titles(&after), vec!["Zephyr Engine".to_string()]);

        // Delete, then search: no longer findable.
        delete(&conn, &created.id).unwrap();
        let gone = search(
            &conn,
            SearchQuery {
                keyword: Some("Zephyr".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(gone.is_empty());
    }

    #[test]
    fn private_prompt_is_encrypted_redacted_unsearchable_and_rekeyed() {
        let pool = search_pool();
        let conn = pool.get().unwrap();
        let encryption = Mutex::new(crate::state::EncryptionState::default());
        crate::services::security::set_master_password(&conn, &encryption, "old-password").unwrap();

        let created = create_secure(
            &conn,
            &encryption,
            PromptCreate {
                title: "Private metadata".into(),
                description: Some("classified description".into()),
                system_prompt: Some("classified system".into()),
                user_prompt: "classified body".into(),
                source: Some("classified source".into()),
                notes: Some("classified notes".into()),
                is_private: Some(true),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(created.user_prompt, "classified body");
        assert!(created.is_private);
        assert!(!created.is_locked);

        let stored = get(&conn, &created.id).unwrap();
        assert!(stored.user_prompt.starts_with("ENC::"));
        assert!(!stored.user_prompt.contains("classified"));
        let stored_revision = crate::services::version::list(&conn, &created.id).unwrap();
        assert!(stored_revision[0].user_prompt.starts_with("ENC::"));

        let found = search_secure(
            &conn,
            &encryption,
            SearchQuery {
                keyword: Some("classified".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(found.total, 0);

        crate::services::security::lock(&encryption).unwrap();
        let locked = get_secure(&conn, &encryption, &created.id).unwrap();
        assert!(locked.is_locked);
        assert!(locked.user_prompt.is_empty());
        assert!(locked.description.is_none());
        assert_eq!(
            copy_secure(&conn, &encryption, &created.id, &Default::default())
                .unwrap_err()
                .code,
            ErrorCode::Unauthorized
        );

        crate::services::security::unlock(&conn, &encryption, "old-password").unwrap();
        crate::services::security::change_master_password(
            &conn,
            &encryption,
            "old-password",
            "new-password",
        )
        .unwrap();
        crate::services::security::lock(&encryption).unwrap();
        crate::services::security::unlock(&conn, &encryption, "new-password").unwrap();
        let rekeyed = get_secure(&conn, &encryption, &created.id).unwrap();
        assert_eq!(rekeyed.user_prompt, "classified body");
        assert_eq!(rekeyed.notes.as_deref(), Some("classified notes"));
    }

    #[test]
    fn ordered_chat_messages_round_trip_in_private_revisions() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let encryption = Mutex::new(EncryptionState::default());
        crate::services::security::set_master_password(&conn, &encryption, "old-password").unwrap();
        let messages = vec![
            PromptMessage {
                role: "system".into(),
                content: "Be concise".into(),
            },
            PromptMessage {
                role: "user".into(),
                content: "Hello {{name}}".into(),
            },
            PromptMessage {
                role: "assistant".into(),
                content: "Hello".into(),
            },
        ];
        let created = create_secure(
            &conn,
            &encryption,
            PromptCreate {
                title: "Private chat".into(),
                user_prompt: String::new(),
                messages: Some(messages.clone()),
                is_private: Some(true),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(created.messages, messages);

        let stored = get(&conn, &created.id).unwrap();
        assert!(stored
            .messages
            .iter()
            .all(|message| message.content.starts_with("ENC::")));
        let revision = crate::services::version::list(&conn, &created.id).unwrap();
        let key = crate::services::security::unlocked_key(&encryption).unwrap();
        assert_eq!(
            present_version(revision[0].clone(), key.as_deref())
                .unwrap()
                .messages,
            messages
        );

        crate::services::security::change_master_password(
            &conn,
            &encryption,
            "old-password",
            "new-password",
        )
        .unwrap();
        crate::services::security::lock(&encryption).unwrap();
        assert!(get_secure(&conn, &encryption, &created.id)
            .unwrap()
            .messages
            .is_empty());
        crate::services::security::unlock(&conn, &encryption, "new-password").unwrap();
        assert_eq!(
            get_secure(&conn, &encryption, &created.id)
                .unwrap()
                .messages,
            messages
        );
    }

    #[test]
    fn custom_type_derives_base_kind_and_round_trips_revision_duplicate_and_lock() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let definition = crate::services::prompt_type::create(
            &conn,
            crate::services::prompt_type::PromptTypeCreate {
                name: "Storyboard".into(),
                base_kind: "image".into(),
            },
        )
        .unwrap();
        let encryption = Mutex::new(EncryptionState::default());
        crate::services::security::set_master_password(&conn, &encryption, "password").unwrap();
        let created = create_secure(
            &conn,
            &encryption,
            PromptCreate {
                title: "Private storyboard".into(),
                user_prompt: "scene".into(),
                type_definition_id: Some(definition.id.clone()),
                is_private: Some(true),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(created.prompt_type, PromptType::Image);
        assert_eq!(created.type_definition_id, Some(definition.id.clone()));

        let revision = crate::services::version::list(&conn, &created.id).unwrap();
        assert_eq!(revision[0].type_definition_id, Some(definition.id.clone()));
        assert_eq!(
            revision[0]
                .type_definition
                .as_ref()
                .map(|snapshot| snapshot.name.as_str()),
            Some("Storyboard")
        );
        let rendered = crate::services::evaluation::render_prompt(
            &conn,
            &encryption,
            &revision[0].id,
            &std::collections::BTreeMap::new(),
        )
        .unwrap();
        assert_eq!(rendered.messages.last().unwrap().content, "scene");

        let duplicate = duplicate(&conn, &created.id).unwrap();
        assert_eq!(duplicate.type_definition_id, Some(definition.id.clone()));
        crate::services::security::lock(&encryption).unwrap();
        let locked = get_secure(&conn, &encryption, &created.id).unwrap();
        assert!(locked.is_locked);
        assert_eq!(locked.type_definition_id, Some(definition.id));
    }

    #[test]
    fn custom_type_rejects_missing_and_mismatched_pairs_before_mutation() {
        let pool = schema_pool();
        let conn = pool.get().unwrap();
        let definition = crate::services::prompt_type::create(
            &conn,
            crate::services::prompt_type::PromptTypeCreate {
                name: "Storyboard".into(),
                base_kind: "image".into(),
            },
        )
        .unwrap();
        let mismatch = create(
            &conn,
            PromptCreate {
                title: "Bad".into(),
                user_prompt: "body".into(),
                prompt_type: Some("text".into()),
                type_definition_id: Some(definition.id.clone()),
                ..Default::default()
            },
        )
        .unwrap_err();
        assert_eq!(mismatch.code_str(), "VALIDATION");
        assert!(list(&conn).unwrap().is_empty());

        let created = create(
            &conn,
            PromptCreate {
                title: "Good".into(),
                user_prompt: "body".into(),
                type_definition_id: Some(definition.id),
                ..Default::default()
            },
        )
        .unwrap();
        let error = update(
            &conn,
            &created.id,
            PromptUpdate {
                title: Some("Must not persist".into()),
                prompt_type: Some("text".into()),
                ..Default::default()
            },
        )
        .unwrap_err();
        assert_eq!(error.code_str(), "VALIDATION");
        assert_eq!(get(&conn, &created.id).unwrap().title, "Good");

        let missing = update(
            &conn,
            &created.id,
            PromptUpdate {
                type_definition_id: Some(Some("missing".into())),
                ..Default::default()
            },
        )
        .unwrap_err();
        assert_eq!(missing.code_str(), "NOT_FOUND");
        let cleared = update(
            &conn,
            &created.id,
            PromptUpdate {
                prompt_type: Some("text".into()),
                type_definition_id: Some(None),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(cleared.prompt_type, PromptType::Text);
        assert_eq!(cleared.type_definition_id, None);
    }
}
