//! # Yeet Agent

use std::fs::{File, read_link};
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::Command;
use std::str;
use std::thread::sleep;
use std::time::Duration;

use anyhow::{Ok, Result, anyhow, bail};
use clap::{Parser, arg};
use ed25519_dalek::SigningKey;
use ed25519_dalek::ed25519::signature::SignerMut as _;
use notify_rust::Notification;
use reqwest::blocking::Client;
use ssh_key::PrivateKey;
use url::Url;
use yeet_api::{Version, VersionRequest, VersionStatus};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Yeet {
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
    let check_url = args.url.join("system/check")?;
    let key = PrivateKey::read_openssh_file(Path::new("/etc/ssh/ssh_host_ed25519_key"))?;

    let mut key = SigningKey::from_bytes(
        &key.key_data()
            .ed25519()
            .ok_or(anyhow!("Key is not of type ED25519"))?
            .private
            .to_bytes(),
    );

    loop {
        let store_path = get_active_version()?;
        let check = Client::new()
            .post(check_url.as_str())
            .json(&VersionRequest {
                key: key.verifying_key(),
                signature: key.sign(store_path.as_bytes()),
                store_path,
            })
            .send()?;
        if !check.status().is_success() {
            bail!("Server Error ({}): {}", check.status(), check.text()?);
        }
        match check.json::<VersionStatus>()? {
            VersionStatus::UpToDate => {
                println!("UpToDate")
            }
            VersionStatus::NewVersionAvailable(version) => {
                println!("NewVersionAvailable");
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
