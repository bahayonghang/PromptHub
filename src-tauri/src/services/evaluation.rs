//! Local prompt execution, deterministic evaluation, caching, and labels.

use std::collections::BTreeMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Mutex;
use std::time::Duration;

use futures_util::StreamExt;
use regex::Regex;
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tokio_util::sync::CancellationToken;

use crate::error::AppError;
use crate::models::*;
use crate::state::EncryptionState;
use crate::storage::mapping::prompt_version_from_row;
use crate::storage::time::{millis_to_iso8601, now_millis};
use crate::storage::DbPool;

pub const RUNTIME_VERSION: &str = "evaluation-v1";
const PROVIDER_TIMEOUT: Duration = Duration::from_secs(120);
const MAX_REDIRECTS: usize = 5;

fn db_err(context: &str, error: rusqlite::Error) -> AppError {
    AppError::internal(format!("{context}: {error}"))
}

fn encode<T: serde::Serialize>(value: &T) -> Result<String, AppError> {
    serde_json::to_string(value)
        .map_err(|error| AppError::internal(format!("failed to encode evaluation data: {error}")))
}

fn decode<T: serde::de::DeserializeOwned>(raw: &str) -> rusqlite::Result<T> {
    serde_json::from_str(raw).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
    })
}

fn required_key(encryption: &Mutex<EncryptionState>) -> Result<Vec<u8>, AppError> {
    crate::services::security::unlocked_key(encryption)?.ok_or_else(|| {
        AppError::unauthorized("unlock the library before storing or using provider credentials")
    })
}

fn validate_profile(input: &ExecutionProfileInput) -> Result<(), AppError> {
    if input.name.trim().is_empty() {
        return Err(AppError::validation("profile name is required"));
    }
    if input.model.trim().is_empty() {
        return Err(AppError::validation("profile model is required"));
    }
    if !input.parameters.is_object() {
        return Err(AppError::validation("profile parameters must be an object"));
    }
    match input.provider.as_str() {
        "mock" => Ok(()),
        "openai-compatible" => {
            if input.endpoint.as_deref().unwrap_or("").trim().is_empty() {
                Err(AppError::validation(
                    "openai-compatible profiles require an endpoint",
                ))
            } else {
                Ok(())
            }
        }
        _ => Err(AppError::validation(
            "provider must be mock or openai-compatible",
        )),
    }
}

fn profile_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ExecutionProfileRevision> {
    let parameters: String = row.get("parameters")?;
    let credential: Option<String> = row.get("credential")?;
    Ok(ExecutionProfileRevision {
        id: row.get("id")?,
        profile_id: row.get("profile_id")?,
        revision: row.get("revision")?,
        name: row.get("name")?,
        provider: row.get("provider")?,
        endpoint: row.get("endpoint")?,
        model: row.get("model")?,
        parameters: decode(&parameters)?,
        has_credential: credential.is_some(),
        created_at: millis_to_iso8601(row.get("created_at")?),
    })
}

pub fn create_profile(
    conn: &Connection,
    encryption: &Mutex<EncryptionState>,
    mut input: ExecutionProfileInput,
) -> Result<ExecutionProfileRevision, AppError> {
    validate_profile(&input)?;
    let profile_id = input
        .profile_id
        .take()
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let latest: Option<(i64, Option<String>)> = conn
        .query_row(
            "SELECT revision, credential FROM execution_profile_revisions WHERE profile_id = ?1 ORDER BY revision DESC LIMIT 1",
            [&profile_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|error| db_err("failed to read profile history", error))?;
    let revision = latest.as_ref().map_or(1, |(revision, _)| revision + 1);
    let credential = match input.credential.take() {
        Some(secret) if !secret.is_empty() => Some(crate::services::security::encrypt(
            &secret,
            &required_key(encryption)?,
        )?),
        Some(_) => None,
        None => latest.and_then(|(_, credential)| credential),
    };
    let id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO execution_profile_revisions (id,profile_id,revision,name,provider,endpoint,model,parameters,credential,created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
        params![
            id,
            profile_id,
            revision,
            input.name.trim(),
            input.provider,
            input.endpoint.filter(|value| !value.trim().is_empty()),
            input.model.trim(),
            encode(&input.parameters)?,
            credential,
            now_millis(),
        ],
    )
    .map_err(|error| db_err("failed to create execution profile revision", error))?;
    get_profile(conn, &id)
}

pub fn list_profiles(conn: &Connection) -> Result<Vec<ExecutionProfileRevision>, AppError> {
    let mut stmt = conn
        .prepare(
            "SELECT * FROM execution_profile_revisions ORDER BY name, profile_id, revision DESC",
        )
        .map_err(|error| db_err("failed to prepare execution profiles", error))?;
    let profiles = stmt
        .query_map([], profile_from_row)
        .map_err(|error| db_err("failed to query execution profiles", error))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| db_err("failed to map execution profiles", error))?;
    Ok(profiles)
}

pub fn get_profile(conn: &Connection, id: &str) -> Result<ExecutionProfileRevision, AppError> {
    conn.query_row(
        "SELECT * FROM execution_profile_revisions WHERE id = ?1",
        [id],
        profile_from_row,
    )
    .optional()
    .map_err(|error| db_err("failed to read execution profile", error))?
    .ok_or_else(|| AppError::not_found(format!("execution profile revision `{id}` not found")))
}

#[derive(Clone)]
pub struct ProviderRequest {
    pub provider: String,
    pub endpoint: Option<String>,
    pub model: String,
    pub parameters: Value,
    pub credential: Option<String>,
    pub messages: Vec<PromptMessage>,
}

