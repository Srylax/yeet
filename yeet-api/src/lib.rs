use chrono::Duration;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum VersionStatus {
    UpToDate,
    NewVersionAvailable(Version),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Version {
    pub store_path: String,
    pub substitutor: String,
    pub public_key: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct HostUpdate {
    pub hostname: String,
    pub store_path: String,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct HostUpdateRequest {
    pub hosts: Vec<HostUpdate>,
    pub substitutor: String,
    pub public_key: String,
}
#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct TokenRequest {
    pub capabilities: Vec<Capability>,
    #[serde_as(as = "serde_with::DurationSeconds<i64>")]
    pub exp: Duration,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Capability {
    SystemCheck { hostname: String },
    Update,
    Register,
    Token,
}
impl From<Capability> for Vec<String> {
    fn from(value: Capability) -> Self {
        match value {
            Capability::SystemCheck { hostname } => vec![format!("/system/{hostname}/check")],
            Capability::Update => vec!["/system/update".to_string()],
            Capability::Register => vec!["/system/register".to_string()],
            Capability::Token => vec!["/token/new".to_string(), "/tokens".to_string()],
        }
    }
}
