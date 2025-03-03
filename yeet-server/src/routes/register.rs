use std::sync::Arc;

use crate::{AppState, Host};
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use ed25519_dalek::{Signature, VerifyingKey};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use yeet_api::VersionStatus::UpToDate;

#[derive(Serialize, Deserialize)]
pub struct HostRegister {
    key: VerifyingKey,
    signature: Signature,
    store_path: String,
}

pub async fn register_host(
    State(state): State<Arc<RwLock<AppState>>>,
    Json(HostRegister {
        store_path,
        key,
        signature,
    }): Json<HostRegister>,
) -> Result<StatusCode, (StatusCode, String)> {
    let mut state = state.write_arc();

    let valid_request = state.build_machines.iter().any(|key| {
        key.verify_strict(
            &[key.as_bytes(), store_path.as_bytes()].concat(),
            &signature,
        )
        .is_ok()
    });

    if !valid_request {
        return Err((
            StatusCode::UNAUTHORIZED,
            "Not a valid signature or not an authorized build machine".to_owned(),
        ));
    }

    if state.hosts.contains_key(&key) {
        return Err((
            StatusCode::BAD_REQUEST,
            "Host already registered".to_owned(),
        ));
    }

    let host = Host {
        store_path,
        status: UpToDate,
        last_ping: None,
    };

    state.hosts.insert(key, host);

    Ok(StatusCode::CREATED)
}

#[cfg(test)]
mod test_register {
    use ed25519_dalek::{SigningKey, ed25519::signature::SignerMut};

    use super::*;
    use crate::{Host, test_server};

    static SECRET_KEY_BYTES: [u8; 32] = [
        157, 97, 177, 157, 239, 253, 90, 96, 186, 132, 74, 244, 146, 236, 44, 196, 68, 73, 197,
        105, 123, 50, 105, 25, 112, 59, 172, 3, 28, 174, 127, 96,
    ];

    #[tokio::test]
    async fn test_register() {
        let mut key = SigningKey::from_bytes(&SECRET_KEY_BYTES);
        let mut state = AppState::default();
        state.build_machines.insert(key.verifying_key());

        let (server, state) = test_server(state);

        server
            .post("/system/register")
            .json(&HostRegister {
                key: key.verifying_key(),
                store_path: "my_store_path".to_owned(),
                signature: key
                    .sign(&[key.verifying_key().as_bytes(), "my_store_path".as_bytes()].concat()),
            })
            .await;
        let state = state.read_arc();

        assert_eq!(
            state.hosts[&key.verifying_key()],
            Host {
                store_path: "my_store_path".to_owned(),
                ..Default::default()
            }
        );
    }

    #[tokio::test]
    async fn test_already_registered() {
        let mut key = SigningKey::from_bytes(&SECRET_KEY_BYTES);
        let mut state = AppState::default();
        let host = Host {
            store_path: "my_store_path".to_owned(),
            ..Default::default()
        };
        state.hosts.insert(key.verifying_key(), host);

        let (mut server, _state) = test_server(state);
        server.expect_failure();

        server
            .post("/system/register")
            .json(&HostRegister {
                key: key.verifying_key(),
                store_path: "my_store_path".to_owned(),
                signature: key
                    .sign(&[key.verifying_key().as_bytes(), "my_store_path".as_bytes()].concat()),
            })
            .await;
    }
}
