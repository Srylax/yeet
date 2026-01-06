use std::sync::Arc;

use axum::{Json,
           extract::{FromRequest, FromRequestParts, Request},
           http::{self, HeaderMap, StatusCode, header}};
use ed25519_dalek::VerifyingKey;
use httpsig_hyper::{ContentDigest as _,
                    MessageSignature as _,
                    MessageSignatureReq as _,
                    RequestContentDigest as _,
                    prelude::{AlgorithmName, PublicKey}};
use parking_lot::RwLock;
use serde::de::DeserializeOwned;

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
            .get_key_by_id(keyid)
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

pub struct VerifiedJson<T>(pub T);

impl<T, S> FromRequest<S> for VerifiedJson<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let req = req
            .verify_content_digest()
            .await
            .with_code(StatusCode::BAD_REQUEST)?;

        if !json_content_type(req.headers()) {
            return Err((
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "Expected request with `Content-Type: application/json`".to_owned(),
            ));
        }

        Json::from_bytes(
            &req.into_bytes()
                .await
                .with_code(StatusCode::INTERNAL_SERVER_ERROR)?,
        )
        .with_code(StatusCode::INTERNAL_SERVER_ERROR)
        .map(|json| VerifiedJson(json.0))
    }
}

fn json_content_type(headers: &HeaderMap) -> bool {
    let Some(content_type) = headers.get(header::CONTENT_TYPE) else {
        return false;
    };

    let Ok(content_type) = content_type.to_str() else {
        return false;
    };

    let Ok(mime) = content_type.parse::<mime::Mime>() else {
        return false;
    };

    mime.type_() == "application"
        && (mime.subtype() == "json" || mime.suffix().is_some_and(|name| name == "json"))
}
