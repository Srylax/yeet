use std::{
    fs::File,
    io::Write as _,
    path::{Path, PathBuf},
};

use inquire::validator::Validation;
use log::info;
use rootcause::Report;
use yeet::server;

use crate::{cli_args::Config, sig::ssh, varlink};

pub async fn approve(
    config: &Config,
    facter_output: Option<PathBuf>,
    code: Option<u32>,
    hostname: Option<String>,
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

    let hostname = {
        if let Some(hostname) = hostname {
            hostname
        } else {
            inquire::Text::new("Hostname:").prompt()?
        }
    };

    let code = {
        if let Some(code) = code {
            code
        } else {
            inquire::CustomType::<u32>::new("Approval code:").prompt()?
        }
    };

    info!("Approving {hostname} with code {code}...");

    let artifacts = server::verify_attempt(
        &url,
        secret_key,
        &api::VerificationAcceptance { code, hostname },
    )
    .await?;

    info!("Approved");

    if artifacts.nixos_facter.is_none() {
        return Ok(());
    }
    let nixos_facter = artifacts.nixos_facter.unwrap();

    // Get file to write facter data
    let facter_output = {
        if let Some(facter_output) = facter_output {
            facter_output
        } else {
            let output = inquire::Text::new("Facter Output:")
                .with_validator(|path: &str| {
                    let Some(parent_dir) = Path::new(path).parent() else {
                        return Ok(Validation::Invalid("Not a directory".into()));
                    };

                    if !parent_dir.exists() {
                        return Ok(Validation::Invalid("Directory does not exist".into()));
                    }

                    if Path::new(path).exists() {
                        return Ok(Validation::Invalid(
                            format!("{path} already exists!").into(),
                        ));
                    }

                    Ok(Validation::Valid)
                })
                .prompt()?;
            PathBuf::from(output)
        }
    };

    File::create_new(&facter_output)?.write_all(nixos_facter.as_bytes())?;
    info!("File {} written", facter_output.as_os_str().display());
    Ok(())
}
