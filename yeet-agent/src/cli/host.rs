use console::style;
use log::info;
use rootcause::Report;
use yeet::server;

use crate::{cli_args::Config, sig::ssh, varlink};

pub async fn remove(config: &Config, hostname: Option<String>) -> Result<(), Report> {
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

    let secret_key = {
        let domain = url
            .domain()
            .ok_or(rootcause::report!("Provided URL has no domain part"))?;
        &ssh::key_by_url(domain)?
    };

    let hostname = if let Some(hostname) = hostname {
        hostname
    } else {
        let hostnames = {
            let hosts = server::get_hosts(&url, secret_key).await?;
            let mut hostnames: Vec<_> = hosts.iter().map(|h| h.name.clone()).collect();
            hostnames.sort();
            hostnames
        };
        let selected =
            inquire::Select::new("Which host do you want to delete>", hostnames).prompt()?;
        selected
    };

    // The user has to confirm the action
    let confirm = inquire::Confirm::new(
        &style(format!(
            "Are you sure you want to delete {hostname}. This action is not reversable"
        ))
        .red()
        .to_string(),
    )
    .with_default(false)
    .prompt()?;

    if !confirm {
        info!("Aborting...");
        return Ok(());
    }

    info!("Deleting {hostname}...");

    // no takies backsies past this point

    server::remove_host(&url, secret_key, &api::HostRemoveRequest { hostname }).await?;

    info!("Deleted!");

    Ok(())
}

pub async fn rename(
    config: &Config,
    current_name: Option<String>,
    new_name: Option<String>,
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

    let secret_key = {
        let domain = url
            .domain()
            .ok_or(rootcause::report!("Provided URL has no domain part"))?;
        &ssh::key_by_url(domain)?
    };

    let current_name = if let Some(current_name) = current_name {
        current_name
    } else {
        let hostnames = {
            let hosts = server::get_hosts(&url, secret_key).await?;
            let mut hostnames: Vec<_> = hosts.into_iter().map(|h| h.name).collect();
            hostnames.sort();
            hostnames
        };

        let selected =
            inquire::Select::new("Which host do you want to rename>", hostnames).prompt()?;
        selected
    };

    let new_name = if let Some(new_name) = new_name {
        new_name
    } else {
        inquire::Text::new("What should the new name be?").prompt()?
    };

    // The user has to confirm the action
    let confirm = inquire::Confirm::new(&format!(
        "Are you sure you want to rename {current_name} to {new_name}."
    ))
    .with_default(false)
    .prompt()?;

    if !confirm {
        info!("Aborting...");
        return Ok(());
    }

    info!("Renaming {current_name} to {new_name}...");

    server::rename_host(
        &url,
        secret_key,
        &api::HostRenameRequest {
            new_name,
            current_name,
        },
    )
    .await?;

    info!("Done!");

    Ok(())
}
