//! # Yeet Agent

use std::fs::{read_link, File, OpenOptions};
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::Command;
use std::str;
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
// TODO: CLAP
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
// TODO: CLAP
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
            .send()?;
        check
            .headers()
            .get("X-Auth-Token")
            .ok_or(anyhow!("No Token provided"))?
            .to_str()?
            .clone_into(&mut config.token);
        save_config(&config)?;
        let check = check.error_for_status()?;
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

fn trusted_public_keys() -> Result<Vec<String>> {
    let file = File::open("/etc/nix/nix.conf")?;
    Ok(BufReader::new(file)
        .lines()
        .map_while(Result::ok)
        .find(|line| line.starts_with("trusted-public-keys"))
        .unwrap_or(String::from(
            "cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY=",
        ))
        .split_whitespace()
        .skip(2)
        .map(str::to_string)
        .collect())
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
        .args(
            [
                vec![
                    "--realise",
                    &version.store_path,
                    "--option",
                    "extra-substituters",
                    &version.substitutor,
                    "--option",
                    "trusted-public-keys",
                    &version.public_key,
                ],
                trusted_public_keys()?.iter().map(String::as_str).collect(),
                vec!["--option", "narinfo-cache-negative-ttl", "0"],
            ]
            .concat(),
        )
        .output()?;
    if !download.status.success() {
        bail!("{}", String::from_utf8(download.stderr)?);
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn activate(version: &Version) -> Result<()> {
    let download = Command::new(format!("{}/activate", version.store_path)).output()?;
    if !download.status.success() {
        bail!("{}", String::from_utf8(download.stderr)?);
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn activate(version: &Version) -> Result<()> {
    let download = Command::new(format!(
        "{}/bin/switch-to-configuration",
        version.store_path
    ))
    .arg("switch")
    .output()?;
    if !download.status.success() {
        bail!("{}", String::from_utf8(download.stderr)?);
    }
    Ok(())
}
