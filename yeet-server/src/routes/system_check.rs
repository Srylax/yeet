use std::sync::Arc;
use std::time::Instant;

use crate::error::WithStatusCode as _;
use crate::AppState;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use ed25519_dalek::{Signature, VerifyingKey};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use yeet_api::VersionStatus::{self, NewVersionAvailable, UpToDate};

#[derive(Serialize, Deserialize)]
pub struct VersionRequest {
    signature: Signature,
    store_path: String,
}

pub async fn system_check(
    State(state): State<Arc<RwLock<AppState>>>,
    Path(hostname): Path<VerifyingKey>,
    Json(VersionRequest {
        store_path,
        signature,
    }): Json<VersionRequest>,
) -> Result<Json<VersionStatus>, (StatusCode, String)> {
    let mut state = state.write_arc();

    let Some(host) = state.hosts.get_mut(&hostname) else {
        return Err((StatusCode::NOT_FOUND, "Host not found".to_owned()));
    };

    host.key
        .verify_strict(store_path.as_bytes(), &signature)
        .with_code(StatusCode::FORBIDDEN)?;

    // If the client did the update
    if let NewVersionAvailable(ref next_version) = host.status {
        if next_version.store_path == store_path {
            host.store_path.clone_from(&store_path);
            host.status = UpToDate;
        }
    }

    // Version mismatch
    if host.store_path != store_path {
        return Err((
            StatusCode::BAD_REQUEST,
            "Current registered version does not match the provided version.\
                If you think this is a mistake, please update the version."
                .to_owned(),
        ));
    }

    host.last_ping = Some(Instant::now());

    Ok(Json(host.status.clone()))
}
