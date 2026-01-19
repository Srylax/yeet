use std::{collections::HashMap, sync::Arc};

use axum::{
    Json,
    extract::{Path, State},
};

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

pub async fn status_by_name(
    State(state): State<Arc<RwLock<AppState>>>,
    Path(hostname): Path<String>,
    HttpSig(key): HttpSig,
) -> Result<Json<Option<api::Host>>, StateError> {
    let state = state.read_arc();
    state.auth_admin(&key)?;
    Ok(Json(state.host_by_name(hostname)))
}

/// Get all registered hosts and their state.
pub async fn get_registered_hosts(
    State(state): State<Arc<RwLock<AppState>>>,
    HttpSig(key): HttpSig,
) -> Result<Json<HashMap<String, api::ProvisionState>>, StateError> {
    let state = state.read_arc();
    state.auth_admin(&key)?;
    Ok(Json(state.registered_hosts()))
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
