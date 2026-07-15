use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::models::{PromptType, PromptTypeDefinition};
use crate::storage::mapping::prompt_type_definition_from_row;
use crate::storage::time::now_millis;

pub const MAX_TYPE_NAME_CHARS: usize = 100;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptTypeCreate {
    pub name: String,
    pub base_kind: String,
}

fn db_err(context: &str, error: rusqlite::Error) -> AppError {
    AppError::internal(format!("{context}: {error}"))
}

pub(crate) fn normalize_name(name: &str) -> String {
    name.trim().to_lowercase()
}

pub(crate) fn parse_base_kind(raw: &str) -> Result<PromptType, AppError> {
    serde_json::from_value(serde_json::Value::String(raw.to_owned())).map_err(|_| {
        AppError::validation(format!(
            "invalid baseKind `{raw}`; expected one of `text`, `image`, `video`"
        ))
    })
}

pub(crate) fn base_kind_wire(base_kind: PromptType) -> &'static str {
    match base_kind {
        PromptType::Text => "text",
        PromptType::Image => "image",
        PromptType::Video => "video",
    }
}

pub fn list(conn: &Connection) -> Result<Vec<PromptTypeDefinition>, AppError> {
    let mut statement = conn
        .prepare("SELECT id,name,base_kind,created_at FROM prompt_type_definitions ORDER BY created_at,id")
        .map_err(|error| db_err("failed to prepare prompt type list", error))?;
    let rows = statement
        .query_map([], prompt_type_definition_from_row)
        .map_err(|error| db_err("failed to query prompt types", error))?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| db_err("failed to map prompt types", error))
}

pub fn get(conn: &Connection, id: &str) -> Result<PromptTypeDefinition, AppError> {
    conn.query_row(
        "SELECT id,name,base_kind,created_at FROM prompt_type_definitions WHERE id=?1",
        [id],
        prompt_type_definition_from_row,
    )
    .optional()
    .map_err(|error| db_err("failed to read prompt type", error))?
    .ok_or_else(|| AppError::not_found(format!("prompt type definition `{id}` not found")))
}

pub fn create(
    conn: &Connection,
    input: PromptTypeCreate,
) -> Result<PromptTypeDefinition, AppError> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err(AppError::validation("prompt type name is required"));
    }
    if name.chars().count() > MAX_TYPE_NAME_CHARS {
        return Err(AppError::validation(format!(
            "prompt type name exceeds the {MAX_TYPE_NAME_CHARS}-character limit"
        )));
    }
    let base_kind = parse_base_kind(&input.base_kind)?;
    let normalized_name = normalize_name(name);
    let id = uuid::Uuid::new_v4().to_string();
    let created_at = now_millis();
    let tx = conn
        .unchecked_transaction()
        .map_err(|error| db_err("failed to begin prompt type transaction", error))?;
    tx.execute(
        "INSERT INTO prompt_type_definitions (id,name,normalized_name,base_kind,created_at) VALUES (?1,?2,?3,?4,?5)",
        params![id, name, normalized_name, base_kind_wire(base_kind), created_at],
    )
    .map_err(|error| match &error {
        rusqlite::Error::SqliteFailure(details, _)
            if details.code == rusqlite::ErrorCode::ConstraintViolation =>
        {
            AppError::conflict(format!("prompt type name `{name}` already exists"))
        }
        _ => db_err("failed to create prompt type", error),
    })?;
    let definition = get(&tx, &id)?;
    tx.commit()
        .map_err(|error| db_err("failed to commit prompt type transaction", error))?;
    Ok(definition)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{create_memory_pool, init_schema};

    #[test]
    fn creates_lists_and_normalizes_unique_names() {
        let pool = create_memory_pool().unwrap();
        let conn = pool.get().unwrap();
        init_schema(&conn).unwrap();
        let created = create(
            &conn,
            PromptTypeCreate {
                name: "  Marketing Copy  ".into(),
                base_kind: "text".into(),
            },
        )
        .unwrap();
        assert_eq!(created.name, "Marketing Copy");
        assert_eq!(created.base_kind, PromptType::Text);
        assert_eq!(list(&conn).unwrap(), vec![created]);

        let error = create(
            &conn,
            PromptTypeCreate {
                name: "marketing copy".into(),
                base_kind: "image".into(),
            },
        )
        .unwrap_err();
        assert_eq!(error.code_str(), "CONFLICT");
        assert_eq!(list(&conn).unwrap().len(), 1);
    }

    #[test]
    fn rejects_empty_long_and_unknown_base_without_writes() {
        let pool = create_memory_pool().unwrap();
        let conn = pool.get().unwrap();
        init_schema(&conn).unwrap();
        for input in [
            PromptTypeCreate {
                name: " ".into(),
                base_kind: "text".into(),
            },
            PromptTypeCreate {
                name: "x".repeat(MAX_TYPE_NAME_CHARS + 1),
                base_kind: "text".into(),
            },
            PromptTypeCreate {
                name: "Audio".into(),
                base_kind: "audio".into(),
            },
        ] {
            assert_eq!(create(&conn, input).unwrap_err().code_str(), "VALIDATION");
        }
        assert!(list(&conn).unwrap().is_empty());
    }
}
