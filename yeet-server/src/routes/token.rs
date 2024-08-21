use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use axum_extra::headers::authorization::Bearer;
use axum_extra::headers::Authorization;
use axum_extra::TypedHeader;
use axum_thiserror::ErrorStatus;
use jsonwebtoken::{DecodingKey, Validation};
use parking_lot::RwLock;
use serde_json::{json, Value};
use thiserror::Error;

use crate::jwt::{create_jwt, Claims};
use crate::AppState;
use yeet_api::{Capability, TokenRequest};
use yeet_server::Jti;

#[derive(Error, Debug, ErrorStatus)]
pub enum CreateTokenError {
    #[error("Could not create token: {0}")]
    #[status(StatusCode::INTERNAL_SERVER_ERROR)]
    InvalidToken(#[from] jsonwebtoken::errors::Error),
    #[error("{0}: Host not Registered")]
    #[status(StatusCode::BAD_REQUEST)]
    HostNotFound(String),
}
pub async fn create_token(
    State(state): State<Arc<RwLock<AppState>>>,
    _claims: Claims,
    Json(TokenRequest { capabilities, exp }): Json<TokenRequest>,
) -> Result<Json<Value>, CreateTokenError> {
    let mut state = state.write();
    let (jwt, jti) = create_jwt(capabilities.clone(), exp, &state.jwt_secret)?;

    // If this is a host registration token, we need to unblock the host
    for capability in capabilities {
        let Capability::SystemCheck { hostname } = capability else {
            continue;
        };
        let Some(host) = state.hosts.get_mut(&hostname) else {
            state.jti_blacklist.insert(jti);
            return Err(CreateTokenError::HostNotFound(hostname));
        };
        host.jti = Jti::Jti(jti);
    }

    Ok(Json(json!({"token": jwt})))
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
    state.jti_blacklist.insert(token.claims.jti);
    StatusCode::OK.into_response()
}
