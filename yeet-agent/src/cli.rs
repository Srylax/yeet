use std::path::PathBuf;

use inquire::{list_option::ListOption, validator::Validation};
use log::info;
use rootcause::{Report, bail, prelude::ResultExt as _, report};
use tokio::fs::read_to_string;
use yeet::{cachix, server};

use crate::{cli_args::Config, nix, sig::ssh, varlink};

pub async fn publish(
    config: &Config,
    path: PathBuf,
    host: Vec<String>,
    netrc: Option<PathBuf>,
    variant: Option<String>,
    darwin: bool,
) -> Result<(), Report> {
    let agent_url = {
        let agent_config = varlink::config().await;
        if let Err(e) = &agent_config {
            log::error!("Could not get agent config: {e}")
        }
        agent_config.ok().map(|config| config.server)
    };

    let url = &config
        .url
        .clone()
        .or(agent_url)
        .ok_or(rootcause::report!("`--url` required for publish"))?;
    let domain = url
        .domain()
        .ok_or(rootcause::report!("Provided URL has no domain part"))?;

    let cachix = config.cachix.clone().ok_or(report!(
        "Cachix cache name required. Set it in config or via the --cachix flag"
    ))?;

    let netrc = match netrc {
        Some(netrc) => Some(
            read_to_string(&netrc)
                .await
                .context("Could not read netrc file")
                .attach(format!("File: {}", &netrc.to_string_lossy()))?,
        ),
        None => None,
    };

    let public_key = if let Some(key) = &config.cachix_key {
        key.clone()
    } else {
        let cache_info = cachix::get_cachix_info(&cachix)
            .await
            .context("Could not get cache information. For private caches use `--cachix-key`")?;
        cache_info
            .public_signing_keys
            .first()
            .cloned()
            .ok_or(report!("Cachix cache has no public signing keys"))?
    };

    let host = if host.is_empty() {
        get_hosts(&path.to_string_lossy(), darwin)?
    } else {
        host
    };

    info!("Building {host:?}");

    let hosts = nix::build_hosts(&path.to_string_lossy(), host, darwin, variant)?;

    if hosts.is_empty() {
        bail!("No hosts found - did you commit your files?")
    }

    info!("Pushing {hosts:?}");

    cachix::push_paths(hosts.values(), &cachix).await?;

    server::update(
        &url,
        &ssh::key_by_url(domain)?,
        &api::HostUpdateRequest {
            hosts,
            public_key,
            substitutor: format!("https://{cachix}.cachix.org"),
            netrc,
        },
    )
    .await?;
    Ok(())
}

pub fn get_hosts(flake_path: &str, darwin: bool) -> Result<Vec<String>, Report> {
    let detected_hosts = nix::list_hosts(flake_path, darwin)?;
    Ok(
        inquire::MultiSelect::new("Which host(s) would you like to publish>", detected_hosts)
            .with_validator(|list: &[ListOption<&String>]| {
                if list.len() < 1 {
                    return Ok(Validation::Invalid("You must select a host!".into()));
                } else {
                    Ok(Validation::Valid)
                }
            })
            .prompt()?,
    )
}
