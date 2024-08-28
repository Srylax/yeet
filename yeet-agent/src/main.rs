//! # Yeet Agent

use std::fs::{read_link, File, OpenOptions};
use std::path::PathBuf;
use std::process::Command;
use std::sync::LazyLock;
use std::thread::sleep;
use std::time::Duration;

use anyhow::{anyhow, bail, Result};
use notify_rust::Notification;
use reqwest::blocking::Client;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json::json;
use yeet_api::{Version, VersionStatus};

#[allow(clippy::expect_used)]
static CONFIG_FILE: LazyLock<PathBuf> = LazyLock::new(|| {
    let dir = dirs::state_dir()
        .or(dirs::home_dir().map(|home| home.join(".local/state/")))
        .map(|state| state.join("yeet/config.json"))
        .expect("Welp! You do not even have a Home directory. No idea where to store the config");
    if let Some(parent) = dir.parent() {
        std::fs::create_dir_all(parent).expect("Failed to create config directory");
    }
    dir
});

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
            .error_for_status()?;
        check
            .headers()
            .get("X-Auth-Token")
            .ok_or(anyhow!("No Token provided"))?
            .to_str()?
            .clone_into(&mut config.token);
        save_config(&config)?;
        match check.json::<VersionStatus>()? {
            VersionStatus::UpToDate => {}
            VersionStatus::NewVersionAvailable(version) => {
                update(&version)?;
            }
        }
        sleep(Duration::from_secs(config.sleep));
    }
}

fn get_config() -> Result<Config> {
    Ok(serde_json::from_reader(File::open(&*CONFIG_FILE)?)?)
}

fn save_config(config: &Config) -> Result<()> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&*CONFIG_FILE)?;
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
    Notification::new()
        .summary("System Update")
        .body("System has been updated successfully")
        .appname("Yeet")
        .show()?;
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
