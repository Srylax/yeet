use std::collections::HashMap;

use log::info;
use yeet_agent::{display::diff_inline, server};

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
                api::ProvisionState::Provisioned(api::RemoteStorePath {
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
            info!("{}", diff_inline(&before, &after));
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
            info!("{}", diff_inline(&before, &after));
        }

        ServerCommands::VerifyStatus => {
            let status =
                server::is_host_verified(&config.url, &get_sig_key(&config.httpsig_key)?).await?;
            info!("{status}");
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
            info!("{code}");
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
            info!("{status}");
        }
    }
    Ok(())
}
