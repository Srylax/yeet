use std::path::PathBuf;

use api::key::get_secret_key;
use log::info;
use rootcause::{Report, bail, prelude::ResultExt as _, report};
use tokio::fs::read_to_string;
use yeet::{cachix, display, server};

use crate::{cli_args::Config, nix, status};

pub async fn publish(
    config: &Config,
    path: PathBuf,
    host: Vec<String>,
    netrc: Option<PathBuf>,
    variant: Option<String>,
    darwin: bool,
) -> Result<(), Report> {
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

    info!("Building {host:?}");

    let hosts = nix::build_hosts(&path.to_string_lossy(), host, darwin, variant)?;

    if hosts.is_empty() {
        bail!("No hosts found - did you commit your files?")
    }

    info!("Pushing {hosts:?}");

    cachix::push_paths(hosts.values(), &cachix).await?;

    let before = status::status_string(&url, &httpsig_key).await?;
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
    let after = status::status_string(&url, &httpsig_key).await?;
    info!("{}", display::diff_inline(&before, &after));
    todo!()
}
