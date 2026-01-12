use std::{
    collections::HashMap,
    fs::{File, read_to_string},
    io::Write as _,
};

use api::key::{get_secret_key, get_verify_key};
use log::info;
use rootcause::{Report, prelude::ResultExt as _};
use yeet::{display::diff_inline, server};

use crate::{
    cli_args::{AuthLevel, Config, ServerCommands},
    status::status_string,
};

pub async fn handle_server_commands(
    command: ServerCommands,
    config: &Config,
) -> Result<(), Report> {
    let url = &config
        .url
        .clone()
        .ok_or(rootcause::report!("`--url` required for server commands"))?;

    let httpsig_key = &config.httpsig_key.clone().ok_or(rootcause::report!(
        "`--httpsig_key` required for server commands"
    ))?;
    match command {
        ServerCommands::Register {
            store_path,
            name,
            public_key,
            substitutor,
            netrc,
        } => {
            let before = status_string(&url, &httpsig_key).await?;

            let provision_state = if let Some(store_path) = store_path
                && let Some(public_key) = public_key
                && let Some(substitutor) = substitutor
            {
                api::ProvisionState::Provisioned(api::RemoteStorePath {
                    public_key,
                    store_path,
                    substitutor,
                    netrc,
                })
            } else {
                api::ProvisionState::NotSet
            };

            server::register(
                &url,
                &get_secret_key(&httpsig_key)?,
                &api::RegisterHost {
                    provision_state,
                    name,
                },
            )
            .await?;
            let after = status_string(&url, &httpsig_key).await?;
            info!("{}", diff_inline(&before, &after));
        }
        ServerCommands::Update {
            host,
            store_path,
            public_key,
            substitutor,
            netrc,
        } => {
            let netrc = match netrc {
                Some(netrc) => Some(
                    read_to_string(&netrc)
                        .context("Could not read netrc file")
                        .attach(format!("File: {}", &netrc.to_string_lossy()))?,
                ),
                None => None,
            };
            server::update(
                &url,
                &get_secret_key(&httpsig_key)?,
                &api::HostUpdateRequest {
                    hosts: HashMap::from([(host, store_path)]),
                    public_key,
                    substitutor,
                    netrc,
                },
            )
            .await?;
        }
        ServerCommands::VerifyStatus => {
            let status = server::is_host_verified(&url, &get_secret_key(&httpsig_key)?).await?;
            info!("{status}");
        }
        ServerCommands::AddVerification {
            store_path,
            public_key,
            facter,
        } => {
            let nixos_facter = if let Some(facter) = facter {
                Some(read_to_string(facter)?)
            } else {
                None
            };

            let code = server::add_verification_attempt(
                &url,
                &api::VerificationAttempt {
                    store_path,
                    key: get_verify_key(&public_key)?,
                    artifacts: api::VerificationArtifacts { nixos_facter },
                },
            )
            .await?;
            info!("{code}");
        }
        ServerCommands::VerifyAttempt { name, code, facter } => {
            let artifacts = server::verify_attempt(
                &url,
                &get_secret_key(&httpsig_key)?,
                &api::VerificationAcceptance {
                    code,
                    host_name: name,
                },
            )
            .await?;
            if let Some(nixos_facter) = artifacts.nixos_facter {
                File::create_new(&facter)?.write_all(nixos_facter.as_bytes())?;
                info!("File {} written", facter.as_os_str().display());
            }
        }
        ServerCommands::AddKey { key, admin } => {
            let level = if admin == AuthLevel::Admin {
                api::AuthLevel::Admin
            } else {
                api::AuthLevel::Build
            };
            let status = server::add_key(
                &url,
                &get_secret_key(&httpsig_key)?,
                &api::AddKey {
                    key: get_verify_key(&key)?,
                    level,
                },
            )
            .await?;
            info!("{status}");
        }
        ServerCommands::RemoveKey { key } => {
            let status =
                server::remove_key(&url, &get_secret_key(&httpsig_key)?, &get_verify_key(&key)?)
                    .await?;
            info!("{status}");
        }
    }
    Ok(())
}
