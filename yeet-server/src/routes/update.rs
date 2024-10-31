use std::sync::Arc;

use crate::claim::Claims;
use crate::error::YeetError;
use crate::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use parking_lot::RwLock;
use yeet_api::VersionStatus::NewVersionAvailable;
use yeet_api::{Capability, HostUpdateRequest, Version};

pub async fn update_hosts(
    State(state): State<Arc<RwLock<AppState>>>,
    claim: Claims,
    Json(req): Json<HostUpdateRequest>,
) -> Result<StatusCode, YeetError> {
    claim.require(Capability::Update)?;
    let mut state = state.write();

    let unknown_hosts: Vec<&str> = req
        .hosts
        .iter()
        .filter(|host| !state.hosts.contains_key(&host.hostname))
        .map(|host| host.hostname.as_str())
        .collect();

    if let Some(host) = unknown_hosts.first() {
        return Err(YeetError::HostNotFound((**host).to_string()));
    }

    for host_update in req.hosts {
        let Some(host) = state.hosts.get_mut(&host_update.hostname) else {
            return Err(YeetError::HostNotFound(host_update.hostname));
        };
        let version = Version {
            store_path: host_update.store_path,
            substitutor: req.substitutor.clone(),
            public_key: req.public_key.clone(),
        };
        host.status = NewVersionAvailable(version);
    }

    Ok(StatusCode::CREATED)
}
