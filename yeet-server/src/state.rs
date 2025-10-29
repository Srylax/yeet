use api::Host;
use httpsig_hyper::prelude::SigningKey as _;
use serde_json_any_key::any_key_map;
use std::collections::{HashMap, HashSet};

use ed25519_dalek::VerifyingKey;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct AppState {
    admin_credentials: HashSet<VerifyingKey>,
    build_machines_credentials: HashSet<VerifyingKey>,
    // name -> Hosts
    hosts: HashMap<String, api::Host>,
    // Maps the public keys to the host names
    #[serde(with = "any_key_map")]
    host_by_key: HashMap<VerifyingKey, String>,
    // 6 digit number -> unverified pub key
    // verification_attempt: HashMap<u32, VerifyingKey>,
    keys: HashMap<String, VerifyingKey>,
}

impl AppState {
    pub fn add_host(&mut self, host: Host) {}
    pub fn add_admin_credential(&mut self, key: VerifyingKey) {
        let signing_key = httpsig_hyper::prelude::SecretKey::from_bytes(
            httpsig_hyper::prelude::AlgorithmName::Ed25519,
            key.as_bytes(),
        )
        .expect("Could not convert ED25519 key to httpsig key - wtf");
        self.admin_credentials.insert(key);
        self.keys.insert(signing_key.key_id(), key);
    }
    pub fn has_admin_credential(&self) -> bool {
        !self.admin_credentials.is_empty()
    }
    pub fn get_key_by_id<S: AsRef<str>>(&self, keyid: S) -> Option<VerifyingKey> {
        self.keys.get(keyid.as_ref()).copied()
    }
}
