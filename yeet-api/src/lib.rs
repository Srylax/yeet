//! API for yeet

use chrono::Duration;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[allow(clippy::exhaustive_enums)]
/// Represents a Version Status
pub enum VersionStatus {
    /// The version is up-to-date - no action required
    UpToDate,
    /// A new version is available - fetch and switch
    NewVersionAvailable(Version),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[allow(clippy::exhaustive_structs)]
/// Represents a Version
/// Each Version can have its own nix cache
pub struct Version {
    /// The store path to fetch from the nix cache
    pub store_path: String,
    /// The substitutor (nix cache) to fetch the store path from
    pub substitutor: String,
    /// The public key the cache uses to sign the store path
    pub public_key: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[allow(clippy::exhaustive_structs)]
/// Represents a Host Update
pub struct HostUpdate {
    /// The hostname of the host to update
    pub hostname: String,
    /// The store path to fetch
    pub store_path: String,
}
#[derive(Serialize, Deserialize, Debug)]
#[allow(clippy::exhaustive_structs)]
/// Represents a Host Update Request
/// The Agent uses the substitutor to fetch the update via nix
pub struct HostUpdateRequest {
    /// The hosts to update
    pub hosts: Vec<HostUpdate>,
    /// The substitutor the agent should use to fetch the update
    pub substitutor: String,
    /// The public key the agent should use to verify the update
    pub public_key: String,
}
#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
#[allow(clippy::exhaustive_structs)]
/// Represents a Token Request
pub struct TokenRequest {
    /// The capabilities the token should have
    pub capabilities: Vec<Capability>,
    #[serde_as(as = "serde_with::DurationSeconds<i64>")]
    /// The duration the token should be valid
    pub exp: Duration,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
/// Represents JWT aud
/// Capabilities can be combined
pub enum Capability {
    /// Used by yeet-agent to check for update
    SystemCheck {
        /// System Check is limited to a single Host
        hostname: String,
    },
    /// Create an Update for a host
    Update,
    /// Register a new host
    Register,
    /// Create new Tokens
    Token {
        /// Allows to with the following capability
        capabilities: Vec<Capability>,
    },
}

impl Capability {
    #[inline]
    #[must_use]
    /// Get the inner capabilities of a `Capability::Token`
    pub fn token(&self) -> Option<&Vec<Capability>> {
        match *self {
            Capability::Token { ref capabilities } => Some(capabilities),
            _ => None,
        }
    }

    #[inline]
    #[must_use]
    /// Create a token with all possible capabilities except `Capability::SystemCheck`
    pub fn all(hosts: Vec<String>) -> Vec<Capability> {
        let host_cap = hosts
            .into_iter()
            .map(|host| Capability::SystemCheck { hostname: host })
            .chain(vec![Capability::Register, Capability::Update])
            .collect();
        vec![
            Capability::Token {
                capabilities: host_cap,
            },
            Capability::Register,
            Capability::Update,
        ]
    }
}
