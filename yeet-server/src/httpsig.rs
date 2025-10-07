use axum::{
    extract::{FromRequest, Request},
    http::StatusCode,
};
use derive_more::From;
use httpsig_hyper::{
    MessageSignature as _, MessageSignatureReq as _, RequestContentDigest as _,
    prelude::{AlgorithmName, PublicKey},
};
use serde::{Deserialize, Serialize};

use crate::{AppState, error::WithStatusCode as _};

#[derive(Debug, From, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct KeyID(String);

impl FromRequest<AppState> for KeyID {
    type Rejection = (StatusCode, String);

    async fn from_request(req: Request, state: &AppState) -> Result<Self, Self::Rejection> {
        let req = req
            .verify_content_digest()
            .await
            .with_code(StatusCode::BAD_REQUEST)?;

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

        let Some(verifying_key) = state.keys.get(&keyid.to_owned().into()) else {
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

        Ok(keyid.to_owned().into())
    }
}
