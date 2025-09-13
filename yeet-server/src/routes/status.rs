use std::sync::Arc;

use axum::{extract::State, http::StatusCode};
use parking_lot::RwLock;
use serde_json_any_key::MapIterToJson;

use crate::{AppState, error::WithStatusCode};

// Not able to return hosts as a struct because of the way HashMap is structured
pub async fn status(
    State(state): State<Arc<RwLock<AppState>>>,
) -> Result<String, (StatusCode, String)> {
    state
        .read_arc()
        .hosts
        .clone()
        .to_json_map()
        .with_code(StatusCode::INTERNAL_SERVER_ERROR)
}
