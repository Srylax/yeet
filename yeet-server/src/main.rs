//! Yeet that Config
use std::collections::{HashMap, HashSet};
use std::fs::{rename, File, OpenOptions};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::Write;
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::vec;

use anyhow::Result;
use axum::routing::post;
use axum::Router;
use parking_lot::RwLock;
use rand::prelude::*;
use rand_hc::Hc128Rng;
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tokio::time::interval;
use uuid::Uuid;

use yeet_api::{Capability, VersionStatus};

use crate::jwt::create_jwt;
use crate::routes::register::register_host;
use crate::routes::system_check::system_check;
use crate::routes::token::{create_token, revoke_token};
use crate::routes::update::update_hosts;

mod routes {
    pub mod register;
    pub mod system_check;
    pub mod token;
    pub mod update;
}

mod jwt;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
struct Host {
    hostname: String,
    store_path: String,
    status: VersionStatus,
    jti: Jti,
    #[serde(skip_serializing, skip_deserializing)]
    last_ping: Option<Instant>,
}
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
enum Jti {
    Jti(Uuid),
    Blocked,
}
#[derive(Serialize, Deserialize, PartialEq, Eq)]
struct AppState {
    hosts: HashMap<String, Host>,
    jwt_secret: [u8; 32],
    jti_blacklist: HashSet<Uuid>,
}

impl Default for AppState {
    fn default() -> Self {
        let mut secret = [0; 32];
        Hc128Rng::from_entropy().fill_bytes(&mut secret);
        Self {
            hosts: HashMap::default(),
            jwt_secret: secret,
            jti_blacklist: HashSet::default(),
        }
    }
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

    let (jwt, _jti) = create_jwt(
        vec![Capability::Token, Capability::Register, Capability::Update],
        chrono::Duration::days(30),
        &state.jwt_secret,
    )
    .expect("Could not Create token");
    println!("{jwt}");

    let state = Arc::new(RwLock::new(state));
    {
        let state = Arc::clone(&state);
        tokio::spawn(async move { save_state(state).await });
    };

    let app = Router::new()
        .route("/system/:hostname/check", post(system_check))
        .route("/system/register", post(register_host))
        .route("/system/update", post(update_hosts))
        .route("/token/new", post(create_token))
        .route("/token/revoke", post(revoke_token))
        .with_state(state);
    let listener = TcpListener::bind("localhost:3000")
        .await
        .expect("Could not bind to port");
    axum::serve(listener, app)
        .await
        .expect("Could not start axum");
}

async fn save_state(state: Arc<RwLock<AppState>>) -> Result<()> {
    let mut interval = interval(Duration::from_millis(500));
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open("state.json")?;

    // file.set_len(1_000_000)?;
    // let mut mmap = unsafe { MmapMut::map_mut(&file)? };

    let mut hash = 0;

    loop {
        interval.tick().await;
        let state = state.read();
        let data = serde_json::to_vec_pretty(&*state)?;
        let mut hasher = DefaultHasher::new();
        data.hash(&mut hasher);

        if hash != hasher.finish() {
            hash = hasher.finish();
            file.write_all(&data)?;
            // file.set_len(data.len() as u64)?;
            // (&mut mmap[..]).write_all(&data)?;
        }
    }
}
