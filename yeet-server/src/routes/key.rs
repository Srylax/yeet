use std::sync::Arc;

use crate::AppState;
use crate::httpsig::{HttpSig, VerifiedJson};
use crate::state::StateError;
use axum::extract::State;
use axum::http::StatusCode;
use ed25519_dalek::VerifyingKey;
use parking_lot::RwLock;

pub async fn add_key(
    State(state): State<Arc<RwLock<AppState>>>,
    HttpSig(http_key): HttpSig,

    VerifiedJson(api::AddKey { key, level }): VerifiedJson<api::AddKey>,
) -> Result<StatusCode, StateError> {
    let mut state = state.write_arc();

    state.auth_admin(&http_key)?;

    state.add_key(key, level);

    Ok(StatusCode::CREATED)
}

pub async fn remove_key(
    State(state): State<Arc<RwLock<AppState>>>,
    HttpSig(http_key): HttpSig,

    VerifiedJson(key): VerifiedJson<VerifyingKey>,
) -> Result<StatusCode, StateError> {
    let mut state = state.write_arc();

    state.auth_admin(&http_key)?;

    state.remove_key(&key);

    Ok(StatusCode::OK)
}
