use api::Host;
use axum::http::StatusCode;
use axum_thiserror::ErrorStatus;
use httpsig_hyper::prelude::{AlgorithmName, SecretKey, SigningKey as _};
use serde_json_any_key::any_key_map;
use std::collections::{HashMap, HashSet, hash_map};
use thiserror::Error;

use ed25519_dalek::VerifyingKey;
use serde::{Deserialize, Serialize};

#[derive(Error, Debug, ErrorStatus)]
pub enum StateError {
    #[error("Key is authenticated but associated host not found")]
    #[status(StatusCode::FORBIDDEN)]
    HostNotFound,

    #[error("Hosts not found: {0:?}")]
    #[status(StatusCode::BAD_REQUEST)]
    MultipleHostsNotFound(Vec<String>),

    #[error("The request is authenticated but you lack admin credentials")]
    #[status(StatusCode::FORBIDDEN)]
    AuthMissingAdmin,

    #[error("The request is authenticated but you lack build credentials")]
    #[status(StatusCode::FORBIDDEN)]
    AuthMissingBuild,
}

type Result<T> = core::result::Result<T, StateError>;

#[derive(Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct AppState {
    admin_credentials: HashSet<VerifyingKey>,
    build_machines_credentials: HashSet<VerifyingKey>,
    // key -> Hosts
    #[serde(with = "any_key_map")]
    hosts: HashMap<VerifyingKey, api::Host>,
    //  keyid -> Key for httpsig
    keyids: HashMap<String, VerifyingKey>,
    // Maps name to the public key
    key_by_name: HashMap<String, VerifyingKey>,
    // 6 digit number -> unverified pub key
    // verification_attempt: HashMap<u32, VerifyingKey>,

    // A list of hosts ready for registration
    pre_register_host: HashMap<String, api::ProvisionState>,
}

impl AppState {
    pub fn register_host(&mut self, host: Host, key: VerifyingKey) {
        let signing_key = SecretKey::from_bytes(AlgorithmName::Ed25519, key.as_bytes())
            .expect("Verifying key already is validated");
        self.key_by_name.insert(host.name.clone(), key);
        self.hosts.insert(key, host);

        self.keyids.insert(signing_key.key_id(), key);
    }

    pub fn pre_register_host(
        &mut self,
        name: String,
        state: api::ProvisionState,
    ) -> Option<api::ProvisionState> {
        self.pre_register_host.insert(name, state)
    }

    /// This is the "ping" command every client should send in a specific interval.
    /// Based on the provision state and the last known version this function takes different parts
    ///
    /// host.latest_store_path()
    /// host.provision_state
    ///
    /// host.last_ping = Zoned::now
    ///
    /// ====== if host.provision_state == Provisioned
    ///
    /// # this is the path when the client did the update
    /// # if "host version is behind but sent version and provision version match"
    /// if host.latest_store_path() != store_path and store_path == host.provision_state
    ///     host.version_history.insert(store_path, Zoned::now)
    ///     -> Nothing
    ///
    /// # this is the path when the client gets notified of an update
    /// # if "host AND sent version is behind but server version is different"
    /// but because there could be a race condition e.g. Update1(v1) -> client does update1 in this time server gets Update2
    /// therefore we need to check if sent version is behind server version
    /// if host.latest_store_path() == store_path && host.latest_store_path() != host.provision_state
    ///     -> SwitchTo(host.provision_state)
    ///
    /// # Lastly if all 3 are the same do nothing
    /// -> Nothing
    ///
    /// ====== if host.provision_state == Detached
    ///
    /// # check if store_path is the same as host.latest_store_path() if not the update host.latest_store_path()
    /// -> Detach
    ///
    /// ====== if host.provision_state == NotSet
    /// -> Nothing
    pub fn system_check(
        &mut self,
        store_path: String,
        key: &VerifyingKey,
    ) -> Result<api::AgentAction> {
        let host = self.hosts.get_mut(key).ok_or(StateError::HostNotFound)?;

        let action = match host.provision_state {
            api::ProvisionState::NotSet => api::AgentAction::Nothing,
            api::ProvisionState::Detached => {
                if host.latest_store_path() != &store_path {
                    host.update_store_path(store_path);
                }
                api::AgentAction::Detach
            }
            api::ProvisionState::Provisioned(ref version) => {
                if &store_path != &version.store_path {
                    // TODO: we do not see if we updated fast in succession we only se the latest
                    api::AgentAction::SwitchTo(version.clone())
                } else if (&version.store_path == &store_path)
                    && &store_path != host.latest_store_path()
                {
                    host.update_store_path(store_path);
                    api::AgentAction::Nothing
                } else {
                    api::AgentAction::Nothing
                }
            }
        };

        host.ping();

        Ok(action)
    }

