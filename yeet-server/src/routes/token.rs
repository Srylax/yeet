use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use axum::response::IntoResponse;
use axum_extra::headers::Authorization;
use axum_extra::headers::authorization::Bearer;
use axum_extra::TypedHeader;
use axum_thiserror::ErrorStatus;
use jsonwebtoken::{DecodingKey, Validation};
use parking_lot::RwLock;
use serde_json::{json, Value};
use thiserror::Error;

use yeet_api::TokenRequest;

use crate::AppState;
use crate::jwt::{Claims, create_jwt};

#[derive(Error, Debug, ErrorStatus)]
pub enum CreateTokenError {
    #[error("Could not create token: {0}")]
    #[status(StatusCode::INTERNAL_SERVER_ERROR)]
    InvalidToken(#[from] jsonwebtoken::errors::Error),
}
pub async fn create_token(
    State(state): State<Arc<RwLock<AppState>>>,
    _claims: Claims,
    Json(TokenRequest { capabilities, exp }): Json<TokenRequest>,
) -> Result<Json<Value>, CreateTokenError> {
    let state = state.read();
    let (jwt, _jti) = create_jwt(capabilities, exp, &state.jwt_secret)?;
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
