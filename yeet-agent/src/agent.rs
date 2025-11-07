use anyhow::{Ok, anyhow};
use backon::{ConstantBuilder, Retryable as _};
use ed25519_dalek::pkcs8::DecodePrivateKey;
use ed25519_dalek::{SigningKey, VerifyingKey};
use httpsig_hyper::prelude::{AlgorithmName, SecretKey};
use std::fs::read_to_string;
use std::io::{BufRead as _, BufReader};
use std::path::Path;
use std::sync::OnceLock;
use std::time::Duration;
use std::{
    fs::{File, read_link},
    process::Command,
};
use tokio::time;
use yeet::server;

use anyhow::bail;
use log::{error, info};
use notify_rust::Notification;
use ssh_key::PrivateKey;

use crate::cli::Config;

static VERIFICATION_CODE: OnceLock<u32> = OnceLock::new();

/// When running the agent should do these things in order:
/// 1. Check if agent is active aka if the key is enrolled with `/system/verify`
///     if not:
///         create a new verification request
///         pull the verify endpoint in a time intervall
/// 2. Continuosly pull the system endpoint and execute based on the provided
pub async fn agent(config: &Config, sleep: u64) -> anyhow::Result<()> {
    let secret_key = read_to_string(&config.httpsig_key)?;
    let (pub_key, key) = if secret_key.contains("BEGIN OPENSSH PRIVATE KEY") {
        let key = PrivateKey::from_openssh(secret_key)?;
        let bytes = key
            .key_data()
            .ed25519()
            .ok_or(anyhow!("Key is not of type ED25519"))?
            .private
            .to_bytes();
        let pub_key = SigningKey::from_bytes(&bytes).verifying_key();
        let key = SecretKey::from_bytes(AlgorithmName::Ed25519, &bytes)?;
        (pub_key, key)
    } else {
        let pub_key = SigningKey::from_pkcs8_pem(secret_key.as_str())?.verifying_key();
        let key = SecretKey::from_pem(secret_key.as_str())?;
        (pub_key, key)
    };

    (|| async { agent_loop(config, &key, pub_key, sleep).await })
        .retry(
            ConstantBuilder::new()
                .without_max_times()
                .with_delay(Duration::from_secs(sleep)),
        )
        .notify(|err: &anyhow::Error, dur: Duration| {
            error!("{err:?} - retrying in {dur:?}");
        })
        .await?;

    Ok(())
}

async fn agent_loop(
    config: &Config,
    key: &SecretKey,
    pub_key: VerifyingKey,
    sleep: u64,
) -> anyhow::Result<()> {
    let verified = server::is_host_verified(&config.url, key)
        .await?
        .is_success();

    if !verified {
        if let Some(code) = VERIFICATION_CODE.get() {
            bail!("Verification requested but not yet approved. Code: {code}");
        }
        let code = server::add_verification_attempt(
            &config.url,
            &api::VerificationAttempt {
                key: pub_key,
                store_path: get_active_version()?,
            },
        )
        .await?;
        VERIFICATION_CODE.set(code);
        info!("Your verification code is: {code}");
        bail!("Waiting for verification");
    }
    info!("Verified!");

    loop {
        let action = server::system_check(
            &config.url,
            key,
            &api::VersionRequest {
                store_path: get_active_version()?,
            },
        )
        .await?;

        info!("{action:#?}");

        agent_action(action)?;
        time::sleep(Duration::from_secs(sleep)).await;
    }
}

fn agent_action(action: api::AgentAction) -> anyhow::Result<()> {
    match action {
        api::AgentAction::Nothing => {}
        api::AgentAction::Detach => {}
        api::AgentAction::SwitchTo(remote_store_path) => update(&remote_store_path)?,
    }
    Ok(())
}

fn get_active_version() -> anyhow::Result<String> {
    Ok(read_link("/run/current-system")?
        .to_string_lossy()
        .to_string())
}

fn trusted_public_keys() -> anyhow::Result<Vec<String>> {
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

fn update(version: &api::RemoteStorePath) -> anyhow::Result<()> {
    download(version)?;
    activate(version)?;
    Notification::new()
        .summary("System Update")
        .body("System has been updated successfully")
        .appname("Yeet")
        .show()?;
    Ok(())
}

fn download(version: &api::RemoteStorePath) -> anyhow::Result<()> {
    info!("Downloading {}", version.store_path);
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

fn set_system_profile(version: &api::RemoteStorePath) -> anyhow::Result<()> {
    info!("Setting system profile to {}", version.store_path);
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
fn activate(version: &api::RemoteStorePath) -> anyhow::Result<()> {
    set_system_profile(version)?;
    info!("Activating {}", version.store_path);
    Command::new(Path::new(&version.store_path).join("activate"))
        .spawn()?
        .wait()?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn activate(version: &api::RemoteStorePath) -> Result<()> {
    info!("Activating {}", version.store_path);
    set_system_profile(version)?;
    Command::new(Path::new(&version.store_path).join("bin/switch-to-configuration"))
        .arg("switch")
        .spawn()?
        .wait()?;
    Ok(())
}
