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
use chrono::Duration;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::claim::Claims;
use crate::AppState;
use yeet_api::Capability;
use yeet_server::Jti;

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

    // Decodes JWT and checks JTI blacklist
    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let state: Arc<RwLock<AppState>> = Arc::from_ref(state);
        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await?;
        let state = state.read();
        let claims = Claims::decode(bearer.token(), parts.uri.path(), &state.jwt_secret)?;
        if state.jti_blacklist.contains(&claims.jti()) {
            return Err(JwtError::BlockedJTI);
        }
        Ok(claims)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct NextJwt(pub [(String, String); 1]);
#[async_trait]
impl<S> FromRequestParts<S> for NextJwt
where
    Arc<RwLock<AppState>>: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = JwtError;

    // After decoding and checking the JTI blacklist, creates a new JWT with a new JTI
    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let state: Arc<RwLock<AppState>> = Arc::from_ref(state);
        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await?;
        let Path(hostname) = parts.extract::<Path<String>>().await?;

        let mut state = state.write_arc();
        let jwt_secret = state.jwt_secret;
        let claims = Claims::decode(bearer.token(), parts.uri.path(), &jwt_secret)?;

        if state.jti_blacklist.contains(&claims.jti()) {
            return Err(JwtError::BlockedJTI);
        }

        let host = state
            .hosts
            .get_mut(&hostname)
            .ok_or(JwtError::HostNotFound(hostname.clone()))?;

        // JWT leaked and either the malicious actor or the agent tried to authenticate with an old JTI
        if host.jti != Jti::Jti(claims.jti()) {
            host.jti = Jti::Blocked;
            return Err(JwtError::InvalidJTI);
        }

        let claim = Claims::new(
            vec![Capability::SystemCheck { hostname }],
            Duration::days(30),
        );

        host.jti = Jti::Jti(claim.jti());
        Ok(NextJwt([(
            "X-Auth-Token".to_owned(),
            claim.encode(&jwt_secret)?,
        )]))
    }
}
