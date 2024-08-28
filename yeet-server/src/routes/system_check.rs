use std::sync::Arc;
use std::time::Instant;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use yeet_api::VersionStatus::{NewVersionAvailable, UpToDate};

use crate::jwt::NextJwt;
use crate::AppState;

#[derive(Serialize, Deserialize)]
pub struct VersionRequest {
    store_path: String,
}

pub async fn system_check(
    State(state): State<Arc<RwLock<AppState>>>,
    Path(hostname): Path<String>,
    NextJwt(jwt): NextJwt,
    Json(VersionRequest { store_path }): Json<VersionRequest>,
) -> impl IntoResponse {
    let mut state = state.write_arc();

    let Some(host) = state.hosts.get_mut(&hostname) else {
        return (StatusCode::NOT_FOUND, jwt, "Host not registered").into_response();
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
        return (StatusCode::BAD_REQUEST, jwt, "Current version mismatch").into_response();
    }
    host.last_ping = Some(Instant::now());

    (jwt, Json(host.status.clone())).into_response()
}
