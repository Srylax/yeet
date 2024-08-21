use std::sync::Arc;

use axum::extract::rejection::PathRejection;
use axum::extract::{FromRef, FromRequestParts, Path};
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum::{async_trait, RequestPartsExt};
use axum_extra::headers::authorization::Bearer;
use axum_extra::headers::Authorization;
use axum_extra::typed_header::TypedHeaderRejection;
use axum_extra::TypedHeader;
use axum_thiserror::ErrorStatus;
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{encode, DecodingKey, EncodingKey, Header, Validation};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::AppState;
use yeet_api::Capability;
use yeet_server::Jti;

fn jwt_validation(aud: &str) -> Validation {
    let mut val = Validation::default();
    val.set_required_spec_claims(&["aud", "exp", "jti"]);
    val.set_audience(&[aud]);
    val
}

#[derive(Serialize, Deserialize)]
pub struct Claims {
    pub aud: Vec<String>,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub exp: DateTime<Utc>,
    pub jti: Uuid,
}

#[derive(Error, Debug, ErrorStatus)]
pub enum JwtError {
    #[error("Invalid Token: {0}")]
    #[status(StatusCode::BAD_REQUEST)]
    TokenMalformed(#[from] TypedHeaderRejection),
    #[error("Invalid Token: {0}")]
    #[status(StatusCode::UNAUTHORIZED)]
    InvalidToken(#[from] jsonwebtoken::errors::Error),
    #[error("Invalid Host")]
    #[status(StatusCode::BAD_REQUEST)]
    PathError(#[from] PathRejection),
    #[error("Host {0} not found")]
    #[status(StatusCode::NOT_FOUND)]
    HostNotFound(String),
    #[error("Invalid JTI. Host is blocked and needs re-authentication")]
    #[status(StatusCode::FORBIDDEN)]
    InvalidJTI,
    #[error("Token was revoked")]
    #[status(StatusCode::FORBIDDEN)]
    BlockedJTI,
}
#[async_trait]
impl<S> FromRequestParts<S> for Claims
where
    Arc<RwLock<AppState>>: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = JwtError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let state: Arc<RwLock<AppState>> = Arc::from_ref(state);
        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await?;
        let state = state.read();
        let claims = jsonwebtoken::decode::<Claims>(
            bearer.token(),
            &DecodingKey::from_secret(&state.jwt_secret),
            &jwt_validation(parts.uri.path()),
        )?
        .claims;
        if state.jti_blacklist.contains(&claims.jti) {
            return Err(JwtError::BlockedJTI);
        }
        Ok(claims)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct NextJwt(pub String);
#[async_trait]
impl<S> FromRequestParts<S> for NextJwt
where
    Arc<RwLock<AppState>>: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = JwtError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let state: Arc<RwLock<AppState>> = Arc::from_ref(state);
        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await?;
        let Path(hostname) = parts.extract::<Path<String>>().await?;

        let mut state = state.write_arc();
        let jwt_secret = state.jwt_secret;
        let claims = jsonwebtoken::decode::<Claims>(
            bearer.token(),
            &DecodingKey::from_secret(&jwt_secret),
            &jwt_validation(parts.uri.path()),
        )?
        .claims;

        if state.jti_blacklist.contains(&claims.jti) {
            return Err(JwtError::BlockedJTI);
        }

        let host = state
            .hosts
            .get_mut(&hostname)
            .ok_or(JwtError::HostNotFound(hostname.clone()))?;

        // JWT leaked and either the malicious actor or the agent tried to authenticate with an old JTI
        if host.jti != Jti::Jti(claims.jti) {
            host.jti = Jti::Blocked;
            return Err(JwtError::InvalidJTI);
        }
        let (jwt, jti) = create_jwt(
            vec![Capability::SystemCheck { hostname }],
            Duration::days(30),
            &jwt_secret,
        )?;

        host.jti = Jti::Jti(jti);
        Ok(NextJwt(jwt))
    }
}

pub fn create_jwt(
    aud: Vec<Capability>,
    duration: Duration,
    secret: &[u8],
) -> jsonwebtoken::errors::Result<(String, Uuid)> {
    let uuid = Uuid::now_v7();
    let claims = Claims {
        aud: aud
            .into_iter()
            .flat_map(Into::<Vec<String>>::into)
            .collect(),
        #[allow(clippy::arithmetic_side_effects)]
        exp: Utc::now() + duration,
        jti: uuid,
    };
    let jwt = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret),
    )?;
    Ok((jwt, uuid))
}
