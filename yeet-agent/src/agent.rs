use std::{
    fs::File,
    io::{self, BufRead as _, BufReader, Write as _},
    path::Path,
    process::Command,
    sync::OnceLock,
    time::Duration,
};

use api::key::{get_secret_key, get_verify_key};
use backon::{ConstantBuilder, Retryable as _};
use ed25519_dalek::VerifyingKey;
use httpsig_hyper::prelude::SecretKey;
use log::{error, info};
use notify_rust::Notification;
use rootcause::{Report, bail, prelude::ResultExt as _, report};
use tempfile::NamedTempFile;
use tokio::time;
use yeet::{nix, server};

use crate::{cli_args::Config, varlink, version::get_active_version};

static VERIFICATION_CODE: OnceLock<u32> = OnceLock::new();

/// When running the agent should do these things in order:
/// 1. Check if agent is active aka if the key is enrolled with `/system/verify`
///     if not:
///         create a new verification request
///         pull the verify endpoint in a time intervall
/// 2. Continuosly pull the system endpoint and execute based on the provided
pub async fn agent(config: &Config, sleep: u64, facter: bool) -> Result<(), Report> {
    let _ = &config
        .url
        .clone()
        .ok_or(rootcause::report!("`url` required for agent"))?;

    let _ = &config
        .httpsig_key
        .clone()
        .ok_or(rootcause::report!("`httpsig_key` required for agent"))?;

    let key = get_secret_key(config.httpsig_key.as_ref().unwrap())?;
    let pub_key = get_verify_key(config.httpsig_key.as_ref().unwrap())?;

    log::info!("Spawning varlink daemon");
    {
        let config = config.clone();
        let key = key.clone();
        tokio::task::spawn_local(async move {
            if let Err(err) = varlink::YeetVarlinkService::start(config, key).await {
                log::error!("Varlink failure:\n{err}");
            }
        });
    }

    (|| async { agent_loop(config, &key, pub_key, sleep, facter).await })
        .retry(
            ConstantBuilder::new()
                .without_max_times()
                .with_delay(Duration::from_secs(sleep)),
        )
        .notify(|err: &Report, dur: Duration| {
            error!("{err} - retrying in {dur:?}");
        })
        .await?;

    Ok(())
}

async fn agent_loop(
    config: &Config,
    key: &SecretKey,
    pub_key: VerifyingKey,
    sleep: u64,
    facter: bool,
) -> Result<(), Report> {
    let verified = server::is_host_verified(&config.url.as_ref().unwrap(), key) //TODO unwrap
        .await?
        .is_success();

    if !verified {
        if let Some(code) = VERIFICATION_CODE.get() {
            bail!("Verification requested but not yet approved. Code: {code}");
        }

        let nixos_facter = if facter {
            info!("Collecting nixos-facter information");
            let facts = Some(nix::facter()?);
            info!("Done collecting facts");
            facts
        } else {
            None
        };

        let code = server::add_verification_attempt(
            &config.url.as_ref().unwrap(), //TODO unwrap
            &api::VerificationAttempt {
                key: pub_key,
                store_path: get_active_version()?,
                artifacts: api::VerificationArtifacts { nixos_facter },
            },
        )
        .await?;
        let _ = VERIFICATION_CODE.set(code);
        info!("Your verification code is: {code}");
        bail!("Waiting for verification");
    }
    info!("Verified!");

    loop {
        let action = server::system_check(
            &config.url.as_ref().unwrap(), //TODO unwrap
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

fn agent_action(action: api::AgentAction) -> Result<(), Report> {
    match action {
        api::AgentAction::Nothing => {}
        api::AgentAction::Detach => {}
        api::AgentAction::SwitchTo(remote_store_path) => update(&remote_store_path)?,
    }
    Ok(())
}

fn trusted_public_keys() -> Result<Vec<String>, Report> {
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

fn update(version: &api::RemoteStorePath) -> Result<(), Report> {
    download(version)?;
    activate(version)?;
    Notification::new()
        .summary("System Update")
        .body("System has been updated successfully")
        .appname("Yeet")
        .show()?;
    Ok(())
}

fn download(version: &api::RemoteStorePath) -> Result<(), Report> {
    info!("Downloading {}", version.store_path);
    let mut keys = trusted_public_keys()?;
    keys.push(version.public_key.clone());
    keys.sort();
    keys.dedup();

    let mut command = Command::new("nix-store");
    command.stderr(io::stderr()).stdout(io::stdout());
    command.args(vec![
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
    ]);

    // Even if we do not end up using the temp file we create it outside of the if scope.
    // Else it would get dropped before nix-store can use it
    let mut netrc_file = NamedTempFile::new().context("Could not create netrc temp file")?;
    if let Some(netrc) = &version.netrc {
        netrc_file
            .write_all(netrc.as_bytes())
            .context("Could not write to the temp netrc file")?;
        netrc_file.flush()?;
        command.args([
            "--option",
            "netrc-file",
            &netrc_file.path().to_string_lossy(),
        ]);
    }

    let download = command.output()?;

    if !download.status.success() {
        return Err(report!("{}", String::from_utf8(download.stderr)?)
            .context("Could not realize new version")
            .attach(format!(
                "Command: {}",
                command
                    .get_args()
                    .map(|ostr| ostr.to_string_lossy())
                    .collect::<Vec<_>>()
                    .join(" ")
            ))
            .into_dynamic());
    }
    Ok(())
}

fn set_system_profile(version: &api::RemoteStorePath) -> Result<(), Report> {
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
fn activate(version: &api::RemoteStorePath) -> Result<(), Report> {
    set_system_profile(version)?;
    info!("Activating {}", version.store_path);
    Command::new(Path::new(&version.store_path).join("activate"))
        .spawn()?
        .wait()?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn activate(version: &api::RemoteStorePath) -> Result<(), Report> {
    info!("Activating {}", version.store_path);
    set_system_profile(version)?;
    Command::new(Path::new(&version.store_path).join("bin/switch-to-configuration"))
        .arg("switch")
        .spawn()?
        .wait()?;
    Ok(())
}
