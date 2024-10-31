use std::sync::Arc;

use crate::claim::Claims;
use crate::error::YeetError;
use crate::AppState;
use axum::extract::{FromRef, FromRequestParts};
use axum::http::request::Parts;
use axum::{async_trait, RequestPartsExt};
use axum_extra::headers::authorization::Bearer;
use axum_extra::headers::Authorization;
use axum_extra::TypedHeader;
use parking_lot::RwLock;

#[async_trait]
impl<S> FromRequestParts<S> for Claims
where
    Arc<RwLock<AppState>>: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = YeetError;

    // Decodes JWT and checks JTI blacklist
    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let state: Arc<RwLock<AppState>> = Arc::from_ref(state);
        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await?;
        let state = state.read();
        let claims = Claims::decode(bearer.token(), &state.jwt_secret)?;
        if state.jti_blacklist.contains(&claims.jti()) {
            return Err(YeetError::BlockedJTI);
        }
        Ok(claims)
    }
}
