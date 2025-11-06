use std::collections::HashMap;

use anyhow::{anyhow, bail};
use yeet_agent::{cachix, display::diff_inline, nix, server};

use crate::{
    cli::{Config, ServerCommands},
    get_sig_key, get_verify_key, status_string,
};

pub async fn handle_server_commands(
    command: ServerCommands,
    config: &Config,
) -> anyhow::Result<()> {
    match command {
        ServerCommands::Register {
            store_path,
            name,
            public_key,
            substitutor,
        } => {
            let before = status_string(&config.url, &config.httpsig_key).await?;

            let provision_state = if let Some(store_path) = store_path
                && let Some(public_key) = public_key
                && let Some(substitutor) = substitutor
            {
                api::ProvisionState::Provisioned(api::Version {
                    public_key,
                    store_path,
                    substitutor,
                })
            } else {
                api::ProvisionState::NotSet
            };

            server::register(
                &config.url,
                &get_sig_key(&config.httpsig_key)?,
                &api::RegisterHost {
                    provision_state,
                    name,
                },
            )
            .await?;
            let after = status_string(&config.url, &config.httpsig_key).await?;
            println!("{}", diff_inline(&before, &after));
        }
        ServerCommands::Update {
            host,
            store_path,
            public_key,
            substitutor,
        } => {
            let before = status_string(&config.url, &config.httpsig_key).await?;
            server::update(
                &config.url,
                &get_sig_key(&config.httpsig_key)?,
                &api::HostUpdateRequest {
                    hosts: HashMap::from([(host, store_path)]),
                    public_key,
                    substitutor,
                },
            )
            .await?;
            let after = status_string(&config.url, &config.httpsig_key).await?;
            println!("{}", diff_inline(&before, &after));
        }
        ServerCommands::Publish { path, host } => {
            let before = status_string(&config.url, &config.httpsig_key).await?;

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

            println!("{hosts:?}");
            cachix::push_paths(hosts.values(), cache_info.name).await?;

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
            println!("{}", diff_inline(&before, &after));
        }
        ServerCommands::VerifyStatus => {
            let status =
                server::is_host_verified(&config.url, &get_sig_key(&config.httpsig_key)?).await?;
            println!("{status}");
        }
        ServerCommands::AddVerification {
            store_path,
            public_key,
        } => {
            let code = server::add_verification_attempt(
                &config.url,
                &api::VerificationAttempt {
                    store_path,
                    key: get_verify_key(&public_key)?,
                },
            )
            .await?;
            println!("{code}");
        }
        ServerCommands::VerifyAttempt { name, code } => {
            let status = server::verify_attempt(
                &config.url,
                &get_sig_key(&config.httpsig_key)?,
                &api::VerificationAcceptance {
                    code,
                    host_name: name,
                },
            )
            .await?;
            println!("{status}");
        }
    }
    Ok(())
}
