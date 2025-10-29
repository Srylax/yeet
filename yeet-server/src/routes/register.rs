use std::sync::Arc;

use crate::{
    AppState,
    httpsig::{HttpSig, VerifiedJson},
};
use axum::extract::State;
use axum::http::StatusCode;
use httpsig_hyper::prelude::{AlgorithmName, SecretKey, SigningKey as _};
use parking_lot::RwLock;

/// Register a new host with the server.
/// Requires the key and the current store path
/// Only Build Machines are allowed to register.
/// Therefore the signature contains the host key and path signed by any build machine.
pub async fn register_host(
    State(state): State<Arc<RwLock<AppState>>>,
    HttpSig(http_key): HttpSig,
    VerifiedJson(api::RegisterHost {
        key,
        store_path,
        name,
    }): VerifiedJson<api::RegisterHost>,
) -> Result<StatusCode, (StatusCode, String)> {
    let mut state = state.write_arc();

    if !state.admin_credentials.contains(&http_key)
        && !state.build_machines_credentials.contains(&http_key)
    {
        return Err((
            StatusCode::FORBIDDEN,
            "The request is authenticated but you lack admin credentials".to_owned(),
        ));
    }

    if state.hosts.contains_key(&name) {
        return Err((
            StatusCode::BAD_REQUEST,
            "Host already registered".to_owned(),
        ));
    }

    let host = api::Host {
        store_path: store_path.unwrap_or_default(),
        key: key.into(),
        name: name.clone(),
        ..Default::default()
    };

    state.hosts.insert(name.clone(), host);
    if let Some(key) = key {
        state.host_by_key.insert(key, name);
        let signing_key = SecretKey::from_bytes(AlgorithmName::Ed25519, key.as_bytes())
            .expect("Verifying key already is validated");

        state.keys.insert(signing_key.key_id(), key);
    }

    Ok(StatusCode::CREATED)
}

#[cfg(test)]
mod test_register {
    use std::sync::LazyLock;

    use api::{RegisterHost, httpsig::ReqwestSig};
    use ed25519_dalek::SigningKey;
    use httpsig_hyper::prelude::{
        AlgorithmName, HttpSignatureParams, SecretKey, SigningKey as _, message_component,
    };

    use super::*;
    use crate::test_server;

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
    async fn test_register() {
        // Build Signature
        let mut signature_params = HttpSignatureParams::try_new(&COMPONENTS).unwrap();
        let signing_key = SecretKey::from_bytes(AlgorithmName::Ed25519, &SECRET_KEY_BYTES).unwrap();
        signature_params.set_key_info(&signing_key);

        let key = SigningKey::from_bytes(&SECRET_KEY_BYTES);
        let mut state = AppState::default();
        state.build_machines_credentials.insert(key.verifying_key());
        state.keys.insert(signing_key.key_id(), key.verifying_key());

        let (server, state) = test_server(state);

        server
            .reqwest_post("/system/register")
            .json(&api::RegisterHost {
                key: Some(key.verifying_key()),
                ..Default::default()
            })
            .sign(&signature_params, &signing_key)
            .await
            .unwrap()
            .send()
            .await
            .unwrap();
        let state = state.read_arc();

        assert_eq!(
            state.hosts[&String::new()],
            api::Host {
                key: api::Key::Verified(key.verifying_key()),
                ..Default::default()
            }
        );
    }

    #[tokio::test]
    async fn test_already_registered() {
        // Build Signature
        let mut signature_params = HttpSignatureParams::try_new(&COMPONENTS).unwrap();
        let signing_key = SecretKey::from_bytes(AlgorithmName::Ed25519, &SECRET_KEY_BYTES).unwrap();
        signature_params.set_key_info(&signing_key);

        let key = SigningKey::from_bytes(&SECRET_KEY_BYTES);
        let mut state = AppState::default();
        let host = api::Host {
            ..Default::default()
        };
        state.hosts.insert(String::new(), host);
        state.keys.insert(signing_key.key_id(), key.verifying_key());

        let (mut server, _state) = test_server(state);
        server.expect_failure();

        server
            .reqwest_post("/system/register")
            .json(&RegisterHost {
                ..Default::default()
            })
            .sign(&signature_params, &signing_key)
            .await
            .unwrap()
            .send()
            .await
            .unwrap();
    }
}
