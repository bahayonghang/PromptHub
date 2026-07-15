//! Tauri_Backend services: the components that own the application's business
//! rules (design: "Layering and Responsibilities").
//!
//! Each service is written against I/O dependencies passed in (a borrowed
//! `rusqlite::Connection`/pool from [`crate::state::AppState`]) rather than
//! reaching into global state, so the rules are unit- and property-testable
//! without a live window. The Command_Layer (task 17.1) is the thin adapter that
//! hands services a pooled connection and maps their `Result<T, AppError>` into
//! the `CommandResult` envelope.
//!
//! Wave-5 services (created concurrently): the Prompt_Service (Req 6), the
//! Folder_Service (Req 8), and the Security_Service (Req 15).

pub mod ai;
pub mod data_path;
pub mod evaluation;
pub mod folder;
pub mod media;
pub mod network_safety;
pub mod portable;
pub mod prompt;
pub mod prompt_type;
pub mod rules;
pub mod security;
pub mod settings;
pub mod sync;
pub mod updater;
pub mod version;
pub mod window;
