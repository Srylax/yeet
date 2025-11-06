//! # Yeet Agent

use std::fs::read_to_string;
use std::path::Path;

use anyhow::{Ok, Result};
use clap::Parser as _;
use ed25519_dalek::VerifyingKey;
use ed25519_dalek::pkcs8::DecodePublicKey as _;
use figment::Figment;
use figment::providers::{Env, Format as _, Serialized, Toml};
use httpsig_hyper::prelude::SecretKey;
use url::Url;
use yeet_agent::nix::{self, run_vm};
use yeet_agent::{display, server};

use crate::cli::{Commands, Config, Yeet};

mod agent;
mod cli;
mod server_cli;

#[tokio::main]
#[expect(clippy::too_many_lines)]
#[expect(clippy::unwrap_in_result)]
async fn main() -> anyhow::Result<()> {
    let xdg_dirs = xdg::BaseDirectories::with_prefix("yeet");
    let args = Yeet::try_parse()?;
    let config: Config = Figment::new()
        .merge(Serialized::defaults(args.config))
        .merge(Toml::file(
            xdg_dirs.find_config_file("agent.toml").unwrap_or_default(),
        ))
        .merge(Toml::file(".config/yeet.toml"))
        .merge(Env::prefixed("YEET_"))
        .extract()?;
    match args.command {
        Commands::Build { path, host } => {
            println!(
                "{:?}",
                nix::build_hosts(
                    &path.to_string_lossy(),
                    host,
                    std::env::consts::ARCH == "aarch64"
                )?
            );
        }
        Commands::VM { host, path } => run_vm(&path, &host)?,
        Commands::Agent { sleep } => todo!(),
        Commands::Status => {
            println!("{}", status_string(&config.url, &config.httpsig_key).await?);
        }
        Commands::Server(args) => server_cli::handle_server_commands(args.command, &config).await?,
    }
    Ok(())
}

pub(crate) async fn status_string(url: &Url, httpsig_key: &Path) -> anyhow::Result<String> {
    let status = server::status(url, get_key(httpsig_key)?).await?;
    let rows = status
        .into_iter()
        .map(|host| display::host(&host))
        .collect::<Result<Vec<_>>>()?;

    Ok(rows.join("\n"))
}

pub(crate) fn get_key(path: &Path) -> anyhow::Result<SecretKey> {
    Ok(SecretKey::from_pem(&read_to_string(path)?)?)
}

pub(crate) fn get_pub_key(path: &Path) -> anyhow::Result<VerifyingKey> {
    Ok(VerifyingKey::from_public_key_pem(&read_to_string(path)?)?)
}
