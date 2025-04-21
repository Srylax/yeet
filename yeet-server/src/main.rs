//! Yeet that Config

use crate::routes::register::register_host;
use crate::routes::system_check::system_check;
use crate::routes::update::update_hosts;
use axum::routing::post;
use axum::Router;
use ed25519_dalek::VerifyingKey;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json_any_key::any_key_map;
use ssh_key::PublicKey;
use std::collections::{HashMap, HashSet};
use std::fs::{rename, File, OpenOptions};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::os::unix::prelude::FileExt as _;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::TcpListener;
use tokio::time::interval;
use yeet_api::{StorePath, VersionStatus};

mod error;
mod routes {
    pub mod register;
    pub mod system_check;
    pub mod update;
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Default)]
struct AppState {
    build_machines: HashSet<VerifyingKey>,
    #[serde(with = "any_key_map")]
    hosts: HashMap<VerifyingKey, Host>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Default)]
struct Host {
    #[serde(skip_serializing, skip_deserializing)]
    last_ping: Option<Instant>,
    status: VersionStatus,
    store_path: StorePath,
}

#[tokio::main]
#[expect(
    clippy::expect_used,
    clippy::print_stdout,
    reason = "allow in server main"
)]
async fn main() {
    let mut state = File::open("state.json")
        .map(serde_json::from_reader)
        .unwrap_or(Ok(AppState::default()))
        .unwrap_or_else(|_err| {
            println!("Could not parse state.json. Moving old state.json to state.json.old");
            rename("state.json", "state.json.old").expect("Could not move unreadable config");
            AppState::default()
        });

    let key = PublicKey::read_openssh_file(Path::new("/etc/ssh/ssh_host_ed25519_key.pub"))
        .expect("Signing key found but not valid");

    let key = VerifyingKey::from_bytes(&key.key_data().ed25519().expect("Not an Ed25519 key").0)
        .expect("Found an Ed25519 key but could not parse it ");

    state.build_machines.insert(key);

    let state = Arc::new(RwLock::new(state));
    {
        let state = Arc::clone(&state);
        tokio::spawn(async move { save_state(&state).await });
    };

    let listener = TcpListener::bind("localhost:3000")
        .await
        .expect("Could not bind to port");
    axum::serve(listener, routes(state))
        .await
        .expect("Could not start axum");
}

fn routes(state: Arc<RwLock<AppState>>) -> Router {
    Router::new()
        .route("/system/check", post(system_check))
        .route("/system/register", post(register_host))
        .route("/system/update", post(update_hosts))
        .with_state(state)
}

#[expect(
    clippy::expect_used,
    clippy::infinite_loop,
    reason = "allow in server main"
)]
async fn save_state(state: &Arc<RwLock<AppState>>) {
    let mut interval = interval(Duration::from_millis(500));
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open("state.json")
        .expect("Could not open state.json");

    let mut hash = 0;

    loop {
        interval.tick().await;
        let state = state.read();
        let data = serde_json::to_vec_pretty(&*state).expect("Could not serialize state");
        let mut hasher = DefaultHasher::new();
        data.hash(&mut hasher);

        if hash != hasher.finish() {
            hash = hasher.finish();
            file.set_len(0).expect("Could not truncate file");
            file.write_all_at(&data, 0)
                .expect("Could not write to file");
        }
    }
}

#[cfg(test)]
use axum_test::TestServer;

#[cfg(test)]
fn test_server(state: AppState) -> (TestServer, Arc<RwLock<AppState>>) {
    let app_state = Arc::new(RwLock::new(state));
    let app_state_copy = Arc::clone(&app_state);
    let app = routes(app_state);
    let server = TestServer::builder()
        .expect_success_by_default()
        .mock_transport()
        .build(app)
        .expect("Could not build TestServer");
    (server, app_state_copy)
}
