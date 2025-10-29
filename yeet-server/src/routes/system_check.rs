use std::sync::Arc;

use crate::{
    AppState,
    httpsig::{HttpSig, VerifiedJson},
    state::StateError,
};
use axum::Json;
use axum::extract::State;
use parking_lot::RwLock;

pub async fn system_check(
    State(state): State<Arc<RwLock<AppState>>>,
    HttpSig(key): HttpSig,
    VerifiedJson(api::VersionRequest { store_path }): VerifiedJson<api::VersionRequest>,
) -> Result<Json<api::AgentAction>, StateError> {
    let mut state = state.write_arc();

    Ok(Json(state.system_check(store_path, &key)?))
}