fn provider_request(
    conn: &Connection,
    encryption: &Mutex<EncryptionState>,
    profile_id: &str,
    messages: Vec<PromptMessage>,
) -> Result<ProviderRequest, AppError> {
    let (provider, endpoint, model, parameters, credential): (
        String,
        Option<String>,
        String,
        String,
        Option<String>,
    ) = conn
        .query_row(
            "SELECT provider,endpoint,model,parameters,credential FROM execution_profile_revisions WHERE id = ?1",
            [profile_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
        )
        .optional()
        .map_err(|error| db_err("failed to read provider profile", error))?
        .ok_or_else(|| AppError::not_found(format!("execution profile revision `{profile_id}` not found")))?;
    let credential = match credential {
        Some(encrypted) => Some(crate::services::security::decrypt(
            &encrypted,
            &required_key(encryption)?,
        )?),
        None => None,
    };
    Ok(ProviderRequest {
        provider,
        endpoint,
        model,
        parameters: serde_json::from_str(&parameters)
            .map_err(|error| AppError::internal(format!("invalid profile parameters: {error}")))?,
        credential,
        messages,
    })
}

fn get_revision(
    conn: &Connection,
    encryption: &Mutex<EncryptionState>,
    id: &str,
) -> Result<PromptVersion, AppError> {
    let revision = conn
        .query_row(
            "SELECT * FROM prompt_versions WHERE id = ?1",
            [id],
            prompt_version_from_row,
        )
        .optional()
        .map_err(|error| db_err("failed to read prompt revision", error))?
        .ok_or_else(|| AppError::not_found(format!("prompt revision `{id}` not found")))?;
    let key = crate::services::security::unlocked_key(encryption)?;
    crate::services::prompt::present_version(revision, key.as_deref())
}

pub fn render_prompt(
    conn: &Connection,
    encryption: &Mutex<EncryptionState>,
    revision_id: &str,
    inputs: &BTreeMap<String, String>,
) -> Result<RenderedPrompt, AppError> {
    let revision = get_revision(conn, encryption, revision_id)?;
    for variable in revision
        .variables
        .iter()
        .filter(|variable| variable.required)
    {
        let value = inputs
            .get(&variable.name)
            .cloned()
            .or_else(|| variable.default_value.clone())
            .unwrap_or_default();
        if value.trim().is_empty() {
            return Err(AppError::validation(format!(
                "required variable `{}` is missing",
                variable.name
            )));
        }
    }
    let values = revision
        .variables
        .iter()
        .filter_map(|variable| {
            inputs
                .get(&variable.name)
                .cloned()
                .or_else(|| variable.default_value.clone())
                .map(|value| (variable.name.clone(), value))
        })
        .collect();
    let source = if revision.messages.is_empty() {
        let mut messages = Vec::new();
        if let Some(system) = revision.system_prompt.filter(|value| !value.is_empty()) {
            messages.push(PromptMessage {
                role: "system".into(),
                content: system,
            });
        }
        messages.push(PromptMessage {
            role: "user".into(),
            content: revision.user_prompt,
        });
        messages
    } else {
        revision.messages
    };
    Ok(RenderedPrompt {
        prompt_revision_id: revision_id.to_string(),
        messages: source
            .into_iter()
            .map(|mut message| {
                message.content =
                    crate::services::prompt::substitute_placeholders(&message.content, &values);
                message
            })
            .collect(),
    })
}

pub fn save_test_set(conn: &Connection, input: TestSetInput) -> Result<TestSet, AppError> {
    if input.name.trim().is_empty() {
        return Err(AppError::validation("test set name is required"));
    }
    if input.cases.len() > 1000 {
        return Err(AppError::validation("test set exceeds 1000 cases"));
    }
    for case in &input.cases {
        if case.name.trim().is_empty() {
            return Err(AppError::validation("test case name is required"));
        }
        if !case.annotations.is_object() {
            return Err(AppError::validation(
                "test case annotations must be an object",
            ));
        }
    }
    let id = input.id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let now = now_millis();
    let tx = conn
        .unchecked_transaction()
        .map_err(|error| db_err("failed to start test set save", error))?;
    let created_at: Option<i64> = tx
        .query_row(
            "SELECT created_at FROM test_sets WHERE id = ?1",
            [&id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| db_err("failed to read test set", error))?;
    tx.execute(
        "INSERT INTO test_sets (id,name,created_at,updated_at) VALUES (?1,?2,?3,?4) ON CONFLICT(id) DO UPDATE SET name=excluded.name,updated_at=excluded.updated_at",
        params![id, input.name.trim(), created_at.unwrap_or(now), now],
    )
    .map_err(|error| db_err("failed to save test set", error))?;
    tx.execute("DELETE FROM test_cases WHERE test_set_id = ?1", [&id])
        .map_err(|error| db_err("failed to replace test cases", error))?;
    for (index, case) in input.cases.into_iter().enumerate() {
        tx.execute(
            "INSERT INTO test_cases (id,test_set_id,name,inputs,expected_output,annotations,sort_order) VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![
                case.id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
                id,
                case.name.trim(),
                encode(&case.inputs)?,
                case.expected_output,
                encode(&case.annotations)?,
                index as i64,
            ],
        )
        .map_err(|error| db_err("failed to save test case", error))?;
    }
    tx.commit()
        .map_err(|error| db_err("failed to commit test set", error))?;
    get_test_set(conn, &id)
}

fn test_case_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<TestCase> {
    let inputs: String = row.get("inputs")?;
    let annotations: String = row.get("annotations")?;
    Ok(TestCase {
        id: row.get("id")?,
        name: row.get("name")?,
        inputs: decode(&inputs)?,
        expected_output: row.get("expected_output")?,
        annotations: decode(&annotations)?,
        sort_order: row.get("sort_order")?,
    })
}

pub fn get_test_set(conn: &Connection, id: &str) -> Result<TestSet, AppError> {
    let (name, created_at, updated_at): (String, i64, i64) = conn
        .query_row(
            "SELECT name,created_at,updated_at FROM test_sets WHERE id = ?1",
            [id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()
        .map_err(|error| db_err("failed to read test set", error))?
        .ok_or_else(|| AppError::not_found(format!("test set `{id}` not found")))?;
    let mut stmt = conn
        .prepare("SELECT * FROM test_cases WHERE test_set_id = ?1 ORDER BY sort_order,id")
        .map_err(|error| db_err("failed to prepare test cases", error))?;
    let cases = stmt
        .query_map([id], test_case_from_row)
        .map_err(|error| db_err("failed to query test cases", error))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| db_err("failed to map test cases", error))?;
    Ok(TestSet {
        id: id.to_string(),
        name,
        cases,
        created_at: millis_to_iso8601(created_at),
        updated_at: millis_to_iso8601(updated_at),
    })
}

pub fn list_test_sets(conn: &Connection) -> Result<Vec<TestSet>, AppError> {
    let mut stmt = conn
        .prepare("SELECT id FROM test_sets ORDER BY updated_at DESC,id")
        .map_err(|error| db_err("failed to prepare test sets", error))?;
    let ids = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|error| db_err("failed to query test sets", error))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| db_err("failed to map test sets", error))?;
    ids.into_iter().map(|id| get_test_set(conn, &id)).collect()
}

pub fn export_test_set(conn: &Connection, id: &str) -> Result<String, AppError> {
    encode(&get_test_set(conn, id)?)
}

pub fn import_test_set(conn: &Connection, raw: &str) -> Result<TestSet, AppError> {
    let imported: TestSet = serde_json::from_str(raw)
        .map_err(|error| AppError::validation(format!("invalid test set JSON: {error}")))?;
    save_test_set(
        conn,
        TestSetInput {
            id: None,
            name: imported.name,
            cases: imported
                .cases
                .into_iter()
                .map(|case| TestCaseInput {
                    id: None,
                    name: case.name,
                    inputs: case.inputs,
                    expected_output: case.expected_output,
                    annotations: case.annotations,
                })
                .collect(),
        },
    )
}

fn evaluator_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<EvaluatorConfig> {
    let config: String = row.get("config")?;
    Ok(EvaluatorConfig {
        id: row.get("id")?,
        name: row.get("name")?,
        kind: row.get("kind")?,
        config: decode(&config)?,
        created_at: millis_to_iso8601(row.get("created_at")?),
    })
}

pub fn create_evaluator(
    conn: &Connection,
    input: EvaluatorInput,
) -> Result<EvaluatorConfig, AppError> {
    if input.name.trim().is_empty() {
        return Err(AppError::validation("evaluator name is required"));
    }
    if !matches!(
        input.kind.as_str(),
        "manual" | "exact" | "contains" | "regex" | "numeric"
    ) {
        return Err(AppError::validation("unsupported evaluator kind"));
    }
    if !input.config.is_object() {
        return Err(AppError::validation("evaluator config must be an object"));
    }
    if input.kind == "regex" {
        let pattern = input
            .config
            .get("pattern")
            .and_then(Value::as_str)
            .unwrap_or("");
        Regex::new(pattern)
            .map_err(|error| AppError::validation(format!("invalid evaluator regex: {error}")))?;
    }
    let id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO evaluator_configs (id,name,kind,config,created_at) VALUES (?1,?2,?3,?4,?5)",
        params![
            id,
            input.name.trim(),
            input.kind,
            encode(&input.config)?,
            now_millis()
        ],
    )
    .map_err(|error| db_err("failed to create evaluator", error))?;
    conn.query_row(
        "SELECT * FROM evaluator_configs WHERE id = ?1",
        [&id],
        evaluator_from_row,
    )
    .map_err(|error| db_err("failed to read evaluator", error))
}

pub fn list_evaluators(conn: &Connection) -> Result<Vec<EvaluatorConfig>, AppError> {
    let mut stmt = conn
        .prepare("SELECT * FROM evaluator_configs ORDER BY created_at,id")
        .map_err(|error| db_err("failed to prepare evaluators", error))?;
    let evaluators = stmt
        .query_map([], evaluator_from_row)
        .map_err(|error| db_err("failed to query evaluators", error))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| db_err("failed to map evaluators", error))?;
    Ok(evaluators)
}

pub fn evaluate_output(
    evaluator: &EvaluatorConfig,
    output: &str,
    expected: Option<&str>,
) -> Result<EvaluationResult, AppError> {
    let (passed, score, skipped, evidence) = match evaluator.kind.as_str() {
        "manual" => (None, None, true, "Awaiting manual review".to_string()),
        "exact" => {
            let target = evaluator
                .config
                .get("value")
                .and_then(Value::as_str)
                .or(expected);
            match target {
                Some(target) => {
                    let passed = output == target;
                    (
                        Some(passed),
                        Some(if passed { 1.0 } else { 0.0 }),
                        false,
                        format!("Expected exact output `{target}`"),
                    )
                }
                None => (None, None, true, "No expected output configured".into()),
            }
        }
        "contains" => {
            let needle = evaluator
                .config
                .get("value")
                .and_then(Value::as_str)
                .or(expected);
            match needle {
                Some(needle) => {
                    let passed = output.contains(needle);
                    (
                        Some(passed),
                        Some(if passed { 1.0 } else { 0.0 }),
                        false,
                        format!("Expected output to contain `{needle}`"),
                    )
                }
                None => (None, None, true, "No contains value configured".into()),
            }
        }
        "regex" => {
            let pattern = evaluator
                .config
                .get("pattern")
                .and_then(Value::as_str)
                .unwrap_or("");
            let passed = Regex::new(pattern)
                .map_err(|error| AppError::validation(format!("invalid evaluator regex: {error}")))?
                .is_match(output);
            (
                Some(passed),
                Some(if passed { 1.0 } else { 0.0 }),
                false,
                format!("Pattern `{pattern}`"),
            )
        }
        "numeric" => {
            let value: f64 = output
                .trim()
                .parse()
                .map_err(|_| AppError::validation("numeric evaluator output is not a number"))?;
            let threshold = evaluator
                .config
                .get("threshold")
                .and_then(Value::as_f64)
                .ok_or_else(|| AppError::validation("numeric evaluator threshold is required"))?;
            let operator = evaluator
                .config
                .get("operator")
                .and_then(Value::as_str)
                .unwrap_or("gte");
            let passed = match operator {
                "gt" => value > threshold,
                "gte" => value >= threshold,
                "lt" => value < threshold,
                "lte" => value <= threshold,
                "eq" => (value - threshold).abs() < f64::EPSILON,
                _ => return Err(AppError::validation("invalid numeric evaluator operator")),
            };
            (
                Some(passed),
                Some(value),
                false,
                format!("Observed {value}; required {operator} {threshold}"),
            )
        }
        _ => return Err(AppError::validation("unsupported evaluator kind")),
    };
    Ok(EvaluationResult {
        evaluator_id: evaluator.id.clone(),
        kind: evaluator.kind.clone(),
        passed,
        score,
        skipped,
        evidence,
    })
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProviderOutput {
    pub content: String,
    pub usage: Option<Value>,
}

pub trait EvaluationEventSink: Send + Sync {
    fn emit_run_chunk(&self, run_id: &str, chunk: &str);
    fn emit_run_terminal(&self, run_id: &str, status: &str);
    fn emit_matrix_progress(
        &self,
        evaluation_run_id: &str,
        completed: i64,
        total: i64,
        cell_id: &str,
    );
}

pub trait ProviderAdapter: Send + Sync {
    fn execute<'a>(
        &'a self,
        request: ProviderRequest,
        cancel: &'a CancellationToken,
        run_id: &'a str,
        sink: &'a dyn EvaluationEventSink,
    ) -> Pin<Box<dyn Future<Output = Result<ProviderOutput, AppError>> + Send + 'a>>;
}

