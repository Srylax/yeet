use std::sync::Arc;

use crate::error::WithStatusCode as _;
use crate::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use ed25519_dalek::Signature;
use parking_lot::RwLock;
use yeet_api::VersionStatus::NewVersionAvailable;
use yeet_api::{HostUpdateRequest, Version};

pub async fn update_hosts(
    State(state): State<Arc<RwLock<AppState>>>,
    Json((req, signature)): Json<(String, Signature)>,
) -> Result<StatusCode, (StatusCode, String)> {
    let mut state = state.write_arc();

    let valid_request = state
        .build_machines
        .iter()
        .any(|key| key.verify_strict(req.as_bytes(), &signature).is_ok());

    if !valid_request {
        return Err((
            StatusCode::UNAUTHORIZED,
            "Not a valid build machine signature".to_owned(),
        ));
    }

    let req: HostUpdateRequest = serde_json::from_str(&req).with_code(StatusCode::BAD_REQUEST)?;

    let unknown_host = req
        .hosts
        .iter()
        .any(|host| !state.hosts.contains_key(&host.key));

    if unknown_host {
        return Err((StatusCode::NOT_FOUND, "Host not found".to_owned()));
    }

    for host_update in req.hosts {
        let host = state
            .hosts
            .get_mut(&host_update.key)
            .ok_or((StatusCode::NOT_FOUND, "Host not found".to_owned()))?;
        let version = Version {
            store_path: host_update.store_path.clone(),
            substitutor: req.substitutor.clone(),
            public_key: req.public_key.clone(),
        };
        host.status = NewVersionAvailable(version);
    }

    Ok(StatusCode::CREATED)
}
