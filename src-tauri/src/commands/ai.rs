use crate::error::{AppError, CommandResult};
use crate::services::ai::{self, AiRequest, AiResponse};
use crate::state::AppState;

use super::ensure_ready;

use tauri::Manager;

#[tauri::command(rename = "ai.request")]
pub async fn ai_request<R: tauri::Runtime>(
    request: AiRequest,
    app: tauri::AppHandle<R>,
) -> CommandResult<AiResponse> {
    let ready = {
        let state = app.state::<AppState>();
        ensure_ready(&state)
    };

    match ready {
        Ok(()) => ai::request(&request).await.into(),
        Err(e) => CommandResult::Err(e),
    }
}

#[tauri::command(rename = "ai.stream")]
pub async fn ai_stream<R: tauri::Runtime>(
    request: AiRequest,
    app: tauri::AppHandle<R>,
) -> CommandResult<()> {
    let request_id = request.request_id.clone();
    let token = {
        let state = app.state::<AppState>();
        if let Err(e) = ensure_ready(&state) {
            return CommandResult::Err(e);
        }
        state.register_request(&request_id)
    };
    let sink = super::events::TauriEventSink::new(app.clone());
    ai::stream(&request, &token, &sink).await;
    app.state::<AppState>().finish_request(&request_id);
    CommandResult::Ok(())
}

#[tauri::command(rename = "ai.cancel")]
pub fn ai_cancel(request_id: String, state: tauri::State<'_, AppState>) -> CommandResult<()> {
    match ensure_ready(&state) {
        Ok(()) => {
            if state.cancel_request(&request_id) {
                CommandResult::Ok(())
            } else {
                CommandResult::Err(AppError::not_found(format!(
                    "AI request `{request_id}` is not active"
                )))
            }
        }
        Err(e) => CommandResult::Err(e),
    }
}
