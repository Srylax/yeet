use std::sync::Arc;

use axum::{Json, extract::State};
use parking_lot::RwLock;

use crate::{AppState, httpsig::HttpSig, state::StateError};

// Not able to return hosts as a struct because of the way HashMap is structured
pub async fn status(
    State(state): State<Arc<RwLock<AppState>>>,
    HttpSig(key): HttpSig,
) -> Result<Json<Vec<api::Host>>, StateError> {
    state.read_arc().auth_admin(&key)?;
    Ok(Json(state.read_arc().hosts().cloned().collect()))
}
