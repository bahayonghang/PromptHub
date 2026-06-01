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

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::models::{Prompt, PromptType, Variable};
use crate::models::{SearchQuery, SortField, SortOrder};
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
    /// Optional system prompt.
    pub system_prompt: Option<String>,
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
    /// Replacement system prompt.
    pub system_prompt: Option<String>,
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

/// Creates a prompt and returns the stored record (Req 6.1, 6.6, 6.7).
///
/// Validates non-empty `title`/`userPrompt` (Req 6.13) and the optional
/// `promptType` (Req 6.14) before writing. Generates a UUID identifier, sets
/// `createdAt` equal to `updatedAt` at creation (Req 6.1), defaults `promptType`
/// to `text`, and persists variables, tags, image/video references, the
/// favorite/pinned flags, usage count, source, and notes (Req 6.7).
pub fn create(conn: &Connection, input: PromptCreate) -> Result<Prompt, AppError> {
    if input.title.trim().is_empty() {
        return Err(AppError::validation("title is required"));
    }
    if input.user_prompt.trim().is_empty() {
        return Err(AppError::validation("userPrompt is required"));
    }
    let prompt_type = match input.prompt_type.as_deref() {
        Some(raw) => parse_prompt_type(raw)?,
        None => PromptType::Text,
    };

    let id = uuid::Uuid::new_v4().to_string();
    let now = now_millis();
    let variables = json_array(&input.variables.unwrap_or_default());
    let tags = json_array(&input.tags.unwrap_or_default());
    let images = json_array(&input.images.unwrap_or_default());
    let videos = json_array(&input.videos.unwrap_or_default());

    conn.execute(
        "INSERT INTO prompts \
         (id, title, description, prompt_type, system_prompt, user_prompt, variables, tags, \
          folder_id, images, videos, is_favorite, is_pinned, current_version, usage_count, \
          source, notes, last_ai_response, created_at, updated_at) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20)",
        params![
            id,
            input.title,
            input.description,
            prompt_type_wire(prompt_type),
            input.system_prompt,
            input.user_prompt,
            variables,
            tags,
            input.folder_id,
            images,
            videos,
            input.is_favorite.unwrap_or(false),
            input.is_pinned.unwrap_or(false),
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
    // Validate the optional promptType before touching the database so a rejected
    // request never mutates stored data (Req 2.3).
    let new_prompt_type = match patch.prompt_type.as_deref() {
        Some(raw) => Some(parse_prompt_type(raw)?),
        None => None,
    };

    // NOT_FOUND when the prompt does not exist; also the base for preserved fields.
    let existing = get(conn, id)?;

    let title = patch.title.unwrap_or(existing.title);
    let description = patch.description.or(existing.description);
    let prompt_type = new_prompt_type.unwrap_or(existing.prompt_type);
    let system_prompt = patch.system_prompt.or(existing.system_prompt);
    let user_prompt = patch.user_prompt.unwrap_or(existing.user_prompt);
    let variables = json_array(&patch.variables.unwrap_or(existing.variables));
    let tags = json_array(&patch.tags.unwrap_or(existing.tags));
    let folder_id = patch.folder_id.or(existing.folder_id);
    let images = json_array(&patch.images.unwrap_or(existing.images));
    let videos = json_array(&patch.videos.unwrap_or(existing.videos));
    let is_favorite = patch.is_favorite.unwrap_or(existing.is_favorite);
    let is_pinned = patch.is_pinned.unwrap_or(existing.is_pinned);
    let usage_count = patch.usage_count.unwrap_or(existing.usage_count);
    let source = patch.source.or(existing.source);
    let notes = patch.notes.or(existing.notes);
    let last_ai_response = patch.last_ai_response.or(existing.last_ai_response);
    let now = now_millis();

    conn.execute(
        "UPDATE prompts SET \
         title=?1, description=?2, prompt_type=?3, system_prompt=?4, user_prompt=?5, \
         variables=?6, tags=?7, folder_id=?8, images=?9, videos=?10, is_favorite=?11, \
         is_pinned=?12, usage_count=?13, source=?14, notes=?15, last_ai_response=?16, \
         updated_at=?17 \
         WHERE id=?18",
        params![
            title,
            description,
            prompt_type_wire(prompt_type),
            system_prompt,
            user_prompt,
            variables,
            tags,
            folder_id,
            images,
            videos,
            is_favorite,
            is_pinned,
            usage_count,
            source,
            notes,
            last_ai_response,
            now,
            id,
        ],
    )
    .map_err(|e| db_err("failed to update prompt", e))?;

    get(conn, id)
}

/// Deletes a prompt by identifier (Req 6.5).
///
/// The `ON DELETE CASCADE` foreign key on `prompt_versions` removes the prompt's
/// version history as part of the same delete (Req 4.4). Returns `NOT_FOUND`
/// when the prompt does not exist (Req 6.12).
pub fn delete(conn: &Connection, id: &str) -> Result<(), AppError> {
    let affected = conn
        .execute("DELETE FROM prompts WHERE id = ?1", [id])
        .map_err(|e| db_err("failed to delete prompt", e))?;
    if affected == 0 {
        return Err(AppError::not_found(format!("prompt `{id}` not found")));
    }
    Ok(())
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
pub fn search(conn: &Connection, query: SearchQuery) -> Result<Vec<Prompt>, AppError> {
    let match_expr = query.keyword.as_deref().and_then(build_fts_match);
    let has_keyword = match_expr.is_some();

    let limit = query.limit.map(|l| l.clamp(1, 100)).unwrap_or(50);
    let offset = query.offset.unwrap_or(0);

    let mut sql = String::from("SELECT prompts.* FROM prompts");
    if has_keyword {
        sql.push_str(" JOIN prompts_fts ON prompts_fts.rowid = prompts.rowid");
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
        sql.push_str(" WHERE ");
        sql.push_str(&clauses.join(" AND "));
    }

    let field = query.sort_by.unwrap_or_default();
    let order = query.sort_order.unwrap_or_default();
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
    Ok(out)
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
fn substitute_placeholders(
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
    let prompt = get(conn, id)?;
    Ok(PromptCopy {
        system_prompt: prompt
            .system_prompt
            .as_deref()
            .map(|text| substitute_placeholders(text, values)),
        user_prompt: substitute_placeholders(&prompt.user_prompt, values),
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
            system_prompt: Some("be helpful".into()),
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
        assert_eq!(created.current_version, 0);

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

        // Insert a version row directly (Version_Service is a later task).
        conn.execute(
            "INSERT INTO prompt_versions (id, prompt_id, version, user_prompt, created_at) \
             VALUES ('v1', ?1, 1, 'U', 0)",
            params![created.id],
        )
        .unwrap();
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
}
