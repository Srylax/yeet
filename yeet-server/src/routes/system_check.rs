use std::sync::Arc;
use std::time::Instant;

use crate::claim::Claims;
use crate::error::IntoResponseWithToken;
use crate::error::YeetError;
use crate::AppState;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use axum_thiserror::ErrorStatus;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use yeet_api::VersionStatus::{NewVersionAvailable, UpToDate};

#[derive(Serialize, Deserialize)]
pub struct VersionRequest {
    store_path: String,
}

#[derive(Error, Debug, ErrorStatus)]
pub enum SystemCheckError {
    #[error(
        "Current registered version does not match the provided version.\
        If you think this is a mistake, please update the version."
    )]
    #[status(StatusCode::BAD_REQUEST)]
    VersionMismatch,
}

pub async fn system_check(
    State(state): State<Arc<RwLock<AppState>>>,
    Path(hostname): Path<String>,
    claims: Claims,
    Json(VersionRequest { store_path }): Json<VersionRequest>,
) -> Result<Response, Response> {
    let mut state = state.write_arc();
    let jwt = claims
        .rotate(&mut state, hostname.clone()) // Requires implicitly the `Capability::SystemCheck { hostname }` capability
        .map_err(IntoResponse::into_response)?;

    let Some(host) = state.hosts.get_mut(&hostname) else {
        return Err(YeetError::HostNotFound(hostname).with_token(&jwt));
    };

    // If the client did the update
    if let NewVersionAvailable(ref next_version) = host.status {
        if next_version.store_path == store_path {
            host.store_path.clone_from(&store_path);
            host.status = UpToDate;
        }
    }

    // Version mismatch
    if host.store_path != store_path {
        return Err(SystemCheckError::VersionMismatch.with_token(&jwt));
    }

    host.last_ping = Some(Instant::now());

    Ok(Json(host.status.clone()).with_token(&jwt))
}
