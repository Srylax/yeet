//! # Yeet Agent
use std::fs::{read_link, File, OpenOptions};
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;

use anyhow::{anyhow, bail, Result};
use reqwest::blocking::Client;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json::json;
use yeet_api::{SystemCheck, Version, VersionStatus};

#[derive(Serialize, Deserialize)]
struct Config {
    url: Url,
    token: String,
    hostname: String,
    sleep: u64,
}

fn main() -> Result<()> {
    let mut config = get_config()?;
    let check_url = config
        .url
        .join(&format!("system/{}/check", config.hostname))?;
    loop {
        let store_path = json! ({
            "store_path": get_active_version()?,
        });
        let check = Client::new()
            .post(check_url.as_str())
            .bearer_auth(&config.token)
            .json(&store_path)
            .send()?
            .error_for_status()?
            .json::<SystemCheck>()?;
        config.token = check.token;
        save_config(&config)?;
        match check.status {
            VersionStatus::UpToDate => {}
            VersionStatus::NewVersionAvailable(version) => {
                update(&version)?;
            }
        }
        sleep(Duration::from_secs(config.sleep));
    }
}

fn get_config() -> Result<Config> {
    let config = dirs::state_dir()
        .or(dirs::data_dir())
        .ok_or(anyhow!("No Cache dir"))?
        .join("yeet-agent/config.json");

    Ok(serde_json::from_reader(File::open(config)?)?)
}

fn save_config(config: &Config) -> Result<()> {
    let config_path = dirs::cache_dir()
        .ok_or(anyhow!("No Cache dir"))?
        .join("yeet-agent/config.json");
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(config_path)?;
    Ok(serde_json::to_writer_pretty(file, config)?)
}

fn get_active_version() -> Result<String> {
    Ok(read_link("/run/current-system")?
        .to_string_lossy()
        .to_string())
}

fn update(version: &Version) -> Result<()> {
    download(version)?;
    activate(version)?;
    Ok(())
}

fn download(version: &Version) -> Result<()> {
    let download = Command::new("nix-store")
        .args([
            "-r",
            &version.store_path,
            "--option",
            "extra-substituters",
            &version.substitutor,
            "--option",
            "trusted-public-keys",
            &version.public_key,
            "--option",
            "narinfo-cache-negative-ttl",
            "0",
        ])
        .output()?;
    if !download.status.success() {
        bail!("{}", String::from_utf8(download.stderr)?);
    }
    Ok(())
}
fn activate(version: &Version) -> Result<()> {
    let download = Command::new(format!("{}/activate", version.store_path)).output()?;
    if !download.status.success() {
        bail!("{}", String::from_utf8(download.stderr)?);
    }
    Ok(())
}
