use std::sync::Arc;
use std::time::Instant;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use axum::response::IntoResponse;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::json;

use yeet_api::VersionStatus::{NewVersionAvailable, UpToDate};

use crate::AppState;
use crate::jwt::NextJwt;

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
        return (StatusCode::NOT_FOUND, "Host not registered").into_response();
    };

    // If the client did the update
    if let NewVersionAvailable(next_version) = &host.status {
        if next_version.store_path == store_path {
            host.store_path = store_path.clone();
            host.status = UpToDate;
        }
    }

    // Version mismatch
    if host.store_path != store_path {
        return (StatusCode::BAD_REQUEST, "Current version mismatch").into_response();
    }
    host.last_ping = Some(Instant::now());

    Json(json!({
        "status": host.status,
        "token": jwt
    }))
    .into_response()
}