pub struct DefaultProviderAdapter;

impl ProviderAdapter for DefaultProviderAdapter {
    fn execute<'a>(
        &'a self,
        request: ProviderRequest,
        cancel: &'a CancellationToken,
        run_id: &'a str,
        sink: &'a dyn EvaluationEventSink,
    ) -> Pin<Box<dyn Future<Output = Result<ProviderOutput, AppError>> + Send + 'a>> {
        Box::pin(async move {
            match request.provider.as_str() {
                "mock" => execute_mock(request, cancel, run_id, sink).await,
                "openai-compatible" => execute_openai(request, cancel, run_id, sink).await,
                _ => Err(AppError::validation("unsupported provider adapter")),
            }
        })
    }
}

async fn execute_mock(
    request: ProviderRequest,
    cancel: &CancellationToken,
    run_id: &str,
    sink: &dyn EvaluationEventSink,
) -> Result<ProviderOutput, AppError> {
    if cancel.is_cancelled() {
        return Err(AppError::internal("request cancelled"));
    }
    let content = request
        .parameters
        .get("response")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .unwrap_or_else(|| {
            request
                .messages
                .iter()
                .rev()
                .find(|message| message.role == "user")
                .map(|message| message.content.clone())
                .unwrap_or_default()
        });
    let midpoint = content.len() / 2;
    let midpoint = content
        .char_indices()
        .map(|(index, _)| index)
        .find(|index| *index >= midpoint)
        .unwrap_or(content.len());
    for chunk in [&content[..midpoint], &content[midpoint..]] {
        if cancel.is_cancelled() {
            return Err(AppError::internal("request cancelled"));
        }
        if !chunk.is_empty() {
            sink.emit_run_chunk(run_id, chunk);
        }
    }
    Ok(ProviderOutput {
        content,
        usage: Some(json!({ "source": "mock" })),
    })
}

fn process_openai_stream_line(
    line: &str,
    content: &mut String,
    usage: &mut Option<Value>,
    run_id: &str,
    sink: &dyn EvaluationEventSink,
) -> Result<(), AppError> {
    let Some(data) = line.trim().strip_prefix("data:") else {
        return Ok(());
    };
    let data = data.trim();
    if data == "[DONE]" || data.is_empty() {
        return Ok(());
    }
    let value: Value = serde_json::from_str(data)
        .map_err(|error| AppError::network(format!("invalid provider stream event: {error}")))?;
    if let Some(chunk) = value
        .pointer("/choices/0/delta/content")
        .and_then(Value::as_str)
    {
        content.push_str(chunk);
        sink.emit_run_chunk(run_id, chunk);
    }
    if value.get("usage").is_some_and(|value| !value.is_null()) {
        *usage = value.get("usage").cloned();
    }
    Ok(())
}

async fn execute_openai(
    request: ProviderRequest,
    cancel: &CancellationToken,
    run_id: &str,
    sink: &dyn EvaluationEventSink,
) -> Result<ProviderOutput, AppError> {
    let mut body = request.parameters.as_object().cloned().unwrap_or_default();
    body.insert("model".into(), Value::String(request.model));
    body.insert(
        "messages".into(),
        serde_json::to_value(&request.messages)
            .map_err(|error| AppError::internal(format!("failed to encode messages: {error}")))?,
    );
    body.insert("stream".into(), Value::Bool(true));
    let body = serde_json::to_string(&body)
        .map_err(|error| AppError::internal(format!("failed to encode provider body: {error}")))?;
    let mut current = request
        .endpoint
        .ok_or_else(|| AppError::validation("provider endpoint is required"))?;

    for _ in 0..=MAX_REDIRECTS {
        let (url, client) =
            crate::services::network_safety::prepare_public_url(&current, PROVIDER_TIMEOUT).await?;
        let mut builder = client
            .post(url.clone())
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .header(reqwest::header::ACCEPT, "text/event-stream")
            .body(body.clone());
        if let Some(credential) = request.credential.as_deref() {
            builder = builder.bearer_auth(credential);
        }
        let response = tokio::select! {
            _ = cancel.cancelled() => return Err(AppError::internal("request cancelled")),
            result = builder.send() => result.map_err(|error| {
                if error.is_timeout() {
                    AppError::timeout("provider request timed out")
                } else {
                    AppError::network(format!("provider request failed: {error}"))
                }
            })?,
        };
        if response.status().is_redirection() {
            let location = response
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|value| value.to_str().ok())
                .ok_or_else(|| AppError::network("provider redirect omitted Location"))?;
            current = url
                .join(location)
                .map_err(|error| AppError::network(format!("invalid provider redirect: {error}")))?
                .to_string();
            continue;
        }
        if !response.status().is_success() {
            return Err(AppError::network(format!(
                "provider returned HTTP {}",
                response.status().as_u16()
            )));
        }

        let mut bytes = Vec::new();
        let mut parsed = 0;
        let mut content = String::new();
        let mut usage = None;
        let mut stream = response.bytes_stream();
        while let Some(chunk) = tokio::select! {
            _ = cancel.cancelled() => return Err(AppError::internal("request cancelled")),
            chunk = stream.next() => chunk,
        } {
            let chunk = chunk.map_err(|error| {
                AppError::network(format!("failed to read provider stream: {error}"))
            })?;
            bytes.extend_from_slice(&chunk);
            while let Some(line_end) = bytes[parsed..].iter().position(|byte| *byte == b'\n') {
                let line_end = parsed + line_end;
                let line = std::str::from_utf8(&bytes[parsed..line_end])
                    .map_err(|_| AppError::network("provider returned invalid UTF-8"))?;
                process_openai_stream_line(line, &mut content, &mut usage, run_id, sink)?;
                parsed = line_end + 1;
            }
        }
        if parsed < bytes.len() {
            let line = std::str::from_utf8(&bytes[parsed..])
                .map_err(|_| AppError::network("provider returned invalid UTF-8"))?;
            process_openai_stream_line(line, &mut content, &mut usage, run_id, sink)?;
        }
        let raw = String::from_utf8(bytes)
            .map_err(|_| AppError::network("provider returned invalid UTF-8"))?;
        if content.is_empty() {
            let value: Value = serde_json::from_str(&raw).map_err(|error| {
                AppError::network(format!("invalid provider response: {error}"))
            })?;
            content = value
                .pointer("/choices/0/message/content")
                .and_then(Value::as_str)
                .ok_or_else(|| AppError::network("provider response omitted message content"))?
                .to_string();
            usage = value.get("usage").cloned();
            sink.emit_run_chunk(run_id, &content);
        }
        return Ok(ProviderOutput { content, usage });
    }
    Err(AppError::network("too many provider redirects"))
}

fn run_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<PromptRun> {
    let inputs: String = row.get("inputs")?;
    let rendered: String = row.get("rendered_messages")?;
    let usage: Option<String> = row.get("usage")?;
    Ok(PromptRun {
        id: row.get("id")?,
        prompt_revision_id: row.get("prompt_revision_id")?,
        profile_revision_id: row.get("profile_revision_id")?,
        test_case_id: row.get("test_case_id")?,
        inputs: decode(&inputs)?,
        rendered_messages: decode(&rendered)?,
        output: row.get("output")?,
        status: row.get("status")?,
        error: row.get("error")?,
        started_at: millis_to_iso8601(row.get("started_at")?),
        completed_at: row
            .get::<_, Option<i64>>("completed_at")?
            .map(millis_to_iso8601),
        duration_ms: row.get("duration_ms")?,
        usage: usage.map(|raw| decode(&raw)).transpose()?,
        cache_key: row.get("cache_key")?,
    })
}

pub fn get_run(conn: &Connection, id: &str) -> Result<PromptRun, AppError> {
    conn.query_row(
        "SELECT * FROM prompt_runs WHERE id = ?1",
        [id],
        run_from_row,
    )
    .optional()
    .map_err(|error| db_err("failed to read prompt run", error))?
    .ok_or_else(|| AppError::not_found(format!("prompt run `{id}` not found")))
}

