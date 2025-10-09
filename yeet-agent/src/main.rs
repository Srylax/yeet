//! # Yeet Agent

use std::collections::HashMap;
use std::env::current_dir;
use std::fs::{File, read_link, read_to_string};
use std::io::{BufRead as _, BufReader};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str;

use anyhow::{Ok, Result, bail};
use clap::{Args, Parser, Subcommand, arg};
use ed25519_dalek::VerifyingKey;
use ed25519_dalek::pkcs8::DecodePublicKey as _;
use figment::Figment;
use figment::providers::{Env, Format as _, Serialized, Toml};
use httpsig_hyper::prelude::SecretKey;
use log::info;
use notify_rust::Notification;
use serde::{Deserialize, Serialize};
use url::Url;
use yeet_agent::nix::run_vm;
use yeet_agent::server;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Yeet {
    #[command(flatten)]
    config: ClapConfig,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Args, Serialize, Deserialize)]
struct ClapConfig {
    /// Base URL of the Yeet Server
    #[arg(long, global = true)]
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<Url>,

    /// Path to the admin key
    #[arg(long, global = true)]
    #[serde(skip_serializing_if = "Option::is_none")]
    httpsig_key: Option<PathBuf>, // TODO: create a key selector
}

#[derive(Serialize, Deserialize)]
struct Config {
    url: Url,
    httpsig_key: PathBuf,
}

#[expect(clippy::doc_markdown, reason = "No Markdown for clap")]
#[derive(Subcommand)]
enum Commands {
    Agent {
        /// Seconds to wait between updates.
        /// Lower bound, may be higher between switching versions
        #[arg(short, long, default_value = "30")]
        sleep: u64,
    },
    /// Query the status of all or some (TODO) hosts [requires Admin credentials]
    Status,
    /// Register a new host
    Register {
        /// Pub key of the client
        #[arg(long)]
        host_key: PathBuf,

        /// Store path of the first version
        #[arg(long)]
        store_path: String,

        /// Pet name for the host
        #[arg(long)]
        name: Option<String>,
    },
    /// Update a host e.g. push a new store_path TODO: batch update
    Update {
        /// Pub key of the client
        #[arg(long)]
        host_key: PathBuf,

        /// The new store path
        #[arg(long)]
        store_path: String,

        /// The public key the agent should use to verify the update
        #[arg(long)]
        public_key: String,

        /// The substitutor the agent should use to fetch the update
        #[arg(long)]
        substitutor: String,
    },
    /// Run you hosts inside a vm
    VM {
        /// NixOs host to run and build
        #[arg(index = 1)]
        host: String,
        /// Path to flake
        #[arg(long, default_value = current_dir().unwrap().into_os_string())]
        path: PathBuf,
    },
}

#[tokio::main]
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
        Commands::VM { host, path } => run_vm(&path, &host)?,
        Commands::Agent { sleep } => todo!(),
        Commands::Status => {
            println!(
                "{:?}",
                server::status(config.url, get_key(&config.httpsig_key)?).await
            );
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
            host_key,
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
                        hosts: HashMap::from([(get_pub_key(&host_key)?, store_path)]),
                        public_key,
                        substitutor
                    }
                )
                .await
            );
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
