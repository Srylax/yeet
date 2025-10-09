//! # Yeet Agent

use std::env::current_dir;
use std::fs::{File, read_link, read_to_string};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str;
use std::thread::sleep;
use std::time::Duration;

use anyhow::{Ok, Result, anyhow, bail};
use clap::{Args, Parser, Subcommand, arg};
use ed25519_dalek::SigningKey;
use ed25519_dalek::ed25519::signature::SignerMut as _;
use httpsig_hyper::prelude::{AlgorithmName, SecretKey};
use log::{error, info};
use notify_rust::Notification;
use ssh_key::PrivateKey;
use url::Url;
use yeet_agent::nix::run_vm;
use yeet_agent::server;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Yeet {
    #[command(subcommand)]
    command: Commands,
}

#[expect(clippy::doc_markdown, reason = "No Markdown for clap")]
#[derive(Subcommand)]
enum Commands {
    Agent {
        /// Base URL of the Yeet Server
        #[arg(short, long)]
        url: Url,

        /// Seconds to wait between updates.
        /// Lower bound, may be higher between switching versions
        #[arg(short, long, default_value = "30")]
        sleep: u64,
    },
    /// Query the status of all or some (TODO) hosts [requires Admin credentials]
    Status {
        /// Base URL of the Yeet Server
        #[arg(short, long)]
        url: Url,

        /// Path to the admin key
        #[arg(long)]
        key: PathBuf, // TODO: create a key selector
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
    let args = Yeet::try_parse()?;
    match args.command {
        Commands::VM { host, path } => run_vm(&path, &host)?,
        Commands::Agent { url, sleep } => todo!(),
        Commands::Status { key, url } => {
            println!("{:?}", server::status(url, get_key(&key)?).await);
        }
    }
    Ok(())
}

fn get_key(path: &Path) -> anyhow::Result<SecretKey> {
    Ok(SecretKey::from_pem(&read_to_string(path)?)?)
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
