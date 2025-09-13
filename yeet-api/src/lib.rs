//! API for yeet

use chrono::{DateTime, Utc};
use serde_json_any_key::any_key_map;
use std::collections::{HashMap, HashSet};

use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};

pub mod status;

pub type StorePath = String;

#[derive(Serialize, Deserialize, Debug, Clone)]
/// Represents a Host Update Request
/// The Agent uses the substitutor to fetch the update via nix
#[expect(
    clippy::exhaustive_structs,
    reason = "API Structs should be breaking change"
)]
pub struct HostUpdateRequest {
    /// The hosts to update
    #[serde(with = "any_key_map")]
    pub hosts: HashMap<VerifyingKey, StorePath>,
    /// The public key the agent should use to verify the update
    pub public_key: String,
    /// The substitutor the agent should use to fetch the update
    pub substitutor: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
/// Represents a Version
/// Each Version can have its own nix cache
#[expect(
    clippy::exhaustive_structs,
    reason = "API Structs should be breaking change"
)]
pub struct Version {
    /// The public key the cache uses to sign the store path
    pub public_key: String,
    /// The store path to fetch from the nix cache
    pub store_path: StorePath,
    /// The substitutor (nix cache) to fetch the store path from
    pub substitutor: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
pub struct Host {
    last_ping: Option<DateTime<Utc>>,
    status: VersionStatus,
    store_path: StorePath,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[expect(
    clippy::exhaustive_structs,
    reason = "API Structs should be breaking change"
)]
pub struct VersionRequest {
    pub key: VerifyingKey,
    pub signature: Signature,
    pub store_path: StorePath,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
/// Represents a Version Status
#[expect(
    clippy::exhaustive_enums,
    reason = "API Structs should be breaking change"
)]
pub enum VersionStatus {
    /// A new version is available - fetch and switch
    NewVersionAvailable(Version),
    /// The version is up-to-date - no action required
    UpToDate,
}

impl Default for VersionStatus {
    #[inline]
    fn default() -> Self {
        Self::UpToDate
    }
}
