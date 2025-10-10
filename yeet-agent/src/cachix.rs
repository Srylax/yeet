use std::ffi::OsStr;

use anyhow::{Ok, bail};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::process::Command;
use url::Url;

use crate::server::ErrorForJson as _;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[expect(clippy::exhaustive_structs)]
pub struct CachixInfo {
    pub github_username: String,
    pub is_public: bool,
    pub name: String,
    pub permission: String,
    pub preferred_compression_method: String,
    pub public_signing_keys: Vec<String>,
    pub uri: String,
}

pub async fn get_cachix_info<S: AsRef<str>>(cache: S) -> anyhow::Result<CachixInfo> {
    Client::new()
        .get(Url::parse("https://app.cachix.org/api/v1/cache/")?.join(cache.as_ref())?)
        .send()
        .await?
        .error_for_json::<CachixInfo>()
        .await
}

pub async fn push_paths<I, S, C>(closures: I, cache: C) -> anyhow::Result<()>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
    C: AsRef<str>,
{
    let exit = Command::new("cachix")
        .args(["push", cache.as_ref()])
        .args(closures)
        .status()
        .await?;
    if !exit.success() {
        bail!("Failed to push closures to cachix");
    }
    Ok(())
}
