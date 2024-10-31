use std::sync::Arc;

use crate::claim::Claims;
use crate::routes::register::RegisterError::HostAlreadyRegistered;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use axum_thiserror::ErrorStatus;
use chrono::Duration;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;
use yeet_api::Capability;
use yeet_api::VersionStatus::UpToDate;
use yeet_server::{AppState, Host, Jti};

#[derive(Serialize, Deserialize)]
pub struct HostRegister {
    store_path: String,
    hostname: String,
}

#[derive(Error, Debug, ErrorStatus)]
pub enum RegisterError {
    #[error("Host with this name already registered")]
    #[status(StatusCode::BAD_REQUEST)]
    HostAlreadyRegistered,
}

pub async fn register_host(
    State(state): State<Arc<RwLock<AppState>>>,
    claims: Claims,
    Json(HostRegister {
        store_path,
        hostname,
    }): Json<HostRegister>,
) -> Result<Json<Value>, Response> {
    claims.require(Capability::Register)?;
    let mut state = state.write();
    if state.hosts.contains_key(&hostname) {
        return Err(HostAlreadyRegistered.into_response());
    }

    let claims = Claims::new(
        vec![Capability::SystemCheck {
            hostname: hostname.clone(),
        }],
        Duration::days(7),
    );

    let host = Host {
        hostname: hostname.clone(),
        store_path,
        status: UpToDate,
        jti: Jti::Jti(claims.jti()),
        last_ping: None,
    };

    state.hosts.insert(hostname, host);

    Ok(Json(json!({
        "token": claims.encode(&state.jwt_secret)?,
    })))
}
