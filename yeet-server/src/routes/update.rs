use std::sync::Arc;

use crate::claim::Claims;
use crate::error::YeetError;
use crate::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use parking_lot::RwLock;
use yeet_api::VersionStatus::NewVersionAvailable;
use yeet_api::{Capability, HostUpdateRequest, Version};

pub async fn update_hosts(
    State(state): State<Arc<RwLock<AppState>>>,
    claim: Claims,
    Json(req): Json<HostUpdateRequest>,
) -> Result<StatusCode, YeetError> {
    claim.require(Capability::Update)?;
    let mut state = state.write();

    let unknown_hosts: Vec<&str> = req
        .hosts
        .iter()
        .filter(|host| !state.hosts.contains_key(&host.hostname))
        .map(|host| host.hostname.as_str())
        .collect();

    if let Some(host) = unknown_hosts.first() {
        return Err(YeetError::HostNotFound((**host).to_string()));
    }

    for host_update in req.hosts {
        let Some(host) = state.hosts.get_mut(&host_update.hostname) else {
            return Err(YeetError::HostNotFound(host_update.hostname));
        };
        let version = Version {
            store_path: host_update.store_path,
            substitutor: req.substitutor.clone(),
            public_key: req.public_key.clone(),
        };
        host.status = NewVersionAvailable(version);
    }

    Ok(StatusCode::CREATED)
}

#[cfg(test)]
mod test_update {
    use super::*;
    use crate::{test_access, test_server};
    use axum::response::IntoResponse;
    use yeet_api::HostUpdate;
    use yeet_api::VersionStatus::UpToDate;
    use yeet_server::{Host, Jti};

    test_access!(update_host, Capability::Update);

    async fn update_host(capabilities: Vec<Capability>) {
        let mut app_state = AppState::default();
        app_state.hosts.insert(
            "my-host".to_owned(),
            Host {
                hostname: "my-host".to_owned(),
                store_path: "/nix/store/abc".to_owned(),
                status: UpToDate,
                jti: Jti::Blocked,
                last_ping: None,
            },
        );
        let (server, state) = test_server(app_state, capabilities);

        server
            .post("/system/update")
            .json(&HostUpdateRequest {
                hosts: vec![HostUpdate {
                    hostname: "my-host".to_owned(),
                    store_path: "/nix/store/abc".to_owned(),
                }],
                substitutor: "my-substitutor".to_owned(),
                public_key: "my-key".to_owned(),
            })
            .await
            .assert_status(StatusCode::CREATED);

        let state = state.read_arc();

        assert_eq!(
            state.hosts.get("my-host"),
            Some(&Host {
                hostname: "my-host".to_owned(),
                store_path: "/nix/store/abc".to_owned(),
                status: NewVersionAvailable(Version {
                    store_path: "/nix/store/abc".to_owned(),
                    substitutor: "my-substitutor".to_owned(),
                    public_key: "my-key".to_owned(),
                }),
                jti: Jti::Blocked,
                last_ping: None,
            })
        );
    }

    #[tokio::test]
    async fn host_not_found() {
        let (mut server, _state) = test_server(AppState::default(), vec![Capability::Update]);

        server.expect_failure();

        let response = server
            .post("/system/update")
            .json(&HostUpdateRequest {
                hosts: vec![HostUpdate {
                    hostname: "my-host".to_owned(),
                    store_path: "/nix/store/abc".to_owned(),
                }],
                substitutor: "my-substitutor".to_owned(),
                public_key: "my-key".to_owned(),
            })
            .await;

        response.assert_status(
            YeetError::HostNotFound(String::new())
                .into_response()
                .status(),
        );
        response.assert_text(YeetError::HostNotFound("my-host".to_owned()).to_string());
    }
}
