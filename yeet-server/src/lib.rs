//! Yeet that Config
use rand::{RngCore, SeedableRng};
use rand_hc::Hc128Rng;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::Instant;
use uuid::Uuid;
use yeet_api::VersionStatus;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
#[allow(clippy::exhaustive_structs, missing_docs)]
pub struct Host {
    pub hostname: String,
    pub store_path: String,
    pub status: VersionStatus,
    pub jti: Jti,
    #[serde(skip_serializing, skip_deserializing)]
    pub last_ping: Option<Instant>,
}
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
#[non_exhaustive]
#[allow(missing_docs)]
pub enum Jti {
    Jti(Uuid),
    Blocked,
}
#[derive(Serialize, Deserialize, PartialEq, Eq)]
#[allow(clippy::exhaustive_structs, missing_docs)]
pub struct AppState {
    pub hosts: HashMap<String, Host>,
    pub jwt_secret: [u8; 32],
    pub jti_blacklist: HashSet<Uuid>,
}

impl Default for AppState {
    #[inline]
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
