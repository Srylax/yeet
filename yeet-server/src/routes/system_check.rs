use std::sync::Arc;

use crate::{
    AppState,
    httpsig::{HttpSig, VerifiedJson},
};
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use jiff::Zoned;
use parking_lot::RwLock;

/// This is the "ping" command every client should send in a specific interval.
/// Updates are handeled implicitly. There is no seperate endpoint that the agent must call to inform the server of an update.
/// Updates are only accepted if the client is in `NewVersionAvailable` state.
/// Else the it is handeled as a version mismatch.
/// The store path needs to be signed by the host.
pub async fn system_check(
    State(state): State<Arc<RwLock<AppState>>>,
    HttpSig(key): HttpSig,
    VerifiedJson(api::VersionRequest { store_path }): VerifiedJson<api::VersionRequest>,
) -> Result<Json<api::HostState>, (StatusCode, String)> {
    let mut state = state.write_arc();

    let Some(host_name) = state.host_by_key.get(&key).cloned() else {
        return Err((StatusCode::NOT_FOUND, "Host not found".to_owned()));
    };

    let Some(host) = state.hosts.get_mut(host_name.clone().as_str()) else {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Host key found but no matching host entry".to_owned(),
        ));
    };

    // If the client did the update
    if let api::HostState::Provisioned(api::ProvisionState::NewVersionAvailable(ref next_version)) =
        host.status
        && next_version.store_path == store_path
    {
        // The versions match up
        host.store_path.clone_from(&store_path);
        host.status = api::HostState::Provisioned(api::ProvisionState::UpToDate);
    }

    if host.status == api::HostState::New && host.store_path.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "This host has state new but has not yet an registered version".to_owned(),
        ));
    }
    // Version mismatch -> this can happen when manually applying or when some tampers with the server
    // In the future we may want to lock up the host for modifications if that happens
    // or at least warn an admin
    if host.store_path != store_path {
        return Err((
            StatusCode::BAD_REQUEST,
            "Current registered version does not match the provided version.\
                If you think this is a mistake, please update the version."
                .to_owned(),
        ));
    }

    match host.status {
        api::HostState::New => {
            host.status = api::HostState::Provisioned(api::ProvisionState::UpToDate);
        }
        api::HostState::Detached => {
            host.store_path = store_path;
        }
        api::HostState::Provisioned(ref _provision) => {}
    }

    host.last_ping = Some(Zoned::now());

    Ok(Json(host.status.clone()))
}

#[cfg(test)]
mod test_system_check {
    use std::sync::LazyLock;

    use api::httpsig::ReqwestSig as _;
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
    async fn test_no_update() {
        // Build Signature
        let mut signature_params = HttpSignatureParams::try_new(&COMPONENTS).unwrap();
        let signing_key = SecretKey::from_bytes(AlgorithmName::Ed25519, &SECRET_KEY_BYTES).unwrap();
        signature_params.set_key_info(&signing_key);

        let key = SigningKey::from_bytes(&SECRET_KEY_BYTES);
        let host = api::Host {
            store_path: "example_store_path".to_owned(),
            ..Default::default()
        };
        let mut state = AppState::default();
        state.keys.insert(signing_key.key_id(), key.verifying_key());
        state.hosts.insert(String::new(), host);
        state.host_by_key.insert(key.verifying_key(), String::new());

        let (server, _state) = test_server(state);

        let response = server
            .reqwest_post("/system/check")
            .json(&api::VersionRequest {
                store_path: "example_store_path".to_owned(),
            })
            .sign(&signature_params, &signing_key)
            .await
            .unwrap()
            .send()
            .await
            .unwrap()
            .json::<api::HostState>()
            .await
            .unwrap();
        assert_eq!(
            response,
            api::HostState::Provisioned(api::ProvisionState::UpToDate)
        );
    }

