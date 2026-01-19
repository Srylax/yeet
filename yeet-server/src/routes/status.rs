use std::{collections::HashMap, sync::Arc};

use axum::{Json, extract::State};

use ed25519_dalek::VerifyingKey;
use parking_lot::RwLock;

use crate::{AppState, httpsig::HttpSig, state::StateError};

// Not able to return hosts as a struct because of the way HashMap is structured
pub async fn status(
    State(state): State<Arc<RwLock<AppState>>>,
    HttpSig(key): HttpSig,
) -> Result<Json<Vec<api::Host>>, StateError> {
    let state = state.read_arc();
    state.auth_admin(&key)?;
    Ok(Json(state.hosts().cloned().collect()))
}

/// hostname -> pub key
pub async fn hosts_by_key(
    State(state): State<Arc<RwLock<AppState>>>,
    HttpSig(key): HttpSig,
) -> Result<Json<HashMap<String, VerifyingKey>>, StateError> {
    let state = state.read_arc();
    state.auth_admin(&key)?;
    Ok(Json(state.hosts_by_key()))
}
