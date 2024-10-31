use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum_extra::typed_header::TypedHeaderRejection;
use axum_thiserror::ErrorStatus;
use thiserror::Error;
use yeet_api::Capability;
/// The difference to the errors defined in `routes`, `YeetError` can occur in multiple routes.
#[derive(Error, Debug, ErrorStatus)]
pub enum YeetError {
    #[error("Token Malformed: {0}")]
    #[status(StatusCode::BAD_REQUEST)]
    TokenMalformed(#[from] TypedHeaderRejection),
    #[error("Capability `{0:?}` required to access this resource")]
    #[status(StatusCode::FORBIDDEN)]
    MissingCapability(Capability),
    #[error("Host {0} not found")]
    #[status(StatusCode::NOT_FOUND)]
    HostNotFound(String),
    #[error("Blocked JTI. Host is blocked and needs re-authentication")]
    #[status(StatusCode::FORBIDDEN)]
    BlockedJTI,
    #[error("Invalid Audience")]
    #[status(StatusCode::UNAUTHORIZED)]
    InvalidAudience,
    #[error("Invalid Token: {0}")]
    #[status(StatusCode::UNAUTHORIZED)]
    InvalidToken(#[from] jsonwebtoken::errors::Error),
}

pub trait IntoResponseWithToken: IntoResponse + Sized {
    fn with_token(self, jwt: &str) -> Response {
        ([("X-Auth-Token", jwt)], self.into_response()).into_response()
    }
}

impl<T: IntoResponse> IntoResponseWithToken for T {}

impl From<YeetError> for Response {
    #[inline]
    fn from(value: YeetError) -> Self {
        value.into_response()
    }
}
