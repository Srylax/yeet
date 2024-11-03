//! # Yeet Agent

use std::fs::{read_link, read_to_string, File};
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::Command;
use std::str;
use std::thread::sleep;
use std::time::Duration;

use anyhow::{anyhow, bail, Result};
use clap::{arg, Parser};
use notify_rust::Notification;
use reqwest::blocking::Client;
use serde_json::json;
use url::Url;
use yeet_api::{Capability, TokenRequest, Version, VersionStatus};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Yeet {
    /// Override the hostname, default is the hostname of the system
    #[arg(short, long)]
    name: String,

    /// Path to the token file
    /// Requires permission `Capability::Token { capabilities: vec![Capability::SystemCheck { hostname: name }] }`
    #[arg(short, long)]
    token_file: PathBuf,

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
    let check_url = args.url.join(&format!("system/{}/check", args.name))?;
    let mut token = create_token(&args)?;
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
        let check = check.error_for_status()?;
        match check.json::<VersionStatus>()? {
            VersionStatus::UpToDate => {}
            VersionStatus::NewVersionAvailable(version) => {
                update(&version)?;
            }
        }
        sleep(Duration::from_secs(args.sleep));
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

fn create_token(args: &Yeet) -> Result<String> {
    let token = read_to_string(&args.token_file)?;
    let token = token.trim();

    let token_url = args.url.join("/token/new")?;
    let token_request = TokenRequest {
        capabilities: vec![Capability::SystemCheck {
            hostname: args.name.clone(),
        }],
        exp: Default::default(),
    };
    let token = Client::new()
        .post(token_url.as_str())
        .bearer_auth(token)
        .json(&token_request)
        .send()?;
    let token = token.error_for_status()?;
    Ok(token
        .json::<serde_json::Value>()?
        .get("token")
        .ok_or(anyhow!("Error creating token"))?
        .as_str()
        .ok_or(anyhow!("Error creating token"))?
        .to_owned())
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

fn set_system_profile(version: &Version) -> Result<()> {
    let profile = Command::new("nix-env")
        .args([
            "--profile",
            "/nix/var/nix/profiles/system",
            "--set",
            &version.store_path,
        ])
        .output()?;
    if !profile.status.success() {
        bail!("{}", String::from_utf8(profile.stderr)?);
    }
    Ok(())
}
#[cfg(target_os = "macos")]
fn activate(version: &Version) -> Result<()> {
    set_system_profile(version)?;
    let activate = Command::new(format!("{}/activate", version.store_path)).output()?;
    if !activate.status.success() {
        bail!("{}", String::from_utf8(activate.stderr)?);
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn activate(version: &Version) -> Result<()> {
    set_system_profile(version)?;
    let activate = Command::new(format!(
        "{}/bin/switch-to-configuration",
        version.store_path
    ))
    .arg("switch")
    .output()?;
    if !activate.status.success() {
        bail!("{}", String::from_utf8(activate.stderr)?);
    }
    Ok(())
}
