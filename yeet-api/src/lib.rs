//! API for yeet

use jiff::Zoned;
use serde_json_any_key::any_key_map;
use std::collections::{HashMap, HashSet};

use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};

pub mod httpsig;
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
    /// The hosts to update identified by their public key
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

#[derive(Serialize, Deserialize)]
pub struct RegisterHost {
    pub key: VerifyingKey,
    pub store_path: String,
    pub name: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
pub struct Host {
    pub name: Option<String>,
    pub key: VerifyingKey,
    pub last_ping: Option<Zoned>,
    pub status: HostState,
    pub store_path: StorePath,
    // pub version_history: Vec<HostState>,
}

// State that the host is currently in
#[expect(clippy::exhaustive_structs)]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum HostState {
    New,
    Detached, // Does not really do anything yet
    Provisioned(ProvisionState),
}

impl Default for HostState {
    #[inline]
    fn default() -> Self {
        Self::New
    }
}

#[expect(clippy::exhaustive_structs)]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum ProvisionState {
    UpToDate,
    NewVersionAvailable(Version),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[expect(clippy::exhaustive_structs)]
pub struct VersionRequest {
    pub store_path: StorePath,
}
