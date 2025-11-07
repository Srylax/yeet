//! # Yeet Agent

use std::fs::read_to_string;
use std::path::Path;

use anyhow::anyhow;
use anyhow::{Ok, Result, bail};
use clap::Parser as _;
use ed25519_dalek::VerifyingKey;
use ed25519_dalek::pkcs8::DecodePublicKey as _;
use figment::Figment;
use figment::providers::{Env, Format as _, Serialized, Toml};
use httpsig_hyper::prelude::SecretKey;
use log::info;
use url::Url;
use yeet::display::diff_inline;
use yeet::nix::{self, run_vm};
use yeet::{cachix, display, server};

use crate::cli::{Commands, Config, Yeet};

mod agent;
mod cli;
mod server_cli;

#[tokio::main]
#[expect(clippy::too_many_lines)]
#[expect(clippy::unwrap_in_result)]
async fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let xdg_dirs = xdg::BaseDirectories::with_prefix("yeet");
    let args = Yeet::try_parse()?;
    let config: Config = Figment::new()
        .merge(Toml::file(
            xdg_dirs.find_config_file("agent.toml").unwrap_or_default(),
        ))
        .merge(Serialized::defaults(args.config))
        .merge(Toml::file(".config/yeet.toml"))
        .merge(Env::prefixed("YEET_"))
        .extract()?;
    match args.command {
        Commands::Build { path, host } => {
            info!(
                "{:?}",
                nix::build_hosts(
                    &path.to_string_lossy(),
                    host,
                    std::env::consts::ARCH == "aarch64"
                )?
            );
        }
        Commands::VM { host, path } => run_vm(&path, &host)?,
        Commands::Agent { sleep } => {
            agent::agent(&config, sleep).await?;
        }
        Commands::Status => {
            info!("{}", status_string(&config.url, &config.httpsig_key).await?);
        }
        Commands::Publish { path, host } => {
            let hosts = nix::build_hosts(
                &path.to_string_lossy(),
                host,
                std::env::consts::ARCH == "aarch64",
            )?;

            if hosts.is_empty() {
                bail!("No hosts found - did you commit your files?")
            }

            let cache_info = cachix::get_cachix_info(config.cachix.clone().ok_or(anyhow!(
                "Cachix cache name required. Set it in config or via the --cachix flag"
            ))?)
            .await?;

            let public_key = cache_info
                .public_signing_keys
                .first()
                .cloned()
                .ok_or(anyhow!("Cachix cache has no public signing keys"))?;

            info!("{hosts:?}");
            cachix::push_paths(hosts.values(), cache_info.name).await?;

            let before = status_string(&config.url, &config.httpsig_key).await?;
            server::update(
                &config.url,
                &get_sig_key(&config.httpsig_key)?,
                &api::HostUpdateRequest {
                    hosts,
                    public_key,
                    substitutor: cache_info.uri,
                },
            )
            .await?;
            let after = status_string(&config.url, &config.httpsig_key).await?;
            info!("{}", diff_inline(&before, &after));
        }
        Commands::Server(args) => server_cli::handle_server_commands(args.command, &config).await?,
    }
    Ok(())
}

pub(crate) async fn status_string(url: &Url, httpsig_key: &Path) -> anyhow::Result<String> {
    let status = server::status(url, &get_sig_key(httpsig_key)?).await?;
    let rows = status
        .into_iter()
        .map(|host| display::host(&host))
        .collect::<Result<Vec<_>>>()?;

    Ok(rows.join("\n"))
}

pub(crate) fn get_sig_key(path: &Path) -> anyhow::Result<SecretKey> {
    Ok(SecretKey::from_pem(&read_to_string(path)?)?)
}

pub(crate) fn get_verify_key(path: &Path) -> anyhow::Result<VerifyingKey> {
    Ok(VerifyingKey::from_public_key_pem(&read_to_string(path)?)?)
}
