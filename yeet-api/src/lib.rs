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
    /// The hosts to update identified by their name
    pub hosts: HashMap<String, StorePath>,
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

#[derive(Serialize, Deserialize, Default)]
pub struct RegisterHost {
    pub provision_state: ProvisionState,
    pub name: String,
}

// values that are needed at start:
// - name
// possible states: (we need to track 3 different states) key state, server state, client state
//
// key: nothing | set
// server: nothing | version | detached
// client: nothing | version
//
//
// client.version requires key.set
// key.set requires client.version
//
//
// (key.nothing, client.nothing, server.nothing | server.version | server.detached)
// (key.set, client.version, server.nothing | server.version | server.detached)
//
// a struct with the following maps represets all possible states
//
// name -> key
// name -> server.state
// key -> client.state
//
// the requirements that a host (name) cannot have a client.state without a key is upheld
// Further there are no shenanigans between the states. A consumer now gets the client state and it is either there or not
//
// And now we can even remove the name because that is now just extra information
// We only need the following association key -> id
// but key is not available at the start so we require name and bind and id to it
// Id is a word with many prejudice so lets use handle
//
// key -> handle
// handle -> server.state
// key -> client.state
//
//
// new_key_type! { struct HostHandle; }
// Everything that a host should only ever have once provisioned is stored under a KeyHandle
// Things that are extra information and should always be accessible are under the HostHandle
// pub struct Hosts {
//     keyids: HashMap<String, VerifyingKey>, // These are registred keys used by httpsig - we need a hashmap because we have no way to store a handle - keyid is derived

//     keys: SlotMap<HostHandle, VerifyingKey>,
//     client_state: SecondaryMap<HostHandle, Vec<(StorePath, Zoned)>>, // Client should only have a state with a key
//     last_ping: SecondaryMap<HostHandle, Option<Zoned>>,
//     server_state: SecondaryMap<HostHandle, ServerState>,
//     names: SecondaryMap<HostHandle, String>,

//     unregistered: Vec<String>,
// }

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
pub struct Host {
    pub name: String,
    pub last_ping: Zoned,
    pub provision_state: ProvisionState,
    // Version with date when the update occured
    pub version_history: Vec<(StorePath, Zoned)>,
}

impl Host {
    pub fn latest_store_path(&self) -> &StorePath {
        &self
            .version_history
            .last()
            .expect("version_history cannot be empty")
            .0
    }
    pub fn update_store_path(&mut self, store_path: String) {
        self.version_history.push((store_path, Zoned::now()));
    }

    pub fn push_update(&mut self, version: Version) {
        if self.is_provisioned() {
            self.provision_state = ProvisionState::Provisioned(version);
        }
    }

    pub fn ping(&mut self) {
        self.last_ping = Zoned::now();
    }

    // pub fn provision_store_path(&self) -> Option<&String> {
    //     match self.provision_state {
    //         ProvisionState::NotSet | ProvisionState::Detached => None,
    //         ProvisionState::Provisioned(ref store_path) => Some(store_path),
    //     }
    // }

    pub fn is_detached(&self) -> bool {
        match self.provision_state {
            ProvisionState::NotSet | ProvisionState::Provisioned(_) => false,
            ProvisionState::Detached => true,
        }
    }

    pub fn is_provisioned(&self) -> bool {
        match self.provision_state {
            ProvisionState::Provisioned(_) => true,
            ProvisionState::NotSet | ProvisionState::Detached => false,
        }
    }
}

// State the Server wants the client to be in
#[expect(clippy::exhaustive_structs)]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum ProvisionState {
    NotSet,
    Detached,
    Provisioned(Version),
}

impl Default for ProvisionState {
    #[inline]
    fn default() -> Self {
        Self::NotSet
    }
}

// Action the server want the client to take
#[expect(clippy::exhaustive_structs)]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum AgentAction {
    Nothing,
    Detach,
    SwitchTo(Version),
}

impl Default for AgentAction {
    #[inline]
    fn default() -> Self {
        Self::Nothing
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[expect(clippy::exhaustive_structs)]
pub struct VersionRequest {
    pub store_path: StorePath,
}

#[inline]
pub fn hash(value: impl std::hash::Hash) -> u64 {
    ahash::RandomState::with_seeds(1, 2, 3, 4).hash_one(value)
}

#[inline]
pub fn hash_hex(value: impl std::hash::Hash) -> String {
    format!("{:x}", hash(value))
}
