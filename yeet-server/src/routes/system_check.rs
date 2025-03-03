use std::sync::Arc;
use std::time::Instant;

use crate::AppState;
use crate::error::WithStatusCode as _;
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use parking_lot::RwLock;
use yeet_api::{
    VersionRequest,
    VersionStatus::{self, NewVersionAvailable, UpToDate},
};

pub async fn system_check(
    State(state): State<Arc<RwLock<AppState>>>,
    Json(VersionRequest {
        key,
        store_path,
        signature,
    }): Json<VersionRequest>,
) -> Result<Json<VersionStatus>, (StatusCode, String)> {
    let mut state = state.write_arc();

    let Some(host) = state.hosts.get_mut(&key) else {
        return Err((StatusCode::NOT_FOUND, "Host not found".to_owned()));
    };

    key.verify_strict(store_path.as_bytes(), &signature)
        .with_code(StatusCode::FORBIDDEN)?;

    // If the client did the update
    if let NewVersionAvailable(ref next_version) = host.status {
        if next_version.store_path == store_path {
            host.store_path.clone_from(&store_path);
            host.status = UpToDate;
        }
    }

    // Version mismatch
    if host.store_path != store_path {
        return Err((
            StatusCode::BAD_REQUEST,
            "Current registered version does not match the provided version.\
                If you think this is a mistake, please update the version."
                .to_owned(),
        ));
    }

    host.last_ping = Some(Instant::now());

    Ok(Json(host.status.clone()))
}

#[cfg(test)]
mod test_system_check {
    use ed25519_dalek::{
        SigningKey,
        ed25519::signature::{Keypair, SignerMut},
    };
    use yeet_api::Version;

    use super::*;
    use crate::{Host, test_server};

    static SECRET_KEY_BYTES: [u8; 32] = [
        157, 97, 177, 157, 239, 253, 90, 96, 186, 132, 74, 244, 146, 236, 44, 196, 68, 73, 197,
        105, 123, 50, 105, 25, 112, 59, 172, 3, 28, 174, 127, 96,
    ];

    #[tokio::test]
    async fn test_no_update() {
        let mut key = SigningKey::from_bytes(&SECRET_KEY_BYTES);
        let host = Host {
            store_path: "example_store_path".to_owned(),
            ..Default::default()
        };
        let mut state = AppState::default();
        state.hosts.insert(key.verifying_key(), host);

        let (server, _state) = test_server(state);

        let response = server
            .post("/system/check")
            .json(&VersionRequest {
                key: key.verifying_key(),
                store_path: "example_store_path".to_owned(),
                signature: key.sign("example_store_path".as_bytes()),
            })
            .await
            .json::<VersionStatus>();
        assert_eq!(response, VersionStatus::UpToDate);
    }

    #[tokio::test]
    async fn test_with_update() {
        let mut key = SigningKey::from_bytes(&SECRET_KEY_BYTES);
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

        let (server, _state) = test_server(state);

        let response = server
            .post("/system/check")
            .json(&VersionRequest {
                key: key.verifying_key(),
                store_path: "example_store_path".to_owned(),
                signature: key.sign("example_store_path".as_bytes()),
            })
            .await
            .json::<VersionStatus>();
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
