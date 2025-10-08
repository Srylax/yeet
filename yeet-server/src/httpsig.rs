use std::sync::Arc;

use axum::{
    extract::FromRequestParts,
    http::{self, StatusCode},
};
use ed25519_dalek::VerifyingKey;
use httpsig_hyper::{
    MessageSignature as _, MessageSignatureReq as _,
    prelude::{AlgorithmName, PublicKey},
};
use parking_lot::RwLock;

use crate::{AppState, error::WithStatusCode as _};

pub struct HttpSig(pub VerifyingKey);

impl FromRequestParts<Arc<RwLock<AppState>>> for HttpSig {
    type Rejection = (StatusCode, String);

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &Arc<RwLock<AppState>>,
    ) -> Result<Self, Self::Rejection> {
        let req = http::Request::from_parts(parts.clone(), String::new());

        let keyids = req.get_key_ids().with_code(StatusCode::BAD_REQUEST)?;
        if keyids.len() != 1 {
            return Err((
                StatusCode::BAD_REQUEST,
                "KeyIDs must be exactly one".to_owned(),
            ));
        }

        let (_signature, keyid) = keyids
            .first()
            .expect("This is safe as long as we check the keyid length");

        let Some(verifying_key) = state
            .try_read()
            .ok_or("Internal State currently not available - try again later")
            .with_code(StatusCode::INTERNAL_SERVER_ERROR)?
            .keys
            .get(keyid)
            .copied()
        else {
            return Err((
                StatusCode::BAD_REQUEST,
                "The KeyID is not registered".to_owned(),
            ));
        };

        let pub_key = PublicKey::from_bytes(AlgorithmName::Ed25519, verifying_key.as_bytes())
            .with_code(StatusCode::BAD_REQUEST)?;

        req.verify_message_signature(&pub_key, Some(keyid))
            .await
            .with_code(StatusCode::BAD_REQUEST)?;

        Ok(HttpSig(verifying_key))
    }
}
