//! # Yeet Agent

use std::fs::read_to_string;
use std::path::Path;

use api::key::get_secret_key;
use clap::Parser as _;
use figment::Figment;
use figment::providers::{Env, Format as _, Serialized, Toml};
use log::info;
use rootcause::hooks::Hooks;
use rootcause::prelude::ResultExt;
use rootcause::{Report, bail, report};
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
async fn main() -> Result<(), Report> {
    Hooks::new()
        .report_formatter(rootcause::hooks::builtin_hooks::report_formatter::DefaultReportFormatter::UNICODE_COLORS)
        .install()
        .expect("failed to install hooks");

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let xdg_dirs = xdg::BaseDirectories::with_prefix("yeet");
    let args = Yeet::try_parse()?;
    let config: Config = Figment::new()
        .merge(Toml::file(
            xdg_dirs.find_config_file("agent.toml").unwrap_or_default(),
        ))
        .merge(Serialized::defaults(args.config))
        .merge(Env::prefixed("YEET_"))
        .extract()?;
    match args.command {
        Commands::Build { path, host, darwin } => {
            info!(
                "{:?}",
                nix::build_hosts(&path.to_string_lossy(), host, darwin)?
            );
        }
        Commands::VM { host, path } => run_vm(&path, &host)?,
        Commands::Agent { sleep, facter } => {
            agent::agent(&config, sleep, facter).await?;
        }
        Commands::Status => {
            info!(
                "{}",
                status_string(&config.url, &config.httpsig_key)
                    .await
                    .context("Failed to get status")?
            );
        }
        Commands::Publish {
            path,
            host,
            darwin,
            netrc,
        } => {
            let cachix = config.cachix.clone().ok_or(report!(
                "Cachix cache name required. Set it in config or via the --cachix flag"
            ))?;

            let netrc = match netrc {
                Some(netrc) => Some(
                    read_to_string(&netrc)
                        .context("Could not read netrc file")
                        .attach(format!("File: {}", &netrc.to_string_lossy()))?,
                ),
                None => None,
            };

            let public_key = if let Some(key) = &config.cachix_key {
                key.clone()
            } else {
                let cache_info = cachix::get_cachix_info(&cachix).await.context(
                    "Could not get cache information. For private caches use `--cachix-key`",
                )?;
                cache_info
                    .public_signing_keys
                    .first()
                    .cloned()
                    .ok_or(report!("Cachix cache has no public signing keys"))?
            };

            info!("Building {host:?}");

            let hosts = nix::build_hosts(&path.to_string_lossy(), host, darwin)?;

            if hosts.is_empty() {
                bail!("No hosts found - did you commit your files?")
            }

            info!("Pushing {hosts:?}");

            cachix::push_paths(hosts.values(), &cachix).await?;

            let before = status_string(&config.url, &config.httpsig_key).await?;
            server::update(
                &config.url,
                &get_secret_key(&config.httpsig_key)?,
                &api::HostUpdateRequest {
                    hosts,
                    public_key,
                    substitutor: format!("https://{cachix}.cachix.org"),
                    netrc,
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

pub(crate) async fn status_string(url: &Url, httpsig_key: &Path) -> Result<String, Report> {
    let status = server::status(url, &get_secret_key(httpsig_key)?).await?;
    let rows = status
        .into_iter()
        .map(|host| display::host(&host))
        .collect::<Result<Vec<_>, Report>>()?;

    Ok(rows.join("\n"))
}
