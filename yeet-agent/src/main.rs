//! # Yeet Agent

use std::fs::{read_link, File};
use std::io::{BufRead, BufReader};
use std::process::Command;
use std::str;
use std::thread::sleep;
use std::time::Duration;

use anyhow::{anyhow, bail, Result};
use clap::{arg, Args, Parser, Subcommand};
use keyring::Entry;
use notify_rust::Notification;
use reqwest::blocking::Client;
use reqwest::Url;
use serde_json::json;
use yeet_api::{Version, VersionStatus};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Yeet {
    /// Override the hostname, default is the hostname of the system
    #[arg(short, long)]
    name: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Store the Token in the keyring
    Auth {
        /// JWT Auth Token
        #[arg(short, long)]
        token: String,
    },
    /// Deploy the Yeet Agent
    Deploy(Deploy),
}
#[derive(Args)]
struct Deploy {
    /// Base URL of the Yeet Server
    #[arg(short, long)]
    url: Url,

    /// Seconds to wait between updates.
    /// Lower bound, may be higher between switching versions
    #[arg(short, long, default_value = "30")]
    sleep: u64,
}

fn main() -> Result<()> {
    let args = Yeet::parse();
    let deploy_args = match args.command {
        Commands::Auth { token } => {
            let keyring_entry = Entry::new_with_target("system", "yeet-agent", &args.name)?;
            keyring_entry.set_password(&token)?;
            return Ok(());
        }
        Commands::Deploy(deploy) => deploy,
    };
    let keyring_entry = Entry::new_with_target("system", "yeet-agent", &args.name)?;
    let check_url = deploy_args
        .url
        .join(&format!("system/{}/check", args.name))?;
    let mut token = keyring_entry.get_password()?;
    loop {
        let store_path = json! ({
            "store_path": get_active_version()?,
        });
        let check = Client::new()
            .post(check_url.as_str())
            .bearer_auth(&token)
            .json(&store_path)
            .send()?;
        check
            .headers()
            .get("x-auth-token")
            .ok_or(anyhow!("No Token provided"))?
            .to_str()?
            .clone_into(&mut token);
        keyring_entry.set_password(&token)?;
        let check = check.error_for_status()?;
        match check.json::<VersionStatus>()? {
            VersionStatus::UpToDate => {}
            VersionStatus::NewVersionAvailable(version) => {
                update(&version)?;
            }
        }
        sleep(Duration::from_secs(deploy_args.sleep));
    }
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
        .map(str::to_owned)
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
    let mut keys = trusted_public_keys()?;
    keys.push(version.public_key.clone());
    let download = Command::new("nix-store")
        .args(vec![
            "--realise",
            &version.store_path,
            "--option",
            "extra-substituters",
            &version.substitutor,
            "--option",
            "trusted-public-keys",
            &keys.join(" "),
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
