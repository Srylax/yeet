use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use axum::response::IntoResponse;
use parking_lot::RwLock;

use yeet_api::{HostUpdateRequest, Version};
use yeet_api::VersionStatus::NewVersionAvailable;

use crate::AppState;
use crate::jwt::Claims;

pub async fn update_hosts(
    State(state): State<Arc<RwLock<AppState>>>,
    _claim: Claims,
    Json(req): Json<HostUpdateRequest>,
) -> impl IntoResponse {
    let mut state = state.write();

    let invalid_hosts: Vec<String> = req
        .hosts
        .iter()
        .filter(|host| !state.hosts.contains_key(&host.hostname))
        .map(|host| host.hostname.clone())
        .collect();

    if !invalid_hosts.is_empty() {
        return (
            StatusCode::NOT_FOUND,
            format!("{:?} not registered", invalid_hosts),
        )
            .into_response();
    }

    for host_update in req.hosts {
        let Some(host) = state.hosts.get_mut(&host_update.hostname) else {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Multithreading Error while accessing hosts",
            )
                .into_response();
        };
        let version = Version {
            store_path: host_update.store_path,
            substitutor: req.substitutor.clone(),
            public_key: req.public_key.clone(),
        };
        host.status = NewVersionAvailable(version);
    }

    StatusCode::CREATED.into_response()
}
