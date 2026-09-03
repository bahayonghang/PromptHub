use crate::error::{AppError, CommandResult};
use crate::models::*;
use crate::services::evaluation::{self, DefaultProviderAdapter};
use crate::state::AppState;

use super::{conn, ensure_ready, events::TauriEvaluationEventSink, into_command};
use tauri::Manager;

#[tauri::command(rename = "evaluation.profileList")]
pub fn profile_list(
    state: tauri::State<'_, AppState>,
) -> CommandResult<Vec<ExecutionProfileRevision>> {
    into_command(conn(&state).and_then(|conn| evaluation::list_profiles(&conn)))
}

#[tauri::command(rename = "evaluation.profileSave")]
pub fn profile_save(
    input: ExecutionProfileInput,
    state: tauri::State<'_, AppState>,
) -> CommandResult<ExecutionProfileRevision> {
    into_command(
        conn(&state).and_then(|conn| evaluation::create_profile(&conn, &state.encryption, input)),
    )
}

#[tauri::command(rename = "evaluation.render")]
pub fn render(
    prompt_revision_id: String,
    inputs: std::collections::BTreeMap<String, String>,
    state: tauri::State<'_, AppState>,
) -> CommandResult<RenderedPrompt> {
    into_command(conn(&state).and_then(|conn| {
        evaluation::render_prompt(&conn, &state.encryption, &prompt_revision_id, &inputs)
    }))
}

#[tauri::command(rename = "evaluation.run")]
pub async fn run<R: tauri::Runtime>(
    request_id: String,
    input: PromptRunInput,
    app: tauri::AppHandle<R>,
) -> CommandResult<PromptRun> {
    let state = app.state::<AppState>();
    if let Err(error) = ensure_ready(&state) {
        return CommandResult::Err(error);
    }
    if request_id.trim().is_empty() {
        return CommandResult::Err(AppError::validation("requestId is required"));
    }
    let pool = match state.pool.lock() {
        Ok(pool) => match pool.clone() {
            Some(pool) => pool,
            None => {
                return CommandResult::Err(AppError::internal("database pool is not initialized"))
            }
        },
        Err(_) => return CommandResult::Err(AppError::internal("database pool lock is poisoned")),
    };
    let token = state.register_request(&request_id);
    let sink = TauriEvaluationEventSink::new(app.clone(), request_id.clone());
    let result = evaluation::execute_run(
        &pool,
        &state.encryption,
        &DefaultProviderAdapter,
        input,
        None,
        &token,
        &sink,
    )
    .await;
    state.finish_request(&request_id);
    result.into()
}

#[tauri::command(rename = "evaluation.runList")]
pub fn run_list(state: tauri::State<'_, AppState>) -> CommandResult<Vec<PromptRun>> {
    into_command(conn(&state).and_then(|conn| evaluation::list_runs(&conn, &state.encryption)))
}

#[tauri::command(rename = "evaluation.runGet")]
pub fn run_get(id: String, state: tauri::State<'_, AppState>) -> CommandResult<PromptRun> {
    into_command(conn(&state).and_then(|conn| evaluation::get_run(&conn, &state.encryption, &id)))
}

#[tauri::command(rename = "evaluation.cancel")]
pub fn cancel(request_id: String, state: tauri::State<'_, AppState>) -> CommandResult<()> {
    match ensure_ready(&state) {
        Ok(()) if state.cancel_request(&request_id) => CommandResult::Ok(()),
        Ok(()) => CommandResult::Err(AppError::not_found(format!(
            "evaluation request `{request_id}` is not active"
        ))),
        Err(error) => CommandResult::Err(error),
    }
}

#[tauri::command(rename = "evaluation.testSetList")]
pub fn test_set_list(state: tauri::State<'_, AppState>) -> CommandResult<Vec<TestSet>> {
    into_command(conn(&state).and_then(|conn| evaluation::list_test_sets(&conn)))
}

#[tauri::command(rename = "evaluation.testSetSave")]
pub fn test_set_save(
    input: TestSetInput,
    state: tauri::State<'_, AppState>,
) -> CommandResult<TestSet> {
    into_command(conn(&state).and_then(|conn| evaluation::save_test_set(&conn, input)))
}

#[tauri::command(rename = "evaluation.testSetExport")]
pub fn test_set_export(id: String, state: tauri::State<'_, AppState>) -> CommandResult<String> {
    into_command(conn(&state).and_then(|conn| evaluation::export_test_set(&conn, &id)))
}

#[tauri::command(rename = "evaluation.testSetImport")]
pub fn test_set_import(json: String, state: tauri::State<'_, AppState>) -> CommandResult<TestSet> {
    into_command(conn(&state).and_then(|conn| evaluation::import_test_set(&conn, &json)))
}

#[tauri::command(rename = "evaluation.evaluatorList")]
pub fn evaluator_list(state: tauri::State<'_, AppState>) -> CommandResult<Vec<EvaluatorConfig>> {
    into_command(conn(&state).and_then(|conn| evaluation::list_evaluators(&conn)))
}

