//! # Yeet Agent

use std::{
    fs::read_to_string,
    io::{IsTerminal as _, Write as _},
};

use api::key::get_secret_key;
use clap::Parser as _;
use figment::{
    Figment,
    providers::{Env, Format as _, Serialized, Toml},
};
use log::info;
use rootcause::{Report, bail, hooks::Hooks, prelude::ResultExt as _, report};
use yeet::{
    cachix,
    display::diff_inline,
    nix::{self, run_vm},
    server,
};

use crate::{
    cli::{Commands, Config, Yeet},
    status::status_string,
};

mod agent;
mod cli;
mod section;
mod server_cli;
mod status;
mod systemd;
mod varlink;
mod version;

#[expect(unexpected_cfgs)]
#[tokio::main(flavor = "local")]
#[expect(clippy::too_many_lines)]
#[expect(clippy::unwrap_in_result)]
async fn main() -> Result<(), Report> {
    Hooks::new()
                .report_formatter(rootcause::hooks::builtin_hooks::report_formatter::DefaultReportFormatter::UNICODE_COLORS)
                .install()
                .expect("failed to install hooks");

    let mut log_builder =
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"));

    if std::io::stderr().is_terminal() {
        log_builder.format(|buf, record| {
            write!(buf, "{}", buf.default_level_style(record.level()))?;
            write!(buf, "{}", record.level())?;
            write!(buf, "{:#}", buf.default_level_style(record.level()))?;
            writeln!(buf, ": {}", record.args())
        });
    }
    log_builder.init();
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
        Commands::Build {
            path,
            host,
            darwin,
            variant,
        } => {
            info!(
                "{:?}",
                nix::build_hosts(&path.to_string_lossy(), host, darwin, variant)?
            );
        }
        Commands::VM { host, path } => run_vm(&path, &host)?,
        Commands::Agent { sleep, facter } => {
            agent::agent(&config, sleep, facter).await?;
        }
        Commands::Status { json } => status::status(json).await?,
        Commands::Publish {
            path,
            host,
            darwin,
            netrc,
            variant,
        } => {
            let url = &config
                .url
                .clone()
                .ok_or(rootcause::report!("`--url` required for publish"))?;

            let httpsig_key = &config
                .httpsig_key
                .clone()
                .ok_or(rootcause::report!("`--httpsig_key` required for publish"))?;

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

            let hosts = nix::build_hosts(&path.to_string_lossy(), host, darwin, variant)?;

            if hosts.is_empty() {
                bail!("No hosts found - did you commit your files?")
            }

            info!("Pushing {hosts:?}");

            cachix::push_paths(hosts.values(), &cachix).await?;

            let before = status_string(&url, &httpsig_key).await?;
            server::update(
                &url,
                &get_secret_key(&httpsig_key)?,
                &api::HostUpdateRequest {
                    hosts,
                    public_key,
                    substitutor: format!("https://{cachix}.cachix.org"),
                    netrc,
                },
            )
            .await?;
            let after = status_string(&url, &httpsig_key).await?;
            info!("{}", diff_inline(&before, &after));
        }
        Commands::Server(args) => server_cli::handle_server_commands(args.command, &config).await?,
    }
    Ok(())
}