pub fn list_runs(conn: &Connection) -> Result<Vec<PromptRun>, AppError> {
    let mut stmt = conn
        .prepare("SELECT * FROM prompt_runs ORDER BY started_at DESC,id DESC LIMIT 500")
        .map_err(|error| db_err("failed to prepare prompt runs", error))?;
    let runs = stmt
        .query_map([], run_from_row)
        .map_err(|error| db_err("failed to query prompt runs", error))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| db_err("failed to map prompt runs", error))?;
    Ok(runs)
}

pub async fn execute_run(
    pool: &DbPool,
    encryption: &Mutex<EncryptionState>,
    adapter: &dyn ProviderAdapter,
    input: PromptRunInput,
    cache_key: Option<String>,
    cancel: &CancellationToken,
    sink: &dyn EvaluationEventSink,
) -> Result<PromptRun, AppError> {
    let run_id = uuid::Uuid::new_v4().to_string();
    let started = now_millis();
    let (rendered, request) = {
        let conn = pool.get().map_err(|error| {
            AppError::io(format!("failed to acquire database connection: {error}"))
        })?;
        let rendered = render_prompt(&conn, encryption, &input.prompt_revision_id, &input.inputs)?;
        let request = provider_request(
            &conn,
            encryption,
            &input.profile_revision_id,
            rendered.messages.clone(),
        )?;
        conn.execute(
            "INSERT INTO prompt_runs (id,prompt_revision_id,profile_revision_id,test_case_id,inputs,rendered_messages,status,started_at,cache_key) VALUES (?1,?2,?3,?4,?5,?6,'running',?7,?8)",
            params![
                run_id,
                input.prompt_revision_id,
                input.profile_revision_id,
                input.test_case_id,
                encode(&input.inputs)?,
                encode(&rendered.messages)?,
                started,
                cache_key,
            ],
        )
        .map_err(|error| db_err("failed to create prompt run", error))?;
        (rendered, request)
    };
    let result = adapter.execute(request, cancel, &run_id, sink).await;
    let completed = now_millis();
    let (status, output, error, usage) = match result {
        Ok(output) if !cancel.is_cancelled() => (
            "success",
            Some(output.content),
            None,
            output.usage.map(|usage| encode(&usage)).transpose()?,
        ),
        _ if cancel.is_cancelled() => ("cancelled", None, None, None),
        Err(error) => ("error", None, Some(error.message), None),
        Ok(_) => ("cancelled", None, None, None),
    };
    let conn = pool
        .get()
        .map_err(|error| AppError::io(format!("failed to acquire database connection: {error}")))?;
    conn.execute(
        "UPDATE prompt_runs SET output=?1,status=?2,error=?3,completed_at=?4,duration_ms=?5,usage=?6 WHERE id=?7",
        params![output, status, error, completed, completed - started, usage, run_id],
    )
    .map_err(|error| db_err("failed to complete prompt run", error))?;
    sink.emit_run_terminal(&run_id, status);
    let _ = rendered;
    get_run(&conn, &run_id)
}

fn evaluator_by_id(conn: &Connection, id: &str) -> Result<EvaluatorConfig, AppError> {
    conn.query_row(
        "SELECT * FROM evaluator_configs WHERE id = ?1",
        [id],
        evaluator_from_row,
    )
    .optional()
    .map_err(|error| db_err("failed to read evaluator", error))?
    .ok_or_else(|| AppError::not_found(format!("evaluator `{id}` not found")))
}

fn cache_key(
    revision_id: &str,
    profile_id: &str,
    case: &TestCase,
    evaluators: &[EvaluatorConfig],
) -> Result<String, AppError> {
    let canonical = json!({
        "promptRevisionId": revision_id,
        "profileRevisionId": profile_id,
        "inputs": case.inputs,
        "expectedOutput": case.expected_output,
        "evaluators": evaluators,
        "runtimeVersion": RUNTIME_VERSION,
    });
    let digest = Sha256::digest(encode(&canonical)?.as_bytes());
    Ok(format!("{digest:x}"))
}

fn evaluation_run_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<EvaluationRun> {
    let revisions: String = row.get("prompt_revision_ids")?;
    let profiles: String = row.get("profile_revision_ids")?;
    let evaluators: String = row.get("evaluator_ids")?;
    Ok(EvaluationRun {
        id: row.get("id")?,
        test_set_id: row.get("test_set_id")?,
        prompt_revision_ids: decode(&revisions)?,
        profile_revision_ids: decode(&profiles)?,
        evaluator_ids: decode(&evaluators)?,
        status: row.get("status")?,
        total_cells: row.get("total_cells")?,
        completed_cells: row.get("completed_cells")?,
        failed_cells: row.get("failed_cells")?,
        started_at: millis_to_iso8601(row.get("started_at")?),
        completed_at: row
            .get::<_, Option<i64>>("completed_at")?
            .map(millis_to_iso8601),
        runtime_version: row.get("runtime_version")?,
    })
}

fn evaluation_cell_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<EvaluationCell> {
    let results: String = row.get("results")?;
    Ok(EvaluationCell {
        id: row.get("id")?,
        evaluation_run_id: row.get("evaluation_run_id")?,
        prompt_revision_id: row.get("prompt_revision_id")?,
        profile_revision_id: row.get("profile_revision_id")?,
        test_case_id: row.get("test_case_id")?,
        prompt_run_id: row.get("prompt_run_id")?,
        status: row.get("status")?,
        cache_hit: row.get("cache_hit")?,
        results: decode(&results)?,
        error: row.get("error")?,
        cache_key: row.get("cache_key")?,
        sort_order: row.get("sort_order")?,
    })
}

pub fn get_evaluation_run(conn: &Connection, id: &str) -> Result<EvaluationRunDetail, AppError> {
    let run = conn
        .query_row(
            "SELECT * FROM evaluation_runs WHERE id = ?1",
            [id],
            evaluation_run_from_row,
        )
        .optional()
        .map_err(|error| db_err("failed to read evaluation run", error))?
        .ok_or_else(|| AppError::not_found(format!("evaluation run `{id}` not found")))?;
    let mut stmt = conn
        .prepare(
            "SELECT * FROM evaluation_cells WHERE evaluation_run_id = ?1 ORDER BY sort_order,id",
        )
        .map_err(|error| db_err("failed to prepare evaluation cells", error))?;
    let cells = stmt
        .query_map([id], evaluation_cell_from_row)
        .map_err(|error| db_err("failed to query evaluation cells", error))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| db_err("failed to map evaluation cells", error))?;
    Ok(EvaluationRunDetail { run, cells })
}

pub fn list_evaluation_runs(conn: &Connection) -> Result<Vec<EvaluationRun>, AppError> {
    let mut stmt = conn
        .prepare("SELECT * FROM evaluation_runs ORDER BY started_at DESC,id DESC LIMIT 100")
        .map_err(|error| db_err("failed to prepare evaluation runs", error))?;
    let runs = stmt
        .query_map([], evaluation_run_from_row)
        .map_err(|error| db_err("failed to query evaluation runs", error))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| db_err("failed to map evaluation runs", error))?;
    Ok(runs)
}

