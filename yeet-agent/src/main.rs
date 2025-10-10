//! # Yeet Agent

use std::collections::HashMap;
use std::env::current_dir;
use std::fs::{File, read_link, read_to_string};
use std::hash::Hash;
use std::io::{BufRead as _, BufReader};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str;

use anyhow::{Ok, Result, anyhow, bail};
use api::hash_hex;
use clap::{Args, Parser, Subcommand, arg};
use ed25519_dalek::VerifyingKey;
use ed25519_dalek::pkcs8::DecodePublicKey as _;
use figment::Figment;
use figment::providers::{Env, Format as _, Serialized, Toml};
use httpsig_hyper::prelude::SecretKey;
use jiff::Zoned;
use log::info;
use notify_rust::Notification;
use serde::{Deserialize, Serialize};
use url::Url;
use yeet_agent::nix::{self, run_vm};
use yeet_agent::{cachix, display, server};

use crate::cli::{Commands, Config, Yeet};

mod cli;

#[tokio::main]
#[expect(clippy::too_many_lines)]
async fn main() -> anyhow::Result<()> {
    let xdg_dirs = xdg::BaseDirectories::with_prefix("yeet");
    let args = Yeet::try_parse()?;
    let config: Config = Figment::new()
        .merge(Serialized::defaults(args.config))
        .merge(Toml::file(
            xdg_dirs.find_config_file("agent.toml").unwrap_or_default(),
        ))
        .merge(Env::prefixed("YEET_"))
        .extract()?;
    match args.command {
        Commands::Build { path, host } => {
            println!(
                "{:?}",
                nix::build_hosts(&path.to_string_lossy(), host, true)?
            );
        }
        Commands::VM { host, path } => run_vm(&path, &host)?,
        Commands::Agent { sleep } => todo!(),
        Commands::Status => {
            let status = server::status(config.url, get_key(&config.httpsig_key)?).await?;
            let rows = status
                .into_iter()
                .map(|host| display::host(&host))
                .collect::<Result<Vec<_>>>()?;

            println!("{}", rows.join("\n"));
        }
        Commands::Register {
            host_key,
            store_path,
            name,
        } => {
            println!(
                "{:?}",
                server::register(
                    config.url,
                    get_key(&config.httpsig_key)?,
                    api::RegisterHost {
                        key: get_pub_key(&host_key)?,
                        store_path,
                        name
                    }
                )
                .await
            );
        }
        Commands::Update {
            host,
            store_path,
            public_key,
            substitutor,
        } => {
            println!(
                "{:?}",
                server::update(
                    config.url,
                    get_key(&config.httpsig_key)?,
                    api::HostUpdateRequest {
                        hosts: HashMap::from([(host, store_path)]),
                        public_key,
                        substitutor
                    }
                )
                .await
            );
        }
        Commands::Publish { path, host } => {
            let hosts = nix::build_hosts(&path.to_string_lossy(), host, true)?;
            let cache_info = cachix::get_cachix_info(config.cachix.ok_or(anyhow!(
                "Cachix cache name required. Set it in config or via the --cachix flag"
            ))?)
            .await?;

            let public_key = cache_info
                .public_signing_keys
                .first()
                .cloned()
                .ok_or(anyhow!("Cachix cache has no public signing keys"))?;

            cachix::push_paths(hosts.values(), cache_info.name).await?;

            server::update(
                config.url,
                get_key(&config.httpsig_key)?,
                api::HostUpdateRequest {
                    hosts,
                    public_key,
                    substitutor: cache_info.uri,
                },
            )
            .await?;
        }
    }
    Ok(())
}

fn get_key(path: &Path) -> anyhow::Result<SecretKey> {
    Ok(SecretKey::from_pem(&read_to_string(path)?)?)
}

fn get_pub_key(path: &Path) -> anyhow::Result<VerifyingKey> {
    Ok(VerifyingKey::from_public_key_pem(&read_to_string(path)?)?)
}

#[cfg(false)]
fn main_loop() -> Result<()> {
    let args = Yeet::try_parse()?;
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
            .json(&VersionRequest { store_path })
            .send()?;
        if !check.status().is_success() {
            bail!("Server Error ({}): {}", check.status(), check.text()?);
        }
        match check.json::<VersionStatus>()? {
            VersionStatus::UpToDate => {
                info!("UpToDate");
            }
            VersionStatus::NewVersionAvailable(version) => {
                info!("NewVersionAvailable");
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

fn update(version: &api::Version) -> Result<()> {
    download(version)?;
    activate(version)?;
    Notification::new()
        .summary("System Update")
        .body("System has been updated successfully")
        .appname("Yeet")
        .show()?;
    Ok(())
}

fn download(version: &api::Version) -> Result<()> {
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

fn set_system_profile(version: &api::Version) -> Result<()> {
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
fn activate(version: &api::Version) -> Result<()> {
    set_system_profile(version)?;
    info!("Activating {}", version.store_path);
    Command::new(Path::new(&version.store_path).join("activate")).spawn()?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn activate(version: &Version) -> Result<()> {
    info!("Activating {}", version.store_path);
    set_system_profile(version)?;
    Command::new(Path::new(&version.store_path).join("bin/switch-to-configuration"))
        .arg("switch")
        .spawn()?;
    Ok(())
}
