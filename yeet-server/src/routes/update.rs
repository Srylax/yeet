use std::sync::Arc;

use crate::AppState;
use crate::httpsig::{HttpSig, VerifiedJson};
use axum::extract::State;
use axum::http::StatusCode;
use parking_lot::RwLock;

/// Endpoint to set a new version for a host.
/// The whole request needs to be signed by a build machine.
/// The update consist of a simple `key` -> `version` and a `substitutor` which is where the agent should get its update
/// This means that for each origin e.g. cachix, you need to call update seperately
pub async fn update_hosts(
    State(state): State<Arc<RwLock<AppState>>>,
    HttpSig(http_key): HttpSig,

    VerifiedJson(api::HostUpdateRequest {
        hosts,
        public_key,
        substitutor,
    }): VerifiedJson<api::HostUpdateRequest>,
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

    let unknown_host = hosts.iter().any(|(key, _)| !state.hosts.contains_key(key));

    if unknown_host {
        return Err((StatusCode::NOT_FOUND, "Host not found".to_owned()));
    }

    for (key, store_path) in hosts {
        let host = state
            .hosts
            .get_mut(&key)
            .ok_or((StatusCode::NOT_FOUND, "Host not found".to_owned()))?;
        let version = api::Version {
            store_path: store_path.clone(),
            substitutor: substitutor.clone(),
            public_key: public_key.clone(),
        };
        host.status =
            api::HostState::Provisioned(api::ProvisionState::NewVersionAvailable(version));
    }

    Ok(StatusCode::CREATED)
}

#[cfg(test)]
mod test_update {
    use std::{collections::HashMap, sync::LazyLock};

    use api::httpsig::ReqwestSig;
    use ed25519_dalek::{SigningKey, VerifyingKey, ed25519::signature::SignerMut};
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
    async fn test_update() {
        // Build Signature
        let mut signature_params = HttpSignatureParams::try_new(&COMPONENTS).unwrap();
        let signing_key = SecretKey::from_bytes(AlgorithmName::Ed25519, &SECRET_KEY_BYTES).unwrap();
        signature_params.set_key_info(&signing_key);

        let mut key = SigningKey::from_bytes(&SECRET_KEY_BYTES);
        let host = api::Host::default();
        let mut state = AppState::default();
        state.hosts.insert(VerifyingKey::default(), host);
        state.build_machines_credentials.insert(key.verifying_key());
        state.keys.insert(signing_key.key_id(), key.verifying_key());

        let (server, state) = test_server(state);

        let request = api::HostUpdateRequest {
            hosts: HashMap::from([(VerifyingKey::default(), "new_path".to_owned())]),
            public_key: "p_key".to_owned(),
            substitutor: "sub".to_owned(),
        };

        server
            .reqwest_post("/system/update")
            .json(&request)
            .sign(&signature_params, &signing_key)
            .await
            .unwrap()
            .send()
            .await
            .unwrap();
        let state = state.read_arc();

        assert_eq!(
            state.hosts[&VerifyingKey::default()].status,
            api::HostState::Provisioned(api::ProvisionState::NewVersionAvailable(api::Version {
                store_path: "new_path".to_owned(),
                substitutor: "sub".to_owned(),
                public_key: "p_key".to_owned(),
            }))
        );
    }
}