#[tauri::command(rename = "evaluation.evaluatorCreate")]
pub fn evaluator_create(
    input: EvaluatorInput,
    state: tauri::State<'_, AppState>,
) -> CommandResult<EvaluatorConfig> {
    into_command(conn(&state).and_then(|conn| evaluation::create_evaluator(&conn, input)))
}

fn async_context(state: &AppState) -> Result<crate::storage::DbPool, AppError> {
    ensure_ready(state)?;
    state
        .pool
        .lock()
        .map_err(|_| AppError::internal("database pool lock is poisoned"))?
        .clone()
        .ok_or_else(|| AppError::internal("database pool is not initialized"))
}

#[tauri::command(rename = "evaluation.matrixRun")]
pub async fn matrix_run<R: tauri::Runtime>(
    request_id: String,
    input: EvaluationMatrixInput,
    app: tauri::AppHandle<R>,
) -> CommandResult<EvaluationRunDetail> {
    let state = app.state::<AppState>();
    let pool = match async_context(&state) {
        Ok(pool) => pool,
        Err(error) => return CommandResult::Err(error),
    };
    if request_id.trim().is_empty() {
        return CommandResult::Err(AppError::validation("requestId is required"));
    }
    let token = state.register_request(&request_id);
    let sink = TauriEvaluationEventSink::new(app.clone(), request_id.clone());
    let result = evaluation::run_matrix(
        &pool,
        &state.encryption,
        &DefaultProviderAdapter,
        input,
        &token,
        &sink,
    )
    .await;
    state.finish_request(&request_id);
    result.into()
}

#[tauri::command(rename = "evaluation.matrixRetry")]
pub async fn matrix_retry<R: tauri::Runtime>(
    request_id: String,
    id: String,
    app: tauri::AppHandle<R>,
) -> CommandResult<EvaluationRunDetail> {
    let state = app.state::<AppState>();
    let pool = match async_context(&state) {
        Ok(pool) => pool,
        Err(error) => return CommandResult::Err(error),
    };
    if request_id.trim().is_empty() {
        return CommandResult::Err(AppError::validation("requestId is required"));
    }
    let token = state.register_request(&request_id);
    let sink = TauriEvaluationEventSink::new(app.clone(), request_id.clone());
    let result = evaluation::retry_evaluation_run(
        &pool,
        &state.encryption,
        &DefaultProviderAdapter,
        &id,
        &token,
        &sink,
    )
    .await;
    state.finish_request(&request_id);
    result.into()
}

#[tauri::command(rename = "evaluation.matrixList")]
pub fn matrix_list(state: tauri::State<'_, AppState>) -> CommandResult<Vec<EvaluationRun>> {
    into_command(conn(&state).and_then(|conn| evaluation::list_evaluation_runs(&conn)))
}

#[tauri::command(rename = "evaluation.matrixGet")]
pub fn matrix_get(
    id: String,
    state: tauri::State<'_, AppState>,
) -> CommandResult<EvaluationRunDetail> {
    into_command(conn(&state).and_then(|conn| evaluation::get_evaluation_run(&conn, &id)))
}

#[tauri::command(rename = "evaluation.manualResult")]
pub fn manual_result(
    cell_id: String,
    evaluator_id: String,
    passed: bool,
    evidence: String,
    state: tauri::State<'_, AppState>,
) -> CommandResult<EvaluationCell> {
    into_command(conn(&state).and_then(|conn| {
        evaluation::set_manual_result(&conn, &cell_id, &evaluator_id, passed, &evidence)
    }))
}

#[tauri::command(rename = "evaluation.labelList")]
pub fn label_list(
    prompt_id: String,
    state: tauri::State<'_, AppState>,
) -> CommandResult<Vec<PromptLabel>> {
    into_command(conn(&state).and_then(|conn| evaluation::list_labels(&conn, &prompt_id)))
}

#[tauri::command(rename = "evaluation.labelMove")]
pub fn label_move(
    prompt_id: String,
    label: String,
    prompt_revision_id: String,
    state: tauri::State<'_, AppState>,
) -> CommandResult<PromptLabel> {
    into_command(conn(&state).and_then(|conn| {
        evaluation::move_label(&conn, &prompt_id, &label, &prompt_revision_id, "move")
    }))
}

#[tauri::command(rename = "evaluation.labelRollback")]
pub fn label_rollback(
    prompt_id: String,
    label: String,
    prompt_revision_id: String,
    state: tauri::State<'_, AppState>,
) -> CommandResult<PromptLabel> {
    into_command(conn(&state).and_then(|conn| {
        evaluation::move_label(&conn, &prompt_id, &label, &prompt_revision_id, "rollback")
    }))
}

#[tauri::command(rename = "evaluation.labelHistory")]
pub fn label_history(
    prompt_id: String,
    state: tauri::State<'_, AppState>,
) -> CommandResult<Vec<PromptLabelHistory>> {
    into_command(conn(&state).and_then(|conn| evaluation::label_history(&conn, &prompt_id)))
}
