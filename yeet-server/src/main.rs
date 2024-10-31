//! Yeet that Config

use crate::claim::Claims;
use crate::routes::register::register_host;
use crate::routes::system_check::system_check;
use crate::routes::token::{create_token, revoke_token};
use crate::routes::update::update_hosts;
use anyhow::Result;
use axum::routing::post;
use axum::Router;
use parking_lot::RwLock;
use std::fs::{rename, File, OpenOptions};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::os::unix::prelude::FileExt;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::time::interval;
use yeet_api::Capability;
use yeet_server::AppState;

mod claim;
mod error;
mod jwt;

mod routes {
    pub mod register;
    pub mod system_check;
    pub mod token;
    pub mod update;
}

#[tokio::main]
#[allow(clippy::expect_used, clippy::print_stdout)]
async fn main() {
    let state = File::open("state.json")
        .map(serde_json::from_reader)
        .unwrap_or(Ok(AppState::default()))
        .unwrap_or_else(|_err| {
            println!("Could not parse state.json. Moving old state.json to state.json.old");
            rename("state.json", "state.json.old").expect("Could not move unreadable config");
            AppState::default()
        });
    let host_cap: Vec<_> = state
        .hosts
        .keys()
        .map(|key| Capability::SystemCheck {
            hostname: key.clone(),
        })
        .chain(vec![Capability::Register, Capability::Update])
        .collect();
    let claims = Claims::new(
        vec![
            Capability::Token {
                capabilities: host_cap,
            },
            Capability::Register,
            Capability::Update,
        ],
        chrono::Duration::days(30),
    );
    println!(
        "{}",
        claims
            .encode(&state.jwt_secret)
            .expect("Could not encode claims")
    );

    let state = Arc::new(RwLock::new(state));
    {
        let state = Arc::clone(&state);
        tokio::spawn(async move { save_state(state).await });
    };
    let router = Router::new()
        .route("/system/:hostname/check", post(system_check))
        .route("/system/register", post(register_host))
        .route("/system/update", post(update_hosts))
        .route("/token/new", post(create_token))
        .route("/token/revoke", post(revoke_token))
        .with_state(state);

    let listener = TcpListener::bind("localhost:3000")
        .await
        .expect("Could not bind to port");
    axum::serve(listener, router)
        .await
        .expect("Could not start axum");
}

async fn save_state(state: Arc<RwLock<AppState>>) -> Result<()> {
    let mut interval = interval(Duration::from_millis(500));
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open("state.json")?;

    let mut hash = 0;

    loop {
        interval.tick().await;
        let state = state.read();
        let data = serde_json::to_vec_pretty(&*state)?;
        let mut hasher = DefaultHasher::new();
        data.hash(&mut hasher);

        if hash != hasher.finish() {
            hash = hasher.finish();
            file.set_len(0)?;
            file.write_all_at(&data, 0)?;
        }
    }
}
