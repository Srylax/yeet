use std::sync::Arc;

use axum::{extract::State, http::StatusCode};
use parking_lot::RwLock;

use crate::{AppState,
            httpsig::{HttpSig, VerifiedJson},
            state::StateError};

/// Pre-registeres the host.
/// Currently only one host at a time
/// If the host already in pre-registered and still in the same state then
/// calling this again will can update the provision state
pub async fn register_host(
    State(state): State<Arc<RwLock<AppState>>>,
    HttpSig(http_key): HttpSig,
    VerifiedJson(api::RegisterHost {
        name,
        provision_state,
    }): VerifiedJson<api::RegisterHost>,
) -> Result<StatusCode, StateError> {
    let mut state = state.write_arc();

    state.auth_build(&http_key)?;

    if state.pre_register_host(name, provision_state).is_none() {
        Ok(StatusCode::CREATED)
    } else {
        Ok(StatusCode::OK)
    }
}
