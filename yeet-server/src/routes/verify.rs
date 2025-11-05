use std::sync::Arc;

/// The goal is to no longer require the pub key at registration of the host.
/// Rather any unauthenticated client can try an `verification_attempt` and supply his public key.
/// This then generates a six digit number which the admin has to retrieve from the client (not the server!)
/// This ensure that the identity of the host is verified.
/// However the identity model is now flipped. Instead of just identifying the host based on
/// the public key it is now tied to an arbitrary name.
/// We could make it so that the client saves its hostname either by looking at its hostname
/// or via config. An other solution would be that when you run `yeet approve` and input the clients
/// one time pin that you also have to input the hostname that it should be associated with.
///
use axum::{Json, extract::State, http::StatusCode};
use ed25519_dalek::VerifyingKey;
use parking_lot::RwLock;

use crate::{
    httpsig::HttpSig,
    state::{AppState, StateError},
};

/// That is literally it because the `HttpSig` extractor checks if the key is in the keyids
pub async fn is_host_verified(HttpSig(_http_key): HttpSig) -> StatusCode {
    StatusCode::OK
}

/// Adds a new key as an verification attempt
pub async fn add_verification_attempt(
    State(state): State<Arc<RwLock<AppState>>>,
    Json(key): Json<VerifyingKey>,
) -> Result<Json<u32>, StateError> {
    Ok(Json(state.write_arc().add_verification_attempt(key)?))
}
