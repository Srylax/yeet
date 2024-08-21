use std::sync::Arc;

use crate::jwt::{create_jwt, Claims};
use crate::routes::register::RegisterError::HostAlreadyRegistered;
use crate::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use axum_thiserror::ErrorStatus;
use chrono::Duration;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;
use yeet_api::Capability;
use yeet_api::VersionStatus::UpToDate;
use yeet_server::{Host, Jti};

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
    #[error("Could not create token: {0}")]
    #[status(StatusCode::INTERNAL_SERVER_ERROR)]
    InvalidToken(#[from] jsonwebtoken::errors::Error),
}

pub async fn register_host(
    State(state): State<Arc<RwLock<AppState>>>,
    _claims: Claims,
    Json(HostRegister {
        store_path,
        hostname,
    }): Json<HostRegister>,
) -> Result<Json<Value>, RegisterError> {
    let mut state = state.write();
    if state.hosts.contains_key(&hostname) {
        return Err(HostAlreadyRegistered);
    }
    let (jwt, jti) = create_jwt(
        vec![Capability::SystemCheck {
            hostname: hostname.clone(),
        }],
        Duration::days(7),
        &state.jwt_secret,
    )?;
    let host = Host {
        hostname: hostname.clone(),
        store_path,
        status: UpToDate,
        jti: Jti::Jti(jti),
        last_ping: None,
    };

    state.hosts.insert(hostname, host);

    Ok(Json(json!({
        "token": jwt
    })))
}
