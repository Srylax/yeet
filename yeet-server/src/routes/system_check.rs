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
use yeet_api::{
    VersionRequest,
    VersionStatus::{self, NewVersionAvailable, UpToDate},
};

/// This is the "ping" command every client should send in a specific interval.
/// Updates are handeled implicitly. There is no seperate endpoint that the agent must call to inform the server of an update.
/// Updates are only accepted if the client is in `NewVersionAvailable` state.
/// Else the it is handeled as a version mismatch.
/// The store path needs to be signed by the host.
pub async fn system_check(
    State(state): State<Arc<RwLock<AppState>>>,
    HttpSig(key): HttpSig,
    VerifiedJson(VersionRequest { store_path }): VerifiedJson<VersionRequest>,
) -> Result<Json<VersionStatus>, (StatusCode, String)> {
    let mut state = state.write_arc();

    let Some(host) = state.hosts.get_mut(&key) else {
        return Err((StatusCode::NOT_FOUND, "Host not found".to_owned()));
    };

    // If the client did the update
    if let NewVersionAvailable(ref next_version) = host.status
        && next_version.store_path == store_path
    {
        // The versions match up
        host.store_path.clone_from(&store_path);
        host.status = UpToDate;
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

    host.last_ping = Some(Zoned::now());

    Ok(Json(host.status.clone()))
}

#[cfg(test)]
mod test_system_check {
    use std::sync::LazyLock;

    use ed25519_dalek::SigningKey;
    use httpsig_hyper::prelude::{
        AlgorithmName, HttpSignatureParams, SecretKey, SigningKey as _, message_component,
    };
    use yeet_api::{Version, httpsig::ReqwestSig};

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
    async fn test_no_update() {
        // Build Signature
        let mut signature_params = HttpSignatureParams::try_new(&COMPONENTS).unwrap();
        let signing_key = SecretKey::from_bytes(AlgorithmName::Ed25519, &SECRET_KEY_BYTES).unwrap();
        signature_params.set_key_info(&signing_key);

        let key = SigningKey::from_bytes(&SECRET_KEY_BYTES);
        let host = Host {
            store_path: "example_store_path".to_owned(),
            ..Default::default()
        };
        let mut state = AppState::default();
        state.keys.insert(signing_key.key_id(), key.verifying_key());
        state.hosts.insert(key.verifying_key(), host);

        let (server, _state) = test_server(state);

        let response = server
            .reqwest_post("/system/check")
            .json(&VersionRequest {
                store_path: "example_store_path".to_owned(),
            })
            .sign(&signature_params, &signing_key)
            .await
            .unwrap()
            .send()
            .await
            .unwrap()
            .json::<VersionStatus>()
            .await
            .unwrap();
        assert_eq!(response, VersionStatus::UpToDate);
    }

    #[tokio::test]
    async fn test_with_update() {
        // Build Signature
        let mut signature_params = HttpSignatureParams::try_new(&COMPONENTS).unwrap();
        let signing_key = SecretKey::from_bytes(AlgorithmName::Ed25519, &SECRET_KEY_BYTES).unwrap();
        signature_params.set_key_info(&signing_key);

        let key = SigningKey::from_bytes(&SECRET_KEY_BYTES);
        let host = Host {
            store_path: "example_store_path".to_owned(),
            status: VersionStatus::NewVersionAvailable(Version {
                public_key: "pub_key".to_owned(),
                store_path: "new_store_path".to_owned(),
                substitutor: "substitutor".to_owned(),
            }),
            ..Default::default()
        };
        let mut state = AppState::default();
        state.hosts.insert(key.verifying_key(), host);
        state.keys.insert(signing_key.key_id(), key.verifying_key());

        let (server, _state) = test_server(state);

        let response = server
            .reqwest_post("/system/check")
            .json(&VersionRequest {
                store_path: "example_store_path".to_owned(),
            })
            .sign(&signature_params, &signing_key)
            .await
            .unwrap()
            .send()
            .await
            .unwrap()
            .json::<VersionStatus>()
            .await
            .unwrap();
        assert_eq!(
            response,
            VersionStatus::NewVersionAvailable(Version {
                public_key: "pub_key".to_owned(),
                store_path: "new_store_path".to_owned(),
                substitutor: "substitutor".to_owned(),
            })
        );
    }
}
