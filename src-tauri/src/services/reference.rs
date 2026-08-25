//! Prompt-to-prompt `@@title` references: extract, persist, expand, and list.

use std::collections::HashSet;
use std::sync::Mutex;

use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::models::{Prompt, PromptMessage};
use crate::services::prompt::{self, PromptCreate, PromptUpdate};
use crate::state::EncryptionState;
use crate::storage::time::now_millis;

pub const MAX_EXPAND_DEPTH: usize = 3;

fn db_err(context: &str, error: rusqlite::Error) -> AppError {
    AppError::internal(format!("{context}: {error}"))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnexpandedReference {
    pub token_title: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutgoingReference {
    pub target_prompt_id: Option<String>,
    pub target_title: Option<String>,
    pub token_title: String,
    pub resolution: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IncomingReference {
    pub source_prompt_id: String,
    pub source_title: String,
    pub token_title: String,
    pub resolution: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceList {
    pub outgoing: Vec<OutgoingReference>,
    pub incoming: Vec<IncomingReference>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptReferenceRecord {
    pub source_prompt_id: String,
    pub target_prompt_id: Option<String>,
    pub token_title: String,
    pub resolution: String,
}

#[derive(Debug, Clone)]
pub struct ReferenceScan {
    pub system_prompt: Option<String>,
    pub user_prompt: String,
    pub messages: Vec<PromptMessage>,
}

impl ReferenceScan {
    pub fn from_create(input: &PromptCreate) -> Self {
        Self {
            system_prompt: input.system_prompt.clone(),
            user_prompt: input.user_prompt.clone(),
            messages: input.messages.clone().unwrap_or_default(),
        }
    }

    pub fn from_update(patch: &PromptUpdate, existing: &Prompt) -> Self {
        Self {
            system_prompt: patch
                .system_prompt
                .clone()
                .or_else(|| existing.system_prompt.clone()),
            user_prompt: patch
                .user_prompt
                .clone()
                .unwrap_or_else(|| existing.user_prompt.clone()),
            messages: patch
                .messages
                .clone()
                .unwrap_or_else(|| existing.messages.clone()),
        }
    }

    pub fn from_prompt(prompt: &Prompt) -> Self {
        Self {
            system_prompt: prompt.system_prompt.clone(),
            user_prompt: prompt.user_prompt.clone(),
            messages: prompt.messages.clone(),
        }
    }

    fn bodies(&self) -> Vec<&str> {
        let mut bodies = Vec::new();
        if let Some(system) = &self.system_prompt {
            bodies.push(system.as_str());
        }
        bodies.push(self.user_prompt.as_str());
        for message in &self.messages {
            bodies.push(message.content.as_str());
        }
        bodies
    }
}

struct TokenSpan {
    title: String,
    start: usize,
    consumed: usize,
}

/// Walks `@@Title@@` then `@@Title` to end of line. Empty titles are skipped.
fn extract_token_spans(body: &str) -> Vec<TokenSpan> {
    let mut spans = Vec::new();
    let mut index = 0;
    while let Some(rel) = body[index..].find("@@") {
        let start = index + rel;
        let after = &body[start + 2..];
        if after.is_empty() {
            break;
        }
        let line_end = after.find('\n').unwrap_or(after.len());
        let line = &after[..line_end];
        let (title, consumed) = if let Some(end) = line.find("@@") {
            (line[..end].trim(), 2 + end + 2)
        } else {
            (line.trim(), 2 + line_end)
        };
        if consumed == 0 {
            break;
        }
        if !title.is_empty() {
            spans.push(TokenSpan {
                title: title.to_string(),
                start,
                consumed,
            });
        }
        index = start + consumed;
    }
    spans
}

/// Extracts distinct `@@Title@@` then `@@Title` tokens in first-seen order.
pub fn extract_tokens(body: &str) -> Vec<String> {
    let mut titles = Vec::new();
    let mut seen = HashSet::new();
    for span in extract_token_spans(body) {
        if seen.insert(span.title.clone()) {
            titles.push(span.title);
        }
    }
    titles
}

fn prompt_exists(conn: &Connection, id: &str) -> Result<bool, AppError> {
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM prompts WHERE id = ?1)",
        [id],
        |row| row.get(0),
    )
    .map_err(|error| db_err("failed to check prompt exists", error))
}

fn resolve_title(
    conn: &Connection,
    title: &str,
) -> Result<(Option<String>, &'static str), AppError> {
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM prompts WHERE title = ?1",
            [title],
            |row| row.get(0),
        )
        .map_err(|error| db_err("failed to count reference targets", error))?;
    if count == 1 {
        let id: String = conn
            .query_row("SELECT id FROM prompts WHERE title = ?1", [title], |row| {
                row.get(0)
            })
            .map_err(|error| db_err("failed to read reference target", error))?;
        Ok((Some(id), "resolved"))
    } else if count == 0 {
        Ok((None, "missing"))
    } else {
        Ok((None, "ambiguous"))
    }
}

pub fn resolve_and_store(
    tx: &Transaction<'_>,
    prompt_id: &str,
    scan: &ReferenceScan,
) -> Result<(), AppError> {
    let mut titles = Vec::new();
    let mut seen = HashSet::new();
    for body in scan.bodies() {
        for title in extract_tokens(body) {
            if seen.insert(title.clone()) {
                titles.push(title);
            }
        }
    }
    tx.execute(
        "DELETE FROM prompt_references WHERE source_prompt_id = ?1",
        [prompt_id],
    )
    .map_err(|error| db_err("failed to replace prompt references", error))?;
    let now = now_millis();
    for title in titles {
        let (target, resolution) = resolve_title(tx, &title)?;
        insert_row(tx, prompt_id, target.as_deref(), &title, resolution, now)?;
    }
    Ok(())
}

fn insert_row(
    tx: &Transaction<'_>,
    source_prompt_id: &str,
    target_prompt_id: Option<&str>,
    token_title: &str,
    resolution: &str,
    created_at: i64,
) -> Result<(), AppError> {
    tx.execute(
        "INSERT INTO prompt_references \
         (id, source_prompt_id, target_prompt_id, token_title, resolution, created_at) \
         VALUES (?1,?2,?3,?4,?5,?6)",
        params![
            uuid::Uuid::new_v4().to_string(),
            source_prompt_id,
            target_prompt_id,
            token_title,
            resolution,
            created_at,
        ],
    )
    .map_err(|error| db_err("failed to insert prompt reference", error))?;
    Ok(())
}

pub fn delete_outgoing(tx: &Transaction<'_>, source_prompt_id: &str) -> Result<(), AppError> {
    tx.execute(
        "DELETE FROM prompt_references WHERE source_prompt_id = ?1",
        [source_prompt_id],
    )
    .map_err(|error| db_err("failed to replace prompt references", error))?;
    Ok(())
}

pub fn copy_outgoing(
    tx: &Transaction<'_>,
    from_source_id: &str,
    to_source_id: &str,
) -> Result<(), AppError> {
    let records = list_for_prompts(tx, &[from_source_id.to_string()])?;
    let now = now_millis();
    for record in records {
        insert_row(
            tx,
            to_source_id,
            record.target_prompt_id.as_deref(),
            &record.token_title,
            &record.resolution,
            now,
        )?;
    }
    Ok(())
}

/// Inserts one imported edge. A missing remapped target is re-resolved by title.
pub fn insert_imported(
    tx: &Transaction<'_>,
    source_prompt_id: &str,
    remapped_target: Option<&str>,
    token_title: &str,
    stored_resolution: &str,
) -> Result<(), AppError> {
    let (target, resolution) = match remapped_target {
        Some(id) if prompt_exists(tx, id)? => (Some(id.to_string()), stored_resolution.to_string()),
        _ => {
            let (id, res) = resolve_title(tx, token_title)?;
            (id, res.to_string())
        }
    };
    insert_row(
        tx,
        source_prompt_id,
        target.as_deref(),
        token_title,
        &resolution,
        now_millis(),
    )
}

pub fn mark_incoming_missing(tx: &Transaction<'_>, target_id: &str) -> Result<(), AppError> {
    tx.execute(
        "UPDATE prompt_references SET resolution = 'missing', target_prompt_id = NULL \
         WHERE target_prompt_id = ?1",
        [target_id],
    )
    .map_err(|error| db_err("failed to mark incoming references missing", error))?;
    Ok(())
}

fn lookup_edge(
    conn: &Connection,
    source_id: &str,
    token_title: &str,
) -> Result<Option<(Option<String>, String)>, AppError> {
    conn.query_row(
        "SELECT target_prompt_id, resolution FROM prompt_references \
         WHERE source_prompt_id = ?1 AND token_title = ?2",
        params![source_id, token_title],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .optional()
    .map_err(|error| db_err("failed to read prompt reference", error))
}

fn inline_token(original: &str, start: usize, consumed: usize, replacement: &str) -> String {
    let mut out = String::with_capacity(original.len() + replacement.len());
    out.push_str(&original[..start]);
    out.push_str(replacement);
    out.push_str(&original[start + consumed..]);
    out
}

fn inline_target_body(
    conn: &Connection,
    encryption: &Mutex<EncryptionState>,
    target: &crate::models::Prompt,
    ancestors: &mut Vec<String>,
    depth: usize,
) -> Result<(String, Vec<UnexpandedReference>), AppError> {
    let mut inlined = String::new();
    let mut unexpanded = Vec::new();
    if let Some(system) = &target.system_prompt {
        let (text, nested) = expand(conn, encryption, &target.id, system, ancestors, depth + 1)?;
        inlined.push_str(&text);
        unexpanded.extend(nested);
    }
    let (user, nested) = expand(
        conn,
        encryption,
        &target.id,
        &target.user_prompt,
        ancestors,
        depth + 1,
    )?;
    if !inlined.is_empty() && !user.is_empty() {
        inlined.push('\n');
    }
    inlined.push_str(&user);
    unexpanded.extend(nested);
    for message in &target.messages {
        let (text, nested) = expand(
            conn,
            encryption,
            &target.id,
            &message.content,
            ancestors,
            depth + 1,
        )?;
        if !inlined.is_empty() {
            inlined.push('\n');
        }
        inlined.push_str(&text);
        unexpanded.extend(nested);
    }
    Ok((inlined, unexpanded))
}

pub fn expand(
    conn: &Connection,
    encryption: &Mutex<EncryptionState>,
    source_prompt_id: &str,
    body: &str,
    ancestors: &mut Vec<String>,
    depth: usize,
) -> Result<(String, Vec<UnexpandedReference>), AppError> {
    let mut unexpanded = Vec::new();
    let mut replacements = Vec::new();
    for span in extract_token_spans(body) {
        let edge = lookup_edge(conn, source_prompt_id, &span.title)?;
        let reason = match edge.as_ref() {
            Some((_, resolution)) if resolution == "missing" => Some("missing"),
            Some((_, resolution)) if resolution == "ambiguous" => Some("ambiguous"),
            None => Some("missing"),
            Some((None, _)) => Some("missing"),
            Some((Some(target_id), _)) if ancestors.contains(target_id) => Some("cycle"),
            Some(_) if depth >= MAX_EXPAND_DEPTH => Some("depth"),
            Some((Some(_), _)) => None,
        };
        if let Some(reason) = reason {
            unexpanded.push(UnexpandedReference {
                token_title: span.title,
                reason: reason.to_string(),
            });
            continue;
        }
        let Some((Some(target_id), _)) = edge else {
            unexpanded.push(UnexpandedReference {
                token_title: span.title,
                reason: "missing".to_string(),
            });
            continue;
        };
        let target = prompt::get_secure(conn, encryption, &target_id)?;
        if target.is_locked {
            unexpanded.push(UnexpandedReference {
                token_title: span.title,
                reason: "locked".to_string(),
            });
            continue;
        }
        ancestors.push(target_id);
        let (inlined, nested) = inline_target_body(conn, encryption, &target, ancestors, depth)?;
        unexpanded.extend(nested);
        ancestors.pop();
        replacements.push((span.start, span.consumed, inlined));
    }
    let mut output = body.to_string();
    for (start, consumed, inlined) in replacements.into_iter().rev() {
        output = inline_token(&output, start, consumed, &inlined);
    }
    Ok((output, unexpanded))
}

pub fn expand_copy(
    conn: &Connection,
    encryption: &Mutex<EncryptionState>,
    prompt: &Prompt,
) -> Result<(Prompt, Vec<UnexpandedReference>), AppError> {
    let mut ancestors = vec![prompt.id.clone()];
    let mut unexpanded = Vec::new();
    let mut next = prompt.clone();
    if let Some(system) = &prompt.system_prompt {
        let (text, nested) = expand(conn, encryption, &prompt.id, system, &mut ancestors, 0)?;
        next.system_prompt = Some(text);
        unexpanded.extend(nested);
    }
    let (user, nested) = expand(
        conn,
        encryption,
        &prompt.id,
        &prompt.user_prompt,
        &mut ancestors,
        0,
    )?;
    next.user_prompt = user;
    unexpanded.extend(nested);
    let mut messages = Vec::new();
    for message in &prompt.messages {
        let (text, nested) = expand(
            conn,
            encryption,
            &prompt.id,
            &message.content,
            &mut ancestors,
            0,
        )?;
        messages.push(PromptMessage {
            role: message.role.clone(),
            content: text,
        });
        unexpanded.extend(nested);
    }
    next.messages = messages;
    Ok((next, unexpanded))
}

pub fn list(conn: &Connection, prompt_id: &str) -> Result<ReferenceList, AppError> {
    prompt::get(conn, prompt_id)?;
    let mut outgoing_stmt = conn
        .prepare(
            "SELECT r.target_prompt_id, t.title, r.token_title, r.resolution \
             FROM prompt_references r \
             LEFT JOIN prompts t ON t.id = r.target_prompt_id \
             WHERE r.source_prompt_id = ?1 \
             ORDER BY r.created_at, r.token_title",
        )
        .map_err(|error| db_err("failed to list outgoing references", error))?;
    let outgoing = outgoing_stmt
        .query_map([prompt_id], |row| {
            Ok(OutgoingReference {
                target_prompt_id: row.get(0)?,
                target_title: row.get(1)?,
                token_title: row.get(2)?,
                resolution: row.get(3)?,
            })
        })
        .map_err(|error| db_err("failed to query outgoing references", error))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| db_err("failed to map outgoing references", error))?;

    let mut incoming_stmt = conn
        .prepare(
            "SELECT r.source_prompt_id, s.title, r.token_title, r.resolution \
             FROM prompt_references r \
             JOIN prompts s ON s.id = r.source_prompt_id \
             WHERE r.target_prompt_id = ?1 AND r.resolution = 'resolved' \
             ORDER BY r.created_at, r.token_title",
        )
        .map_err(|error| db_err("failed to list incoming references", error))?;
    let incoming = incoming_stmt
        .query_map([prompt_id], |row| {
            Ok(IncomingReference {
                source_prompt_id: row.get(0)?,
                source_title: row.get(1)?,
                token_title: row.get(2)?,
                resolution: row.get(3)?,
            })
        })
        .map_err(|error| db_err("failed to query incoming references", error))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| db_err("failed to map incoming references", error))?;
    Ok(ReferenceList { outgoing, incoming })
}

pub fn list_for_prompts(
    conn: &Connection,
    prompt_ids: &[String],
) -> Result<Vec<PromptReferenceRecord>, AppError> {
    if prompt_ids.is_empty() {
        return Ok(Vec::new());
    }
    let mut records = Vec::new();
    for id in prompt_ids {
        let mut stmt = conn
            .prepare(
                "SELECT source_prompt_id, target_prompt_id, token_title, resolution \
                 FROM prompt_references WHERE source_prompt_id = ?1",
            )
            .map_err(|error| db_err("failed to export prompt references", error))?;
        let rows = stmt
            .query_map([id], |row| {
                Ok(PromptReferenceRecord {
                    source_prompt_id: row.get(0)?,
                    target_prompt_id: row.get(1)?,
                    token_title: row.get(2)?,
                    resolution: row.get(3)?,
                })
            })
            .map_err(|error| db_err("failed to query exported references", error))?;
        for row in rows {
            records.push(row.map_err(|error| db_err("failed to map exported reference", error))?);
        }
    }
    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::prompt::{self, PromptCreate, PromptUpdate};
    use crate::storage::{create_memory_pool, init_schema};
    use std::sync::Mutex;

    fn conn() -> r2d2::PooledConnection<r2d2_sqlite::SqliteConnectionManager> {
        let pool = create_memory_pool().unwrap();
        let conn = pool.get().unwrap();
        init_schema(&conn).unwrap();
        conn
    }

    fn create_titled(conn: &Connection, title: &str, body: &str) -> Prompt {
        prompt::create(
            conn,
            PromptCreate {
                title: title.into(),
                user_prompt: body.into(),
                description: None,
                prompt_type: None,
                type_definition_id: None,
                system_prompt: None,
                messages: None,
                variables: None,
                tags: None,
                folder_id: None,
                images: None,
                videos: None,
                is_favorite: None,
                is_pinned: None,
                is_private: None,
                usage_count: None,
                source: None,
                notes: None,
            },
        )
        .unwrap()
    }

    #[test]
    fn extract_tokens_reads_explicit_then_shorthand() {
        assert_eq!(
            extract_tokens("see @@Alpha@@ and @@Beta"),
            vec!["Alpha".to_string(), "Beta".to_string()]
        );
        assert_eq!(extract_tokens("@@"), Vec::<String>::new());
        assert_eq!(extract_tokens("@@@@"), Vec::<String>::new());
        assert_eq!(extract_tokens("start @@\nend"), Vec::<String>::new());
        assert_eq!(
            extract_tokens("use {{name}} and @@Title@@"),
            vec!["Title".to_string()]
        );
        assert_eq!(extract_tokens("@@foo{{x}}@@"), vec!["foo{{x}}".to_string()]);
        assert_eq!(
            extract_tokens("@@Alpha@@ @@Alpha"),
            vec!["Alpha".to_string()]
        );
    }

    #[test]
    fn resolve_and_store_writes_resolved_and_missing() {
        let conn = conn();
        let target = create_titled(&conn, "A", "body-a");
        let source = create_titled(&conn, "S", "see @@A@@ and @@Missing");
        let listed = list(&conn, &source.id).unwrap();
        assert_eq!(listed.outgoing.len(), 2);
        let resolved = listed
            .outgoing
            .iter()
            .find(|item| item.token_title == "A")
            .unwrap();
        assert_eq!(resolved.resolution, "resolved");
        assert_eq!(
            resolved.target_prompt_id.as_deref(),
            Some(target.id.as_str())
        );
        let missing = listed
            .outgoing
            .iter()
            .find(|item| item.token_title == "Missing")
            .unwrap();
        assert_eq!(missing.resolution, "missing");

        prompt::update(
            &conn,
            &source.id,
            PromptUpdate {
                user_prompt: Some("see @@A@@ and @@Missing".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(list(&conn, &source.id).unwrap().outgoing.len(), 2);
    }

    #[test]
    fn private_prompt_writes_the_same_edges() {
        let conn = conn();
        let encryption = Mutex::new(crate::state::EncryptionState::default());
        crate::services::security::set_master_password(&conn, &encryption, "password123").unwrap();
        let target = create_titled(&conn, "A", "body-a");
        let source = prompt::create_secure(
            &conn,
            &encryption,
            PromptCreate {
                title: "PrivateS".into(),
                user_prompt: "see @@A@@ and @@Missing".into(),
                is_private: Some(true),
                ..Default::default()
            },
        )
        .unwrap();
        let listed = list(&conn, &source.id).unwrap();
        assert_eq!(listed.outgoing.len(), 2);
        assert_eq!(
            listed
                .outgoing
                .iter()
                .find(|item| item.token_title == "A")
                .unwrap()
                .target_prompt_id
                .as_deref(),
            Some(target.id.as_str())
        );
    }

    #[test]
    fn delete_marks_incoming_missing_and_keeps_source_readable() {
        let conn = conn();
        let target = create_titled(&conn, "A", "body-a");
        let source = create_titled(&conn, "S", "see @@A");
        prompt::delete(&conn, &target.id).unwrap();
        let listed = list(&conn, &source.id).unwrap();
        assert_eq!(listed.outgoing[0].resolution, "missing");
        assert!(listed.outgoing[0].target_prompt_id.is_none());
        assert_eq!(prompt::get(&conn, &source.id).unwrap().title, "S");
    }

    #[test]
    fn duplicate_resolves_edges_under_the_new_id() {
        let conn = conn();
        let target = create_titled(&conn, "A", "body-a");
        let source = create_titled(&conn, "S", "see @@A");
        let copy = prompt::duplicate(&conn, &source.id).unwrap();
        let listed = list(&conn, &copy.id).unwrap();
        assert_eq!(listed.outgoing.len(), 1);
        assert_eq!(listed.outgoing[0].resolution, "resolved");
        assert_eq!(
            listed.outgoing[0].target_prompt_id.as_deref(),
            Some(target.id.as_str())
        );
        assert_ne!(copy.id, source.id);
    }

    #[test]
    fn expand_two_level_chain_and_four_level_hits_depth() {
        let conn = conn();
        create_titled(&conn, "C", "leaf");
        create_titled(&conn, "B", "B-@@C@@");
        let a = create_titled(&conn, "A", "A-@@B@@");
        let copied = prompt::copy(&conn, &a.id, &Default::default()).unwrap();
        assert_eq!(copied.user_prompt, "A-B-leaf");
        assert!(copied.unexpanded.is_empty());

        create_titled(&conn, "E", "e");
        create_titled(&conn, "D4", "@@E");
        create_titled(&conn, "C4", "@@D4");
        create_titled(&conn, "B4", "@@C4");
        let a4 = create_titled(&conn, "A4", "@@B4");
        let deep = prompt::copy(&conn, &a4.id, &Default::default()).unwrap();
        assert_eq!(deep.user_prompt, "@@E");
        assert_eq!(deep.unexpanded[0].reason, "depth");
        assert_eq!(MAX_EXPAND_DEPTH, 3);
    }

    #[test]
    fn expand_cycle_diamond_rename_and_locked_target() {
        let conn = conn();
        let b = create_titled(&conn, "CB", "placeholder");
        let a = create_titled(&conn, "CA", "@@CB");
        prompt::update(
            &conn,
            &b.id,
            PromptUpdate {
                user_prompt: Some("@@CA".into()),
                ..Default::default()
            },
        )
        .unwrap();
        let cycled = prompt::copy(&conn, &a.id, &Default::default()).unwrap();
        assert_eq!(cycled.user_prompt, "@@CA");
        assert_eq!(cycled.unexpanded[0].reason, "cycle");

        create_titled(&conn, "D", "diamond");
        create_titled(&conn, "Left", "@@D");
        create_titled(&conn, "Right", "@@D");
        let top = create_titled(&conn, "Top", "@@Left@@ and @@Right@@");
        let diamond = prompt::copy(&conn, &top.id, &Default::default()).unwrap();
        assert_eq!(diamond.user_prompt, "diamond and diamond");

        let foo = create_titled(&conn, "Foo", "hello");
        let src = create_titled(&conn, "Ren", "@@Foo");
        prompt::update(
            &conn,
            &foo.id,
            PromptUpdate {
                title: Some("Bar".into()),
                ..Default::default()
            },
        )
        .unwrap();
        let renamed = list(&conn, &src.id).unwrap();
        assert_eq!(renamed.outgoing[0].target_title.as_deref(), Some("Bar"));
        let copied = prompt::copy(&conn, &src.id, &Default::default()).unwrap();
        assert_eq!(copied.user_prompt, "hello");

        let encryption = Mutex::new(crate::state::EncryptionState::default());
        crate::services::security::set_master_password(&conn, &encryption, "password123").unwrap();
        let private = prompt::create_secure(
            &conn,
            &encryption,
            PromptCreate {
                title: "Secret".into(),
                user_prompt: "classified".into(),
                is_private: Some(true),
                ..Default::default()
            },
        )
        .unwrap();
        let locker = create_titled(&conn, "LockSrc", "@@Secret");
        let unlocked =
            prompt::copy_secure(&conn, &encryption, &locker.id, &Default::default()).unwrap();
        assert_eq!(unlocked.user_prompt, "classified");
        crate::services::security::lock(&encryption).unwrap();
        let locked =
            prompt::copy_secure(&conn, &encryption, &locker.id, &Default::default()).unwrap();
        assert_eq!(locked.user_prompt, "@@Secret");
        assert_eq!(locked.unexpanded[0].reason, "locked");
        assert_eq!(private.title, "Secret");
    }

    #[test]
    fn copy_expands_then_substitutes_including_messages() {
        let conn = conn();
        create_titled(&conn, "T", "Hi {{name}}");
        let source = prompt::create(
            &conn,
            PromptCreate {
                title: "Src".into(),
                user_prompt: "@@T".into(),
                ..Default::default()
            },
        )
        .unwrap();
        let mut values = std::collections::HashMap::new();
        values.insert("name".into(), "Ada".into());
        let copied = prompt::copy(&conn, &source.id, &values).unwrap();
        assert_eq!(copied.user_prompt, "Hi Ada");

        let chat = prompt::create(
            &conn,
            PromptCreate {
                title: "Chat".into(),
                user_prompt: String::new(),
                messages: Some(vec![crate::models::PromptMessage {
                    role: "user".into(),
                    content: "@@T".into(),
                }]),
                ..Default::default()
            },
        )
        .unwrap();
        let copied_chat = prompt::copy(&conn, &chat.id, &values).unwrap();
        assert_eq!(copied_chat.messages[0].content, "Hi Ada");
    }
}