    /// Endpoint to set a new version for a host.
    /// The whole request needs to be signed by a build machine.
    /// The update consist of a simple `name` -> `version` and a `substitutor` which is where the agent should get its update
    /// This means that for each origin e.g. cachix, you need to call update seperately
    pub fn update_hosts(
        &mut self,
        hosts: HashMap<String, api::StorePath>,
        public_key: String,
        substitutor: String,
    ) -> Result<()> {
        let unknown_hosts = hosts
            .iter()
            .filter(|(name, _)| !self.key_by_name.contains_key(*name))
            .map(|(name, _)| name)
            .cloned()
            .collect::<Vec<_>>();

        if !unknown_hosts.is_empty() {
            return Err(StateError::MultipleHostsNotFound(unknown_hosts));
        }

        for (name, store_path) in hosts {
            let host = self
                .host_by_name_mut(&name)
                .expect("Race condition because we checked above - maybe change this TOCTOU");
            let version = api::Version {
                store_path: store_path.clone(),
                substitutor: substitutor.clone(),
                public_key: public_key.clone(),
            };

            host.push_update(version);
        }

        Ok(())
    }

    pub fn auth_build(&self, key: &VerifyingKey) -> Result<()> {
        if self.admin_credentials.contains(key) || self.build_machines_credentials.contains(key) {
            Ok(())
        } else {
            Err(StateError::AuthMissingBuild)
        }
    }

    pub fn add_build(&mut self, key: VerifyingKey) {
        let signing_key = SecretKey::from_bytes(AlgorithmName::Ed25519, key.as_bytes())
            .expect("Verifying key already is validated");
        self.build_machines_credentials.insert(key);
        self.keyids.insert(signing_key.key_id(), key);
    }

    pub fn auth_admin(&self, key: &VerifyingKey) -> Result<()> {
        if self.admin_credentials.contains(key) {
            Ok(())
        } else {
            Err(StateError::AuthMissingAdmin)
        }
    }

    pub fn add_admin(&mut self, key: VerifyingKey) {
        let signing_key = SecretKey::from_bytes(AlgorithmName::Ed25519, key.as_bytes())
            .expect("Verifying key already is validated");
        self.admin_credentials.insert(key);
        self.keyids.insert(signing_key.key_id(), key);
    }

    pub fn contains_host(&self, host: &String) -> bool {
        self.key_by_name.contains_key(host)
    }

    pub(crate) fn hosts(&self) -> hash_map::Values<'_, VerifyingKey, api::Host> {
        self.hosts.values()
    }

    fn host_by_name_mut(&mut self, host: &String) -> Option<&mut Host> {
        self.key_by_name
            .get(host)
            .and_then(|key| self.hosts.get_mut(key))
    }

    pub fn add_admin_credential(&mut self, key: VerifyingKey) {
        let signing_key = httpsig_hyper::prelude::SecretKey::from_bytes(
            httpsig_hyper::prelude::AlgorithmName::Ed25519,
            key.as_bytes(),
        )
        .expect("Could not convert ED25519 key to httpsig key - wtf");
        self.admin_credentials.insert(key);
        self.keyids.insert(signing_key.key_id(), key);
    }
    pub fn has_admin_credential(&self) -> bool {
        !self.admin_credentials.is_empty()
    }
    pub fn get_key_by_id<S: AsRef<str>>(&self, keyid: S) -> Option<VerifyingKey> {
        self.keyids.get(keyid.as_ref()).copied()
    }
}

#[cfg(test)]
mod test_state {
    use crate::state::AppState;
}
