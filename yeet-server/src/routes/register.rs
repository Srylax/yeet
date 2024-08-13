use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use axum::response::IntoResponse;
use chrono::Duration;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::json;

use yeet_api::Capability;
use yeet_api::VersionStatus::UpToDate;

use crate::{AppState, Host, JTI};
use crate::jwt::{Claims, create_jwt};

#[derive(Serialize, Deserialize)]
pub struct HostRegister {
    store_path: String,
    hostname: String,
}

pub async fn register_host(
    State(state): State<Arc<RwLock<AppState>>>,
    _claims: Claims,
    Json(HostRegister {
        store_path,
        hostname,
    }): Json<HostRegister>,
) -> impl IntoResponse {
    let mut state = state.write();
    if state.hosts.contains_key(&hostname) {
        return (
            StatusCode::BAD_REQUEST,
            "Host with this name already registered",
        )
            .into_response();
    }
    let (jwt, jti) = create_jwt(
        vec![Capability::SystemCheck {
            hostname: hostname.clone(),
        }],
        Duration::days(7),
        &state.jwt_secret,
    )
    .unwrap();
    let host = Host {
        hostname: hostname.clone(),
        store_path,
        status: UpToDate,
        jti: JTI::JTI(jti),
        last_ping: None,
    };

    state.hosts.insert(hostname, host);
    (
        StatusCode::CREATED,
        Json(json!({
            "token": jwt
        })),
    )
        .into_response()
}