pub async fn run_matrix(
    pool: &DbPool,
    encryption: &Mutex<EncryptionState>,
    adapter: &dyn ProviderAdapter,
    input: EvaluationMatrixInput,
    cancel: &CancellationToken,
    sink: &dyn EvaluationEventSink,
) -> Result<EvaluationRunDetail, AppError> {
    if input.prompt_revision_ids.is_empty()
        || input.profile_revision_ids.is_empty()
        || input.evaluator_ids.is_empty()
    {
        return Err(AppError::validation(
            "matrix requires prompt revisions, profiles, and evaluators",
        ));
    }
    let (test_set, evaluators) = {
        let conn = pool.get().map_err(|error| {
            AppError::io(format!("failed to acquire database connection: {error}"))
        })?;
        let test_set = get_test_set(&conn, &input.test_set_id)?;
        if test_set.cases.is_empty() {
            return Err(AppError::validation("matrix test set has no cases"));
        }
        let evaluators = input
            .evaluator_ids
            .iter()
            .map(|id| evaluator_by_id(&conn, id))
            .collect::<Result<Vec<_>, _>>()?;
        for revision in &input.prompt_revision_ids {
            get_revision(&conn, encryption, revision)?;
        }
        for profile in &input.profile_revision_ids {
            get_profile(&conn, profile)?;
        }
        (test_set, evaluators)
    };
    let total = (input.prompt_revision_ids.len()
        * input.profile_revision_ids.len()
        * test_set.cases.len()) as i64;
    let evaluation_run_id = uuid::Uuid::new_v4().to_string();
    {
        let conn = pool.get().map_err(|error| {
            AppError::io(format!("failed to acquire database connection: {error}"))
        })?;
        let tx = conn
            .unchecked_transaction()
            .map_err(|error| db_err("failed to start evaluation matrix", error))?;
        tx.execute(
            "INSERT INTO evaluation_runs (id,test_set_id,prompt_revision_ids,profile_revision_ids,evaluator_ids,status,total_cells,started_at,runtime_version) VALUES (?1,?2,?3,?4,?5,'running',?6,?7,?8)",
            params![
                evaluation_run_id,
                input.test_set_id,
                encode(&input.prompt_revision_ids)?,
                encode(&input.profile_revision_ids)?,
                encode(&input.evaluator_ids)?,
                total,
                now_millis(),
                RUNTIME_VERSION,
            ],
        )
        .map_err(|error| db_err("failed to create evaluation run", error))?;
        let mut order = 0_i64;
        for revision in &input.prompt_revision_ids {
            for profile in &input.profile_revision_ids {
                for case in &test_set.cases {
                    let key = cache_key(revision, profile, case, &evaluators)?;
                    tx.execute(
                        "INSERT INTO evaluation_cells (id,evaluation_run_id,prompt_revision_id,profile_revision_id,test_case_id,status,cache_key,sort_order) VALUES (?1,?2,?3,?4,?5,'pending',?6,?7)",
                        params![
                            uuid::Uuid::new_v4().to_string(),
                            evaluation_run_id,
                            revision,
                            profile,
                            case.id,
                            key,
                            order,
                        ],
                    )
                    .map_err(|error| db_err("failed to create evaluation cell", error))?;
                    order += 1;
                }
            }
        }
        tx.commit()
            .map_err(|error| db_err("failed to commit evaluation matrix", error))?;
    }

    let cell_ids = {
        let conn = pool.get().map_err(|error| {
            AppError::io(format!("failed to acquire database connection: {error}"))
        })?;
        let mut stmt = conn
            .prepare(
                "SELECT id FROM evaluation_cells WHERE evaluation_run_id = ?1 ORDER BY sort_order",
            )
            .map_err(|error| db_err("failed to prepare matrix cells", error))?;
        let ids = stmt
            .query_map([&evaluation_run_id], |row| row.get::<_, String>(0))
            .map_err(|error| db_err("failed to query matrix cells", error))?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| db_err("failed to map matrix cells", error))?;
        ids
    };
    let mut completed = 0_i64;
    let mut failed = 0_i64;
    for cell_id in cell_ids {
        if cancel.is_cancelled() {
            let conn = pool.get().map_err(|error| {
                AppError::io(format!("failed to acquire database connection: {error}"))
            })?;
            conn.execute(
                "UPDATE evaluation_cells SET status='cancelled' WHERE evaluation_run_id=?1 AND status IN ('pending','running')",
                [&evaluation_run_id],
            )
            .map_err(|error| db_err("failed to cancel evaluation cells", error))?;
            conn.execute(
                "UPDATE evaluation_runs SET status='cancelled',completed_at=?1,completed_cells=?2,failed_cells=?3 WHERE id=?4",
                params![now_millis(), completed, failed, evaluation_run_id],
            )
            .map_err(|error| db_err("failed to cancel evaluation run", error))?;
            return get_evaluation_run(&conn, &evaluation_run_id);
        }
        let (revision_id, profile_id, case_id, key) = {
            let conn = pool.get().map_err(|error| {
                AppError::io(format!("failed to acquire database connection: {error}"))
            })?;
            conn.execute(
                "UPDATE evaluation_cells SET status='running' WHERE id=?1",
                [&cell_id],
            )
            .map_err(|error| db_err("failed to start evaluation cell", error))?;
            conn.query_row(
                "SELECT prompt_revision_id,profile_revision_id,test_case_id,cache_key FROM evaluation_cells WHERE id=?1",
                [&cell_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?)),
            )
            .map_err(|error| db_err("failed to read evaluation cell", error))?
        };
        let case = test_set
            .cases
            .iter()
            .find(|case| case.id == case_id)
            .ok_or_else(|| AppError::internal("evaluation case disappeared"))?;
        let cached = {
            let conn = pool.get().map_err(|error| {
                AppError::io(format!("failed to acquire database connection: {error}"))
            })?;
            conn.query_row(
                "SELECT prompt_run_id,results FROM evaluation_cells WHERE cache_key=?1 AND status='success' AND id<>?2 ORDER BY rowid DESC LIMIT 1",
                params![key, cell_id],
                |row| Ok((row.get::<_, Option<String>>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()
            .map_err(|error| db_err("failed to read evaluation cache", error))?
        };
        if let Some((prompt_run_id, results)) = cached {
            let cached_results: Vec<EvaluationResult> = serde_json::from_str(&results)
                .map_err(|error| AppError::internal(format!("invalid cached results: {error}")))?;
            if cached_results
                .iter()
                .any(|result| result.passed == Some(false))
            {
                failed += 1;
            }
            let conn = pool.get().map_err(|error| {
                AppError::io(format!("failed to acquire database connection: {error}"))
            })?;
            conn.execute(
                "UPDATE evaluation_cells SET prompt_run_id=?1,status='success',cache_hit=1,results=?2,error=NULL WHERE id=?3",
                params![prompt_run_id, results, cell_id],
            )
            .map_err(|error| db_err("failed to apply evaluation cache", error))?;
        } else {
            let run = execute_run(
                pool,
                encryption,
                adapter,
                PromptRunInput {
                    prompt_revision_id: revision_id,
                    profile_revision_id: profile_id,
                    inputs: case.inputs.clone(),
                    test_case_id: Some(case.id.clone()),
                },
                Some(key),
                cancel,
                sink,
            )
            .await;
            let (prompt_run_id, status, results, error) = match run {
                Ok(run) if run.status == "success" => {
                    let output = run.output.as_deref().unwrap_or_default();
                    let results = evaluators
                        .iter()
                        .map(|evaluator| {
                            evaluate_output(evaluator, output, case.expected_output.as_deref())
                                .or_else(|error| {
                                    Ok(EvaluationResult {
                                        evaluator_id: evaluator.id.clone(),
                                        kind: evaluator.kind.clone(),
                                        passed: Some(false),
                                        score: None,
                                        skipped: false,
                                        evidence: error.message,
                                    })
                                })
                        })
                        .collect::<Result<Vec<_>, AppError>>()?;
                    if results.iter().any(|result| result.passed == Some(false)) {
                        failed += 1;
                    }
                    (Some(run.id), "success".to_string(), results, None)
                }
                Ok(run) => {
                    failed += 1;
                    (Some(run.id), run.status, Vec::new(), run.error)
                }
                Err(error) => {
                    failed += 1;
                    (
                        None,
                        "error".to_string(),
                        Vec::new(),
                        Some(format!("[{}] {}", error.code_str(), error.message)),
                    )
                }
            };
            let conn = pool.get().map_err(|error| {
                AppError::io(format!("failed to acquire database connection: {error}"))
            })?;
            conn.execute(
                "UPDATE evaluation_cells SET prompt_run_id=?1,status=?2,results=?3,error=?4 WHERE id=?5",
                params![prompt_run_id, status, encode(&results)?, error, cell_id],
            )
            .map_err(|error| db_err("failed to complete evaluation cell", error))?;
        }
        completed += 1;
        let conn = pool.get().map_err(|error| {
            AppError::io(format!("failed to acquire database connection: {error}"))
        })?;
        conn.execute(
            "UPDATE evaluation_runs SET completed_cells=?1,failed_cells=?2 WHERE id=?3",
            params![completed, failed, evaluation_run_id],
        )
        .map_err(|error| db_err("failed to update evaluation progress", error))?;
        sink.emit_matrix_progress(&evaluation_run_id, completed, total, &cell_id);
    }
    let conn = pool
        .get()
        .map_err(|error| AppError::io(format!("failed to acquire database connection: {error}")))?;
    conn.execute(
        "UPDATE evaluation_runs SET status='success',completed_at=?1,completed_cells=?2,failed_cells=?3 WHERE id=?4",
        params![now_millis(), completed, failed, evaluation_run_id],
    )
    .map_err(|error| db_err("failed to complete evaluation run", error))?;
    get_evaluation_run(&conn, &evaluation_run_id)
}

pub async fn retry_evaluation_run(
    pool: &DbPool,
    encryption: &Mutex<EncryptionState>,
    adapter: &dyn ProviderAdapter,
    id: &str,
    cancel: &CancellationToken,
    sink: &dyn EvaluationEventSink,
) -> Result<EvaluationRunDetail, AppError> {
    let previous = {
        let conn = pool.get().map_err(|error| {
            AppError::io(format!("failed to acquire database connection: {error}"))
        })?;
        get_evaluation_run(&conn, id)?.run
    };
    run_matrix(
        pool,
        encryption,
        adapter,
        EvaluationMatrixInput {
            test_set_id: previous.test_set_id,
            prompt_revision_ids: previous.prompt_revision_ids,
            profile_revision_ids: previous.profile_revision_ids,
            evaluator_ids: previous.evaluator_ids,
        },
        cancel,
        sink,
    )
    .await
}

pub fn set_manual_result(
    conn: &Connection,
    cell_id: &str,
    evaluator_id: &str,
    passed: bool,
    evidence: &str,
) -> Result<EvaluationCell, AppError> {
    let mut cell = conn
        .query_row(
            "SELECT * FROM evaluation_cells WHERE id=?1",
            [cell_id],
            evaluation_cell_from_row,
        )
        .optional()
        .map_err(|error| db_err("failed to read evaluation cell", error))?
        .ok_or_else(|| AppError::not_found(format!("evaluation cell `{cell_id}` not found")))?;
    let evaluator = evaluator_by_id(conn, evaluator_id)?;
    if evaluator.kind != "manual" {
        return Err(AppError::validation("evaluator is not manual"));
    }
    let result = EvaluationResult {
        evaluator_id: evaluator_id.to_string(),
        kind: "manual".into(),
        passed: Some(passed),
        score: Some(if passed { 1.0 } else { 0.0 }),
        skipped: false,
        evidence: evidence.trim().to_string(),
    };
    if let Some(existing) = cell
        .results
        .iter_mut()
        .find(|item| item.evaluator_id == evaluator_id)
    {
        *existing = result;
    } else {
        cell.results.push(result);
    }
    conn.execute(
        "UPDATE evaluation_cells SET results=?1 WHERE id=?2",
        params![encode(&cell.results)?, cell_id],
    )
    .map_err(|error| db_err("failed to save manual result", error))?;
    Ok(cell)
}

fn label_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<PromptLabel> {
    Ok(PromptLabel {
        prompt_id: row.get("prompt_id")?,
        label: row.get("label")?,
        prompt_revision_id: row.get("prompt_revision_id")?,
        updated_at: millis_to_iso8601(row.get("updated_at")?),
    })
}

pub fn move_label(
    conn: &Connection,
    prompt_id: &str,
    label: &str,
    revision_id: &str,
    action: &str,
) -> Result<PromptLabel, AppError> {
    if !matches!(label, "baseline" | "candidate") {
        return Err(AppError::validation("label must be baseline or candidate"));
    }
    if !matches!(action, "move" | "rollback") {
        return Err(AppError::validation("invalid label action"));
    }
    let belongs: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM prompt_versions WHERE id=?1 AND prompt_id=?2)",
            params![revision_id, prompt_id],
            |row| row.get(0),
        )
        .map_err(|error| db_err("failed to validate label revision", error))?;
    if !belongs {
        return Err(AppError::not_found(
            "prompt revision does not belong to prompt",
        ));
    }
    let evaluated: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM evaluation_cells WHERE prompt_revision_id=?1 AND status='success')",
            [revision_id],
            |row| row.get(0),
        )
        .map_err(|error| db_err("failed to validate evaluated revision", error))?;
    if !evaluated {
        return Err(AppError::validation(
            "only a successfully evaluated revision can receive a label",
        ));
    }
    let previous: Option<String> = conn
        .query_row(
            "SELECT prompt_revision_id FROM prompt_labels WHERE prompt_id=?1 AND label=?2",
            params![prompt_id, label],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| db_err("failed to read current prompt label", error))?;
    let now = now_millis();
    let tx = conn
        .unchecked_transaction()
        .map_err(|error| db_err("failed to start label move", error))?;
    tx.execute(
        "INSERT INTO prompt_labels (prompt_id,label,prompt_revision_id,updated_at) VALUES (?1,?2,?3,?4) ON CONFLICT(prompt_id,label) DO UPDATE SET prompt_revision_id=excluded.prompt_revision_id,updated_at=excluded.updated_at",
        params![prompt_id, label, revision_id, now],
    )
    .map_err(|error| db_err("failed to move prompt label", error))?;
    tx.execute(
        "INSERT INTO prompt_label_history (id,prompt_id,label,from_revision_id,to_revision_id,action,created_at) VALUES (?1,?2,?3,?4,?5,?6,?7)",
        params![uuid::Uuid::new_v4().to_string(), prompt_id, label, previous, revision_id, action, now],
    )
    .map_err(|error| db_err("failed to record prompt label history", error))?;
    tx.commit()
        .map_err(|error| db_err("failed to commit prompt label move", error))?;
    conn.query_row(
        "SELECT * FROM prompt_labels WHERE prompt_id=?1 AND label=?2",
        params![prompt_id, label],
        label_from_row,
    )
    .map_err(|error| db_err("failed to read prompt label", error))
}

