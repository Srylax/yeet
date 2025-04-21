use std::sync::Arc;

use crate::error::WithStatusCode as _;
use crate::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use ed25519_dalek::Signature;
use parking_lot::RwLock;
use serde_json::Value;
use yeet_api::VersionStatus::NewVersionAvailable;
use yeet_api::{HostUpdateRequest, Version};

/// Endpoint to set a new version for a host.
/// The whole request needs to be signed by a build machine.
/// The update consist of a simple `key` -> `version` and a `substitutor` which is where the agent should get its update
/// This means that for each origin e.g. cachix, you need to call update seperately
pub async fn update_hosts(
    State(state): State<Arc<RwLock<AppState>>>,
    Json((req, signature)): Json<(Value, Signature)>,
) -> Result<StatusCode, (StatusCode, String)> {
    let mut state = state.write_arc();

    let valid_request = state.build_machines.iter().any(|key| {
        key.verify_strict(req.to_string().as_bytes(), &signature)
            .is_ok()
    });

    if !valid_request {
        return Err((
            StatusCode::UNAUTHORIZED,
            "Not a valid build machine signature".to_owned(),
        ));
    }

    let req: HostUpdateRequest = serde_json::from_value(req).with_code(StatusCode::BAD_REQUEST)?;

    let unknown_host = req
        .hosts
        .iter()
        .any(|(key, _)| !state.hosts.contains_key(key));

    if unknown_host {
        return Err((StatusCode::NOT_FOUND, "Host not found".to_owned()));
    }

    for (key, store_path) in req.hosts {
        let host = state
            .hosts
            .get_mut(&key)
            .ok_or((StatusCode::NOT_FOUND, "Host not found".to_owned()))?;
        let version = Version {
            store_path: store_path.clone(),
            substitutor: req.substitutor.clone(),
            public_key: req.public_key.clone(),
        };
        host.status = NewVersionAvailable(version);
    }

    Ok(StatusCode::CREATED)
}

#[cfg(test)]
mod test_update {
    use std::collections::HashMap;

    use ed25519_dalek::{ed25519::signature::SignerMut, SigningKey, VerifyingKey};
    use yeet_api::VersionStatus;

    use super::*;
    use crate::{test_server, Host};

    static SECRET_KEY_BYTES: [u8; 32] = [
        157, 97, 177, 157, 239, 253, 90, 96, 186, 132, 74, 244, 146, 236, 44, 196, 68, 73, 197,
        105, 123, 50, 105, 25, 112, 59, 172, 3, 28, 174, 127, 96,
    ];

    #[tokio::test]
    async fn test_update() {
        let mut key = SigningKey::from_bytes(&SECRET_KEY_BYTES);
        let host = Host::default();
        let mut state = AppState::default();
        state.hosts.insert(VerifyingKey::default(), host);
        state.build_machines.insert(key.verifying_key());

        let (server, state) = test_server(state);

        let request = HostUpdateRequest {
            hosts: HashMap::from([(VerifyingKey::default(), "new_path".to_owned())]),
            public_key: "p_key".to_owned(),
            substitutor: "sub".to_owned(),
        };

        server
            .post("/system/update")
            .json(&(
                request.clone(),
                key.sign(&serde_json::to_vec(&request).unwrap()),
            ))
            .await;
        let state = state.read_arc();

        assert_eq!(
            state.hosts[&VerifyingKey::default()].status,
            VersionStatus::NewVersionAvailable(Version {
                store_path: "new_path".to_owned(),
                substitutor: "sub".to_owned(),
                public_key: "p_key".to_owned(),
            })
        );
    }
}
