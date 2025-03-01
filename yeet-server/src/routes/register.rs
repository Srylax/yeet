use std::sync::Arc;

use crate::{AppState, Host};
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use ed25519_dalek::{Signature, VerifyingKey};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use yeet_api::VersionStatus::UpToDate;

#[derive(Serialize, Deserialize)]
pub struct HostRegister {
    key: VerifyingKey,
    signature: Signature,
    store_path: String,
}

pub async fn register_host(
    State(state): State<Arc<RwLock<AppState>>>,
    Json(HostRegister {
        store_path,
        key,
        signature,
    }): Json<HostRegister>,
) -> Result<StatusCode, (StatusCode, String)> {
    let mut state = state.write_arc();

    let valid_request = state.build_machines.iter().any(|key| {
        key.verify_strict(
            &[key.as_bytes(), store_path.as_bytes()].concat(),
            &signature,
        )
        .is_ok()
    });

    if !valid_request {
        return Err((
            StatusCode::UNAUTHORIZED,
            "Not a valid build machine signature".to_owned(),
        ));
    }

    if state.hosts.contains_key(&key) {
        return Err((
            StatusCode::BAD_REQUEST,
            "Host already registered".to_owned(),
        ));
    }

    let host = Host {
        key,
        store_path,
        status: UpToDate,
        last_ping: None,
    };

    state.hosts.insert(key, host);

    Ok(StatusCode::CREATED)
}
