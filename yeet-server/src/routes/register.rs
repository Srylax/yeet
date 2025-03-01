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
    let mut state = state.write_arc();
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

#[cfg(test)]
mod test_register {
    use super::*;
    use crate::{test_access, test_server};

    #[derive(Deserialize)]
    struct Token {
        token: String,
    }

    test_access!(register_host, Capability::Register);

    async fn register_host(capabilities: Vec<Capability>) {
        let (server, state) = test_server(AppState::default(), capabilities);

        let response = server
            .post("/system/register")
            .json(&HostRegister {
                store_path: "/nix/store/abc".to_owned(),
                hostname: "my-host".to_owned(),
            })
            .await;

        let state = state.read_arc();
        let response = response.json::<Token>();

        let token =
            Claims::decode(&response.token, &state.jwt_secret).expect("Could not decode token");

        assert_eq!(
            state.hosts.get("my-host"),
            Some(&Host {
                hostname: "my-host".to_owned(),
                store_path: "/nix/store/abc".to_owned(),
                status: UpToDate,
                jti: Jti::Jti(token.jti()),
                last_ping: None,
            })
        );
    }

    #[tokio::test]
    async fn host_already_registered() {
        let mut app_state = AppState::default();
        app_state.hosts.insert(
            "my-host".to_owned(),
            Host {
                hostname: "my-host".to_owned(),
                store_path: "/nix/store/abc".to_owned(),
                status: UpToDate,
                jti: Jti::Blocked,
                last_ping: None,
            },
        );

        let (mut server, _state) = test_server(app_state, vec![Capability::Register]);

        server.expect_failure();

        let response = server
            .post("/system/register")
            .json(&HostRegister {
                store_path: "/nix/store/abc".to_owned(),
                hostname: "my-host".to_owned(),
            })
            .await;

        response.assert_status(HostAlreadyRegistered.into_response().status());
        response.assert_text(HostAlreadyRegistered.to_string());
    }
}
