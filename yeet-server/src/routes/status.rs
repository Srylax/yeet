use std::sync::Arc;

use axum::{extract::State, http::StatusCode};
use parking_lot::RwLock;
use serde_json_any_key::MapIterToJson as _;

use crate::{AppState, error::WithStatusCode as _, httpsig::HttpSig};

// Not able to return hosts as a struct because of the way HashMap is structured
pub async fn status(
    State(state): State<Arc<RwLock<AppState>>>,
    HttpSig(key): HttpSig,
) -> Result<String, (StatusCode, String)> {
    if !state.read_arc().admin_credentials.contains(&key) {
        return Err((
            StatusCode::FORBIDDEN,
            "The request is authenticated but you lack admin credentials".to_owned(),
        ));
    }
    state
        .read_arc()
        .hosts
        .clone()
        .to_json_map()
        .with_code(StatusCode::INTERNAL_SERVER_ERROR)
}

#[cfg(test)]
mod test_status {
    use std::sync::LazyLock;

    use ed25519_dalek::SigningKey;
    use httpsig_hyper::prelude::*;
    use httpsig_hyper::prelude::{HttpSignatureParams, SecretKey};
    use yeet_api::httpsig::ReqwestSig as _;

    use super::*;
    use crate::{Host, test_server};

    static SECRET_KEY_BYTES: [u8; 32] = [
        157, 97, 177, 157, 239, 253, 90, 96, 186, 132, 74, 244, 146, 236, 44, 196, 68, 73, 197,
        105, 123, 50, 105, 25, 112, 59, 172, 3, 28, 174, 127, 96,
    ];

    static COMPONENTS: LazyLock<Vec<message_component::HttpMessageComponentId>> =
        LazyLock::new(|| {
            ["date", "@path", "@method", "content-digest"]
                .iter()
                .map(|component| message_component::HttpMessageComponentId::try_from(*component))
                .collect::<Result<Vec<_>, _>>()
                .unwrap()
        });

    #[tokio::test]
    async fn state() {
        // Build Signature
        let mut signature_params = HttpSignatureParams::try_new(&COMPONENTS).unwrap();
        let signing_key = SecretKey::from_bytes(AlgorithmName::Ed25519, &SECRET_KEY_BYTES).unwrap();
        signature_params.set_key_info(&signing_key);

        // Build State
        let mut state = AppState::default();

        let key = SigningKey::from_bytes(&SECRET_KEY_BYTES);
        // Just so we have some data
        let host = Host {
            store_path: "example_store_path".to_owned(),
            ..Default::default()
        };
        state.hosts.insert(key.verifying_key(), host);
        state.keys.insert(signing_key.key_id(), key.verifying_key());
        state.admin_credentials.insert(key.verifying_key()); // So we can access the data

        let (server, state) = test_server(state);

        // Build Request
        let response = server
            .reqwest_get("/status")
            .sign(&signature_params, &signing_key)
            .await
            .unwrap()
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();

        assert_eq!(response, state.read().hosts.to_json_map().unwrap());
    }

    #[tokio::test]
    async fn non_admin() {
        // Build Signature
        let mut signature_params = HttpSignatureParams::try_new(&COMPONENTS).unwrap();
        let signing_key = SecretKey::from_bytes(AlgorithmName::Ed25519, &SECRET_KEY_BYTES).unwrap();
        signature_params.set_key_info(&signing_key);

        // Build State
        let mut state = AppState::default();

        let key = SigningKey::from_bytes(&SECRET_KEY_BYTES);
        state.keys.insert(signing_key.key_id(), key.verifying_key());

        let (mut server, _state) = test_server(state);
        server.expect_failure(); // We expect a 403

        // Build Request
        let response = server
            .reqwest_get("/status")
            .sign(&signature_params, &signing_key)
            .await
            .unwrap()
            .send()
            .await
            .unwrap()
            .status();

        assert_eq!(response, StatusCode::FORBIDDEN);
    }
}