    #[tokio::test]
    async fn test_with_update() {
        // Build Signature
        let mut signature_params = HttpSignatureParams::try_new(&COMPONENTS).unwrap();
        let signing_key = SecretKey::from_bytes(AlgorithmName::Ed25519, &SECRET_KEY_BYTES).unwrap();
        signature_params.set_key_info(&signing_key);

        let key = SigningKey::from_bytes(&SECRET_KEY_BYTES);
        let host = api::Host {
            store_path: "example_store_path".to_owned(),
            status: api::HostState::Provisioned(api::ProvisionState::NewVersionAvailable(
                api::Version {
                    public_key: "pub_key".to_owned(),
                    store_path: "new_store_path".to_owned(),
                    substitutor: "substitutor".to_owned(),
                },
            )),
            ..Default::default()
        };
        let mut state = AppState::default();
        state.hosts.insert(String::new(), host);
        state.host_by_key.insert(key.verifying_key(), String::new());
        state.keys.insert(signing_key.key_id(), key.verifying_key());

        let (server, _state) = test_server(state);

        let response = server
            .reqwest_post("/system/check")
            .json(&api::VersionRequest {
                store_path: "example_store_path".to_owned(),
            })
            .sign(&signature_params, &signing_key)
            .await
            .unwrap()
            .send()
            .await
            .unwrap()
            .json::<api::HostState>()
            .await
            .unwrap();
        assert_eq!(
            response,
            api::HostState::Provisioned(api::ProvisionState::NewVersionAvailable(api::Version {
                public_key: "pub_key".to_owned(),
                store_path: "new_store_path".to_owned(),
                substitutor: "substitutor".to_owned(),
            }))
        );
    }

    #[tokio::test]
    async fn new_to_uptodate() {
        // Build Signature
        let mut signature_params = HttpSignatureParams::try_new(&COMPONENTS).unwrap();
        let signing_key = SecretKey::from_bytes(AlgorithmName::Ed25519, &SECRET_KEY_BYTES).unwrap();
        signature_params.set_key_info(&signing_key);

        let key = SigningKey::from_bytes(&SECRET_KEY_BYTES);
        let host = api::Host {
            store_path: "expected_store_path".to_owned(),
            status: api::HostState::New,
            ..Default::default()
        };
        let mut state = AppState::default();
        state.hosts.insert(String::new(), host);
        state.host_by_key.insert(key.verifying_key(), String::new());
        state.keys.insert(signing_key.key_id(), key.verifying_key());

        let (server, _state) = test_server(state);

        let response = server
            .reqwest_post("/system/check")
            .json(&api::VersionRequest {
                store_path: "expected_store_path".to_owned(),
            })
            .sign(&signature_params, &signing_key)
            .await
            .unwrap()
            .send()
            .await
            .unwrap()
            .json::<api::HostState>()
            .await
            .unwrap();
        assert_eq!(
            response,
            api::HostState::Provisioned(api::ProvisionState::UpToDate)
        );
    }

    #[tokio::test]
    async fn new_with_wrong_path() {
        // Build Signature
        let mut signature_params = HttpSignatureParams::try_new(&COMPONENTS).unwrap();
        let signing_key = SecretKey::from_bytes(AlgorithmName::Ed25519, &SECRET_KEY_BYTES).unwrap();
        signature_params.set_key_info(&signing_key);

        let key = SigningKey::from_bytes(&SECRET_KEY_BYTES);
        let host = api::Host {
            store_path: "expected_store_path".to_owned(),
            status: api::HostState::New,
            ..Default::default()
        };
        let mut state = AppState::default();
        state.hosts.insert(String::new(), host);
        state.host_by_key.insert(key.verifying_key(), String::new());
        state.keys.insert(signing_key.key_id(), key.verifying_key());

        let (mut server, _state) = test_server(state);
        server.expect_failure();

        server
            .reqwest_post("/system/check")
            .json(&api::VersionRequest {
                store_path: "wrong_store_path".to_owned(),
            })
            .sign(&signature_params, &signing_key)
            .await
            .unwrap()
            .send()
            .await
            .unwrap();
    }
}
