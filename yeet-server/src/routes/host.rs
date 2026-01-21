use std::sync::Arc;

use axum::{Json, extract::State, http::StatusCode};
use parking_lot::RwLock;

use crate::{
    httpsig::{HttpSig, VerifiedJson},
    state::{AppState, StateError},
};

/// Host creation is done via the approve command. Now we need functions to delete and rename hosts.

pub async fn remove_host(
    State(state): State<Arc<RwLock<AppState>>>,
    HttpSig(key): HttpSig,
    VerifiedJson(api::HostRemoveRequest { hostname }): VerifiedJson<api::HostRemoveRequest>,
) -> Result<Json<api::Host>, StateError> {
    let mut state = state.write_arc();
    state.auth_admin(&key)?;
    Ok(Json(state.remove_host(&hostname)?))
}

pub async fn rename_host(
    State(state): State<Arc<RwLock<AppState>>>,
    HttpSig(key): HttpSig,
    VerifiedJson(api::HostRenameRequest { old_name, new_name }): VerifiedJson<
        api::HostRenameRequest,
    >,
) -> Result<StatusCode, StateError> {
    let mut state = state.write_arc();
    state.auth_admin(&key)?;
    state.rename_host(&old_name, new_name)?;
    Ok(StatusCode::OK)
}
