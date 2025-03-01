use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use axum_extra::headers::authorization::Bearer;
use axum_extra::headers::Authorization;
use axum_extra::TypedHeader;
use jsonwebtoken::{DecodingKey, Validation};
use parking_lot::RwLock;
use serde_json::{json, Value};

use crate::claim::Claims;
use crate::error::YeetError;
use crate::AppState;
use yeet_api::{Capability, TokenRequest};
use yeet_server::Jti;

pub async fn create_token(
    State(state): State<Arc<RwLock<AppState>>>,
    claims: Claims,
    Json(TokenRequest { capabilities, exp }): Json<TokenRequest>,
) -> Result<Json<Value>, YeetError> {
    let mut state = state.write();
    let mut token_cap = claims
        .cap()
        .iter()
        .find_map(Capability::token) // Requires implicitly the `Capability::Token { C }` capability
        .ok_or(YeetError::MissingCapability(Capability::Token {
            capabilities: capabilities.clone(),
        }))?
        .clone();

    // With Capability::Token { C }, you can grant capability C and Capability::Token { C }
    token_cap.extend(token_cap.clone().into_iter().map(|cap| Capability::Token {
        capabilities: vec![cap],
    }));

    if let Some(missing_cap) = capabilities.iter().find(|cap| !token_cap.contains(cap)) {
        return Err(YeetError::MissingCapability(Capability::Token {
            capabilities: vec![missing_cap.clone()],
        }));
    }

    let new_claims = Claims::new(capabilities.clone(), exp);
    let token = new_claims.encode(&state.jwt_secret)?; // encode before modifying state

    // If this is a host registration token, we need to unblock the host
    for capability in capabilities {
        let Capability::SystemCheck { hostname } = capability else {
            continue;
        };
        let Some(host) = state.hosts.get_mut(&hostname) else {
            state.jti_blacklist.insert(new_claims.jti());
            return Err(YeetError::HostNotFound(hostname));
        };
        host.jti = Jti::Jti(new_claims.jti());
    }

    Ok(Json(json!({"token": token})))
}

pub async fn revoke_token(
    State(state): State<Arc<RwLock<AppState>>>,
    TypedHeader(Authorization(bearer)): TypedHeader<Authorization<Bearer>>,
) -> impl IntoResponse {
    let mut state = state.write();
    let mut val = Validation::default();
    val.validate_aud = false;
    let Ok(token) = jsonwebtoken::decode::<Claims>(
        bearer.token(),
        &DecodingKey::from_secret(&state.jwt_secret),
        &val,
    ) else {
        return (StatusCode::OK, "Token already invalid").into_response();
    };
    state.jti_blacklist.insert(token.claims.jti());
    StatusCode::OK.into_response()
}

#[cfg(test)]
mod test_token {
    use super::*;
    use crate::{test_access, test_server};
    use axum::response::IntoResponse;
    use yeet_api::Capability;
    use yeet_api::VersionStatus::{NewVersionAvailable, UpToDate};
    use yeet_api::{HostUpdate, HostUpdateRequest, Version};
    use yeet_server::{Host, Jti};

    test_access!(
        create_token,
        Capability::Token {
            capabilities: vec![Capability::Update]
        }
    );

    async fn create_token(capabilities: Vec<Capability>) {
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
        let (server, state) = test_server(app_state, capabilities);

        server
            .post("/token/new")
            .json(&HostUpdateRequest {
                hosts: vec![HostUpdate {
                    hostname: "my-host".to_owned(),
                    store_path: "/nix/store/abc".to_owned(),
                }],
                substitutor: "my-substitutor".to_owned(),
                public_key: "my-key".to_owned(),
            })
            .await
            .assert_status(StatusCode::CREATED);

        let state = state.read_arc();

        assert_eq!(
            state.hosts.get("my-host"),
            Some(&Host {
                hostname: "my-host".to_owned(),
                store_path: "/nix/store/abc".to_owned(),
                status: NewVersionAvailable(Version {
                    store_path: "/nix/store/abc".to_owned(),
                    substitutor: "my-substitutor".to_owned(),
                    public_key: "my-key".to_owned(),
                }),
                jti: Jti::Blocked,
                last_ping: None,
            })
        );
    }

    #[tokio::test]
    async fn host_not_found() {
        let (mut server, _state) = test_server(AppState::default(), vec![Capability::Update]);

        server.expect_failure();

        let response = server
            .post("/system/update")
            .json(&HostUpdateRequest {
                hosts: vec![HostUpdate {
                    hostname: "my-host".to_owned(),
                    store_path: "/nix/store/abc".to_owned(),
                }],
                substitutor: "my-substitutor".to_owned(),
                public_key: "my-key".to_owned(),
            })
            .await;

        response.assert_status(
            YeetError::HostNotFound(String::new())
                .into_response()
                .status(),
        );
        response.assert_text(YeetError::HostNotFound("my-host".to_owned()).to_string());
    }
}
