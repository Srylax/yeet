//! API for yeet

use ed25519_dalek::VerifyingKey;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
/// Represents a Host Update
#[expect(
    clippy::exhaustive_structs,
    reason = "API Structs should be breaking change"
)]
pub struct HostUpdate {
    /// The ssh pub host key of the machine
    pub key: VerifyingKey,
    /// The store path to fetch
    pub store_path: String,
}
#[derive(Serialize, Deserialize, Debug)]
/// Represents a Host Update Request
/// The Agent uses the substitutor to fetch the update via nix
#[expect(
    clippy::exhaustive_structs,
    reason = "API Structs should be breaking change"
)]
pub struct HostUpdateRequest {
    /// The hosts to update
    pub hosts: Vec<HostUpdate>,
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
    pub store_path: String,
    /// The substitutor (nix cache) to fetch the store path from
    pub substitutor: String,
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