pub fn list_labels(conn: &Connection, prompt_id: &str) -> Result<Vec<PromptLabel>, AppError> {
    let mut stmt = conn
        .prepare("SELECT * FROM prompt_labels WHERE prompt_id=?1 ORDER BY label")
        .map_err(|error| db_err("failed to prepare prompt labels", error))?;
    let labels = stmt
        .query_map([prompt_id], label_from_row)
        .map_err(|error| db_err("failed to query prompt labels", error))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| db_err("failed to map prompt labels", error))?;
    Ok(labels)
}

pub fn label_history(
    conn: &Connection,
    prompt_id: &str,
) -> Result<Vec<PromptLabelHistory>, AppError> {
    let mut stmt = conn
        .prepare("SELECT * FROM prompt_label_history WHERE prompt_id=?1 ORDER BY created_at DESC,id DESC")
        .map_err(|error| db_err("failed to prepare prompt label history", error))?;
    let history = stmt
        .query_map([prompt_id], |row| {
            Ok(PromptLabelHistory {
                id: row.get("id")?,
                prompt_id: row.get("prompt_id")?,
                label: row.get("label")?,
                from_revision_id: row.get("from_revision_id")?,
                to_revision_id: row.get("to_revision_id")?,
                action: row.get("action")?,
                created_at: millis_to_iso8601(row.get("created_at")?),
            })
        })
        .map_err(|error| db_err("failed to query prompt label history", error))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| db_err("failed to map prompt label history", error))?;
    Ok(history)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::prompt::{self, PromptCreate, PromptUpdate};
    use crate::storage::{create_memory_pool, create_pool, init_schema};
    use std::sync::atomic::{AtomicBool, Ordering};

    struct Sink;
    impl EvaluationEventSink for Sink {
        fn emit_run_chunk(&self, _run_id: &str, _chunk: &str) {}
        fn emit_run_terminal(&self, _run_id: &str, _status: &str) {}
        fn emit_matrix_progress(&self, _id: &str, _completed: i64, _total: i64, _cell: &str) {}
    }

    #[derive(Default)]
    struct RecordingSink(Mutex<Vec<(i64, i64)>>);
    impl EvaluationEventSink for RecordingSink {
        fn emit_run_chunk(&self, _run_id: &str, _chunk: &str) {}
        fn emit_run_terminal(&self, _run_id: &str, _status: &str) {}
        fn emit_matrix_progress(&self, _id: &str, completed: i64, total: i64, _cell: &str) {
            self.0.lock().unwrap().push((completed, total));
        }
    }

    #[derive(Default)]
    struct ChunkRecordingSink(Mutex<String>);
    impl EvaluationEventSink for ChunkRecordingSink {
        fn emit_run_chunk(&self, _run_id: &str, chunk: &str) {
            self.0.lock().unwrap().push_str(chunk);
        }
        fn emit_run_terminal(&self, _run_id: &str, _status: &str) {}
        fn emit_matrix_progress(&self, _id: &str, _completed: i64, _total: i64, _cell: &str) {}
    }

    struct FlakyAdapter(AtomicBool);
    impl ProviderAdapter for FlakyAdapter {
        fn execute<'a>(
            &'a self,
            _request: ProviderRequest,
            _cancel: &'a CancellationToken,
            run_id: &'a str,
            sink: &'a dyn EvaluationEventSink,
        ) -> Pin<Box<dyn Future<Output = Result<ProviderOutput, AppError>> + Send + 'a>> {
            Box::pin(async move {
                if self.0.swap(false, Ordering::SeqCst) {
                    Err(AppError::network("transient mock failure"))
                } else {
                    sink.emit_run_chunk(run_id, "yes");
                    Ok(ProviderOutput {
                        content: "yes".into(),
                        usage: None,
                    })
                }
            })
        }
    }

    fn setup() -> (DbPool, Mutex<EncryptionState>, PromptVersion) {
        let pool = create_memory_pool().unwrap();
        let conn = pool.get().unwrap();
        init_schema(&conn).unwrap();
        let encryption = Mutex::new(EncryptionState::default());
        let prompt = prompt::create(
            &conn,
            PromptCreate {
                title: "Chat".into(),
                user_prompt: String::new(),
                messages: Some(vec![
                    PromptMessage {
                        role: "user".into(),
                        content: "Hello {{name}}".into(),
                    },
                    PromptMessage {
                        role: "assistant".into(),
                        content: "Hi".into(),
                    },
                ]),
                variables: Some(vec![Variable {
                    name: "name".into(),
                    r#type: "text".into(),
                    label: None,
                    default_value: None,
                    options: None,
                    required: true,
                }]),
                ..Default::default()
            },
        )
        .unwrap();
        let revision = crate::services::version::list(&conn, &prompt.id)
            .unwrap()
            .pop()
            .unwrap();
        drop(conn);
        (pool, encryption, revision)
    }

    #[test]
    fn renderer_preserves_message_order_and_requires_variables() {
        let (pool, encryption, revision) = setup();
        let conn = pool.get().unwrap();
        let error = render_prompt(&conn, &encryption, &revision.id, &BTreeMap::new()).unwrap_err();
        assert_eq!(error.code_str(), "VALIDATION");
        let rendered = render_prompt(
            &conn,
            &encryption,
            &revision.id,
            &BTreeMap::from([("name".into(), "Ada".into())]),
        )
        .unwrap();
        assert_eq!(rendered.messages[0].content, "Hello Ada");
        assert_eq!(rendered.messages[1].role, "assistant");
    }

    #[test]
    fn deterministic_evaluators_keep_evidence_and_skips() {
        let exact = EvaluatorConfig {
            id: "e1".into(),
            name: "Exact".into(),
            kind: "exact".into(),
            config: json!({}),
            created_at: String::new(),
        };
        let result = evaluate_output(&exact, "yes", Some("yes")).unwrap();
        assert_eq!(result.passed, Some(true));
        assert!(!result.evidence.is_empty());
        let manual = EvaluatorConfig {
            kind: "manual".into(),
            ..exact
        };
        assert!(evaluate_output(&manual, "anything", None).unwrap().skipped);
    }

    #[test]
    fn openai_stream_lines_emit_chunks_and_usage_incrementally() {
        let sink = ChunkRecordingSink::default();
        let mut content = String::new();
        let mut usage = None;

        process_openai_stream_line(
            r#"data: {"choices":[{"delta":{"content":"hel"}}]}"#,
            &mut content,
            &mut usage,
            "run-1",
            &sink,
        )
        .unwrap();
        process_openai_stream_line(
            r#"data: {"choices":[{"delta":{"content":"lo"}}],"usage":{"total_tokens":2}}"#,
            &mut content,
            &mut usage,
            "run-1",
            &sink,
        )
        .unwrap();

        assert_eq!(content, "hello");
        assert_eq!(sink.0.lock().unwrap().as_str(), "hello");
        assert_eq!(usage, Some(json!({ "total_tokens": 2 })));
    }

    #[test]
    fn profile_credentials_are_encrypted_redacted_and_rekeyed() {
        let pool = create_memory_pool().unwrap();
        let conn = pool.get().unwrap();
        init_schema(&conn).unwrap();
        let encryption = Mutex::new(EncryptionState::default());
        crate::services::security::set_master_password(&conn, &encryption, "old-password").unwrap();
        let profile = create_profile(
            &conn,
            &encryption,
            ExecutionProfileInput {
                profile_id: None,
                name: "Remote".into(),
                provider: "openai-compatible".into(),
                endpoint: Some("https://example.com/v1/chat/completions".into()),
                model: "model".into(),
                parameters: json!({}),
                credential: Some("secret-token".into()),
            },
        )
        .unwrap();
        assert!(profile.has_credential);
        assert!(!serde_json::to_string(&profile)
            .unwrap()
            .contains("secret-token"));
        let stored: String = conn
            .query_row(
                "SELECT credential FROM execution_profile_revisions WHERE id=?1",
                [&profile.id],
                |row| row.get(0),
            )
            .unwrap();
        assert!(stored.starts_with("ENC::"));
        assert!(!stored.contains("secret-token"));

        crate::services::security::change_master_password(
            &conn,
            &encryption,
            "old-password",
            "new-password",
        )
        .unwrap();
        let request = provider_request(&conn, &encryption, &profile.id, vec![]).unwrap();
        assert_eq!(request.credential.as_deref(), Some("secret-token"));
    }

    #[tokio::test]
    async fn matrix_rerun_uses_completed_cell_cache() {
        let (pool, encryption, revision) = setup();
        let (profile, test_set, evaluator) = {
            let conn = pool.get().unwrap();
            let profile = create_profile(
                &conn,
                &encryption,
                ExecutionProfileInput {
                    profile_id: None,
                    name: "Mock".into(),
                    provider: "mock".into(),
                    endpoint: None,
                    model: "deterministic".into(),
                    parameters: json!({ "response": "yes" }),
                    credential: None,
                },
            )
            .unwrap();
            let test_set = save_test_set(
                &conn,
                TestSetInput {
                    id: None,
                    name: "Cases".into(),
                    cases: vec![TestCaseInput {
                        id: None,
                        name: "One".into(),
                        inputs: BTreeMap::from([("name".into(), "Ada".into())]),
                        expected_output: Some("no".into()),
                        annotations: json!({}),
                    }],
                },
            )
            .unwrap();
            let evaluator = create_evaluator(
                &conn,
                EvaluatorInput {
                    name: "Exact".into(),
                    kind: "exact".into(),
                    config: json!({}),
                },
            )
            .unwrap();
            (profile, test_set, evaluator)
        };
        let input = EvaluationMatrixInput {
            test_set_id: test_set.id,
            prompt_revision_ids: vec![revision.id],
            profile_revision_ids: vec![profile.id],
            evaluator_ids: vec![evaluator.id],
        };
        let first = run_matrix(
            &pool,
            &encryption,
            &DefaultProviderAdapter,
            input.clone(),
            &CancellationToken::new(),
            &Sink,
        )
        .await
        .unwrap();
        assert!(!first.cells[0].cache_hit);
        assert_eq!(first.run.failed_cells, 1);
        let second = run_matrix(
            &pool,
            &encryption,
            &DefaultProviderAdapter,
            input,
            &CancellationToken::new(),
            &Sink,
        )
        .await
        .unwrap();
        assert!(second.cells[0].cache_hit);
        assert_eq!(second.cells[0].results[0].passed, Some(false));
        assert_eq!(second.run.failed_cells, 1);
    }

    #[tokio::test]
    async fn matrix_records_render_validation_as_a_terminal_cell_error() {
        let (pool, encryption, revision) = setup();
        let (profile, test_set, evaluator) = {
            let conn = pool.get().unwrap();
            let profile = create_profile(
                &conn,
                &encryption,
                ExecutionProfileInput {
                    profile_id: None,
                    name: "Mock".into(),
                    provider: "mock".into(),
                    endpoint: None,
                    model: "deterministic".into(),
                    parameters: json!({ "response": "yes" }),
                    credential: None,
                },
            )
            .unwrap();
            let test_set = save_test_set(
                &conn,
                TestSetInput {
                    id: None,
                    name: "Missing input".into(),
                    cases: vec![TestCaseInput {
                        id: None,
                        name: "No name".into(),
                        inputs: BTreeMap::new(),
                        expected_output: Some("yes".into()),
                        annotations: json!({}),
                    }],
                },
            )
            .unwrap();
            let evaluator = create_evaluator(
                &conn,
                EvaluatorInput {
                    name: "Exact".into(),
                    kind: "exact".into(),
                    config: json!({}),
                },
            )
            .unwrap();
            (profile, test_set, evaluator)
        };

        let detail = run_matrix(
            &pool,
            &encryption,
            &DefaultProviderAdapter,
            EvaluationMatrixInput {
                test_set_id: test_set.id,
                prompt_revision_ids: vec![revision.id],
                profile_revision_ids: vec![profile.id],
                evaluator_ids: vec![evaluator.id],
            },
            &CancellationToken::new(),
            &Sink,
        )
        .await
        .unwrap();

        assert_eq!(detail.run.status, "success");
        assert_eq!(detail.run.completed_cells, 1);
        assert_eq!(detail.run.failed_cells, 1);
        assert_eq!(detail.cells[0].status, "error");
        assert!(detail.cells[0].prompt_run_id.is_none());
        assert!(detail.cells[0]
            .error
            .as_deref()
            .is_some_and(|error| error.contains("[VALIDATION]")));
    }

    #[tokio::test]
    async fn matrix_orders_two_revisions_two_profiles_across_twenty_cases() {
        let (pool, encryption, first_revision) = setup();
        let (second_revision, profiles, test_set, evaluator) = {
            let conn = pool.get().unwrap();
            let second_revision = crate::services::version::create(
                &conn,
                &first_revision.prompt_id,
                Some("matrix candidate".into()),
            )
            .unwrap();
            let profiles = ["A", "B"]
                .into_iter()
                .map(|name| {
                    create_profile(
                        &conn,
                        &encryption,
                        ExecutionProfileInput {
                            profile_id: None,
                            name: format!("Mock {name}"),
                            provider: "mock".into(),
                            endpoint: None,
                            model: format!("model-{name}"),
                            parameters: json!({ "response": "yes" }),
                            credential: None,
                        },
                    )
                    .unwrap()
                })
                .collect::<Vec<_>>();
            let cases = (0..20)
                .map(|index| TestCaseInput {
                    id: None,
                    name: format!("Case {index:02}"),
                    inputs: BTreeMap::from([("name".into(), format!("User {index}"))]),
                    expected_output: Some("yes".into()),
                    annotations: json!({ "index": index }),
                })
                .collect();
            let test_set = save_test_set(
                &conn,
                TestSetInput {
                    id: None,
                    name: "Twenty cases".into(),
                    cases,
                },
            )
            .unwrap();
            let evaluator = create_evaluator(
                &conn,
                EvaluatorInput {
                    name: "Exact".into(),
                    kind: "exact".into(),
                    config: json!({}),
                },
            )
            .unwrap();
            (second_revision, profiles, test_set, evaluator)
        };
        let sink = RecordingSink::default();
        let detail = run_matrix(
            &pool,
            &encryption,
            &DefaultProviderAdapter,
            EvaluationMatrixInput {
                test_set_id: test_set.id,
                prompt_revision_ids: vec![first_revision.id, second_revision.id],
                profile_revision_ids: profiles.into_iter().map(|profile| profile.id).collect(),
                evaluator_ids: vec![evaluator.id],
            },
            &CancellationToken::new(),
            &sink,
        )
        .await
        .unwrap();
        assert_eq!(detail.run.total_cells, 80);
        assert_eq!(detail.run.completed_cells, 80);
        assert_eq!(detail.run.failed_cells, 0);
        assert_eq!(detail.cells.len(), 80);
        assert!(detail
            .cells
            .iter()
            .enumerate()
            .all(|(index, cell)| cell.sort_order == index as i64));
        assert_eq!(sink.0.lock().unwrap().last(), Some(&(80, 80)));
    }

    #[tokio::test]
    async fn retry_reruns_provider_error_cells() {
        let (pool, encryption, revision) = setup();
        let (profile, test_set, evaluator) = {
            let conn = pool.get().unwrap();
            let profile = create_profile(
                &conn,
                &encryption,
                ExecutionProfileInput {
                    profile_id: None,
                    name: "Flaky".into(),
                    provider: "mock".into(),
                    endpoint: None,
                    model: "flaky".into(),
                    parameters: json!({}),
                    credential: None,
                },
            )
            .unwrap();
            let test_set = save_test_set(
                &conn,
                TestSetInput {
                    id: None,
                    name: "Retry".into(),
                    cases: vec![TestCaseInput {
                        id: None,
                        name: "One".into(),
                        inputs: BTreeMap::from([("name".into(), "Ada".into())]),
                        expected_output: Some("yes".into()),
                        annotations: json!({}),
                    }],
                },
            )
            .unwrap();
            let evaluator = create_evaluator(
                &conn,
                EvaluatorInput {
                    name: "Exact".into(),
                    kind: "exact".into(),
                    config: json!({}),
                },
            )
            .unwrap();
            (profile, test_set, evaluator)
        };
        let adapter = FlakyAdapter(AtomicBool::new(true));
        let first = run_matrix(
            &pool,
            &encryption,
            &adapter,
            EvaluationMatrixInput {
                test_set_id: test_set.id,
                prompt_revision_ids: vec![revision.id],
                profile_revision_ids: vec![profile.id],
                evaluator_ids: vec![evaluator.id],
            },
            &CancellationToken::new(),
            &Sink,
        )
        .await
        .unwrap();
        assert_eq!(first.cells[0].status, "error");
        let retried = retry_evaluation_run(
            &pool,
            &encryption,
            &adapter,
            &first.run.id,
            &CancellationToken::new(),
            &Sink,
        )
        .await
        .unwrap();
        assert_eq!(retried.cells[0].status, "success");
        assert!(!retried.cells[0].cache_hit);
    }

    #[tokio::test]
    async fn native_mock_workflow_persists_across_file_database_restart() {
        let temp = tempfile::tempdir().unwrap();
        let database = temp.path().join("prompthub.db");
        let pool = create_pool(&database).unwrap();
        let encryption = Mutex::new(EncryptionState::default());
        let (prompt, revisions, profile, test_set, evaluator) = {
            let conn = pool.get().unwrap();
            init_schema(&conn).unwrap();
            let prompt = prompt::create(
                &conn,
                PromptCreate {
                    title: "Native evaluation smoke".into(),
                    user_prompt: String::new(),
                    messages: Some(vec![
                        PromptMessage {
                            role: "user".into(),
                            content: "Hello {{name}}".into(),
                        },
                        PromptMessage {
                            role: "assistant".into(),
                            content: "Example reply".into(),
                        },
                    ]),
                    variables: Some(vec![Variable {
                        name: "name".into(),
                        r#type: "text".into(),
                        label: None,
                        default_value: None,
                        options: None,
                        required: true,
                    }]),
                    ..Default::default()
                },
            )
            .unwrap();
            prompt::update(
                &conn,
                &prompt.id,
                PromptUpdate {
                    messages: Some(vec![
                        PromptMessage {
                            role: "system".into(),
                            content: "Answer briefly.".into(),
                        },
                        PromptMessage {
                            role: "user".into(),
                            content: "Hello {{name}}".into(),
                        },
                        PromptMessage {
                            role: "assistant".into(),
                            content: "Example reply".into(),
                        },
                    ]),
                    ..Default::default()
                },
            )
            .unwrap();
            let mut revisions = crate::services::version::list(&conn, &prompt.id).unwrap();
            revisions.sort_by_key(|revision| revision.version);
            let profile = create_profile(
                &conn,
                &encryption,
                ExecutionProfileInput {
                    profile_id: None,
                    name: "Native mock".into(),
                    provider: "mock".into(),
                    endpoint: None,
                    model: "deterministic".into(),
                    parameters: json!({ "response": "mock-output" }),
                    credential: None,
                },
            )
            .unwrap();
            let test_set = save_test_set(
                &conn,
                TestSetInput {
                    id: None,
                    name: "Native cases".into(),
                    cases: vec![TestCaseInput {
                        id: None,
                        name: "Ada".into(),
                        inputs: BTreeMap::from([("name".into(), "Ada".into())]),
                        expected_output: Some("mock-output".into()),
                        annotations: json!({ "source": "native-smoke" }),
                    }],
                },
            )
            .unwrap();
            let evaluator = create_evaluator(
                &conn,
                EvaluatorInput {
                    name: "Exact".into(),
                    kind: "exact".into(),
                    config: json!({}),
                },
            )
            .unwrap();
            (prompt, revisions, profile, test_set, evaluator)
        };

        assert_eq!(revisions.len(), 2);
        assert_eq!(revisions[1].messages[0].role, "system");
        let rendered = {
            let conn = pool.get().unwrap();
            render_prompt(
                &conn,
                &encryption,
                &revisions[1].id,
                &BTreeMap::from([("name".into(), "Ada".into())]),
            )
            .unwrap()
        };
        assert_eq!(rendered.messages[1].content, "Hello Ada");

        let playground = execute_run(
            &pool,
            &encryption,
            &DefaultProviderAdapter,
            PromptRunInput {
                prompt_revision_id: revisions[1].id.clone(),
                profile_revision_id: profile.id.clone(),
                inputs: BTreeMap::from([("name".into(), "Ada".into())]),
                test_case_id: None,
            },
            None,
            &CancellationToken::new(),
            &Sink,
        )
        .await
        .unwrap();
        assert_eq!(playground.output.as_deref(), Some("mock-output"));

        let matrix = run_matrix(
            &pool,
            &encryption,
            &DefaultProviderAdapter,
            EvaluationMatrixInput {
                test_set_id: test_set.id,
                prompt_revision_ids: revisions
                    .iter()
                    .map(|revision| revision.id.clone())
                    .collect(),
                profile_revision_ids: vec![profile.id],
                evaluator_ids: vec![evaluator.id],
            },
            &CancellationToken::new(),
            &Sink,
        )
        .await
        .unwrap();
        assert_eq!(matrix.run.completed_cells, 2);
        assert!(matrix.cells.iter().all(|cell| cell.status == "success"));

        drop(pool);
        let restarted = create_pool(&database).unwrap();
        let conn = restarted.get().unwrap();
        init_schema(&conn).unwrap();
        assert_eq!(
            crate::services::version::list(&conn, &prompt.id)
                .unwrap()
                .len(),
            2
        );
        assert_eq!(list_runs(&conn).unwrap().len(), 3);
        assert_eq!(list_evaluation_runs(&conn).unwrap().len(), 1);
        assert_eq!(get_run(&conn, &playground.id).unwrap().status, "success");
    }

    #[tokio::test]
    #[ignore = "requires explicit PROMPTHUB_LIVE_OPENAI_* environment variables"]
    async fn live_openai_compatible_smoke() {
        let endpoint = std::env::var("PROMPTHUB_LIVE_OPENAI_ENDPOINT")
            .expect("PROMPTHUB_LIVE_OPENAI_ENDPOINT is required");
        let model = std::env::var("PROMPTHUB_LIVE_OPENAI_MODEL")
            .expect("PROMPTHUB_LIVE_OPENAI_MODEL is required");
        let credential = std::env::var("PROMPTHUB_LIVE_OPENAI_API_KEY").ok();
        let output = execute_openai(
            ProviderRequest {
                provider: "openai-compatible".into(),
                endpoint: Some(endpoint),
                model,
                parameters: json!({}),
                credential,
                messages: vec![PromptMessage {
                    role: "user".into(),
                    content: "Reply with OK.".into(),
                }],
            },
            &CancellationToken::new(),
            "live-smoke",
            &Sink,
        )
        .await
        .unwrap();
        assert!(!output.content.trim().is_empty());
    }
}
