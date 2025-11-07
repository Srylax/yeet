//! Yeet that Config

use crate::routes::register::register_host;
use crate::routes::system_check::system_check;
use crate::routes::update::update_hosts;
use crate::routes::verify::{add_verification_attempt, is_host_verified, verify_attempt};
use crate::state::AppState;
use axum::Router;
use axum::routing::{get, post};
use ed25519_dalek::SigningKey;
use ed25519_dalek::pkcs8::EncodePrivateKey as _;
use ed25519_dalek::pkcs8::spki::der::pem::LineEnding;
use parking_lot::RwLock;
use routes::status;
use std::env;
use std::fs::{File, OpenOptions, rename};
use std::hash::{DefaultHasher, Hash as _, Hasher as _};
use std::os::unix::prelude::FileExt as _;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::time::interval; // TODO: is this enough or do we need to use rand_chacha?

mod error;
mod httpsig;
mod state;
mod routes {
    pub mod register;
    pub mod status;
    pub mod system_check;
    pub mod update;
    pub mod verify;
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

    // TODO: make this interactive if interactive shell found
    if !state.has_admin_credential() {
        println!("Creating new admin credentials");
        let key = SigningKey::generate(&mut rand_core::OsRng);
        key.write_pkcs8_pem_file("yeet-admin.pem", LineEnding::LF)
            .expect("Could not write the admin credential file");
        println!("Written to file `yeet-admin.pem`");
        state.add_admin_key(key.verifying_key());
    }

    let state = Arc::new(RwLock::new(state));
    {
        let state = Arc::clone(&state);
        tokio::spawn(async move { save_state(&state).await });
    };

    let port = env::var("YEET_PORT").unwrap_or("4337".to_owned());
    let host = env::var("YEET_HOST").unwrap_or("localhost".to_owned());

    let listener = TcpListener::bind(format!("{host}:{port}"))
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
        .route("/system/verify/accept", post(verify_attempt))
        .route("/system/verify", get(is_host_verified))
        .route("/system/verify", post(add_verification_attempt))
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
