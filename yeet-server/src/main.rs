//! Yeet that Config

// use crate::routes::register::register_host;
// use crate::routes::system_check::system_check;
// use crate::routes::update::update_hosts;
use axum::Router;
use axum::routing::{get, post};
use ed25519_dalek::VerifyingKey;
use jiff::Zoned;
use parking_lot::RwLock;
use routes::status;
use serde::{Deserialize, Serialize};
use serde_json_any_key::any_key_map;
use std::collections::{HashMap, HashSet};
use std::fs::{File, OpenOptions, rename};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::os::unix::prelude::FileExt as _;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::time::interval;
use yeet_api::{StorePath, VersionStatus};

mod error;
mod httpsig;
mod routes {
    pub mod register;
    pub mod status;
    pub mod system_check;
    pub mod update;
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Default)]
struct AppState {
    admin_credentials: HashSet<VerifyingKey>,
    build_machines_credentials: HashSet<VerifyingKey>,
    #[serde(with = "any_key_map")]
    hosts: HashMap<VerifyingKey, Host>,
    keys: HashMap<String, VerifyingKey>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
struct Host {
    last_ping: Option<Zoned>,
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
        // .route("/system/check", post(system_check))
        // .route("/system/register", post(register_host))
        // .route("/system/update", post(update_hosts))
        .route("/status", get(status::status))
        .with_state(state)
}

#[expect(
    clippy::expect_used,
    clippy::infinite_loop,
    reason = "Save state as long as the server is running"
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
        .http_transport()
        .build(app)
        .expect("Could not build TestServer");
    (server, app_state_copy)
}
