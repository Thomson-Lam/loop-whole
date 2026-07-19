use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::get,
};
use serde::Serialize;

use crate::{
    schema::{SessionSnapshot, ToolCallDetail},
    store::SessionStore,
};

#[derive(Debug, Clone)]
pub struct ApiState {
    pub store: SessionStore,
}

pub fn router(state: Arc<ApiState>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/v1/sessions/current", get(current_session))
        .route("/api/v1/tool-calls/{id}", get(tool_call))
        .with_state(state)
}

#[derive(Serialize)]
struct Health {
    status: &'static str,
}

async fn health() -> Json<Health> {
    Json(Health { status: "ok" })
}

async fn current_session(State(state): State<Arc<ApiState>>) -> Json<SessionSnapshot> {
    Json(state.store.snapshot())
}

async fn tool_call(
    State(state): State<Arc<ApiState>>,
    Path(id): Path<i64>,
) -> Result<Json<ToolCallDetail>, (StatusCode, String)> {
    state
        .store
        .tool_call(id)
        .map(Json)
        .ok_or_else(|| (StatusCode::NOT_FOUND, "tool call not found".to_string()))
}
