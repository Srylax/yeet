use std::{fmt::Display, path::Path};

use api::{AgentAction, key::get_secret_key};
use console::style;
use httpsig_hyper::prelude::SecretKey;
use rootcause::{Report, prelude::ResultExt};
use serde::{Deserialize, Serialize};
use url::Url;
use yeet::{display, nix, server};

use crate::{systemd, version};

shadow_rs::shadow!(build);

type Section = (String, Vec<(String, String)>);

macro_rules! section {
    ( $title:expr => [ $( $k:expr, $v:expr ),* $(,)? ] ) => {
        ( $title.to_string(),
            vec![ $( ($k.to_string(), $v.to_string()) ),*]
        )
    };
}

async fn print_yeet_info(url: &Url, key: &SecretKey) -> Result<Section, Report> {
    let remote_state = server::system_check(
        url,
        key,
        &api::VersionRequest {
            store_path: version::get_active_version()?,
        },
    )
    .await;
    if let Err(err) = &remote_state {
        println!("{err}");
        log::error!(
            "Could not retrieve remote state. If you want to only display local consider using `--local`"
        );
    }

    let up_to_date = match &remote_state {
        Ok(AgentAction::Nothing) => style("yes").green().bold(),
        Ok(AgentAction::Detach) => style("detached").yellow().bold(),
        Ok(AgentAction::SwitchTo(_)) => style("no").red().bold(),
        Err(_) => style("Unknown").red().bold(),
    };

    let mode = match &remote_state {
        Ok(AgentAction::Nothing) => style("Provisioned").green().bold(),
        Ok(AgentAction::Detach) => style("Detached").yellow().bold(),
        Ok(AgentAction::SwitchTo(_)) => style("Provisioned").green().bold(),
        Err(_) => style("Unknown").red().bold(),
    };

    Ok(section!(
        style("Yeet:").underlined() => [
            "Up to date", up_to_date,
            "Mode", format!("{} ({})",mode,style("https://yeet.bsiag.com").underlined()),
            "Daemon", &systemd::systemd_status_value("Active","yeet")?.unwrap_or("Service health not found".to_owned()),
            "Version", format!("{}", build::CLAP_LONG_VERSION),
        ]
    ))
}

pub async fn status(url: &Url, key: &SecretKey) -> Result<(), Report> {
    let yeet_section = print_yeet_info(url, key).await?;
    let system_section = system_info()?;
    print_status(&[yeet_section, system_section]);
    Ok(())
}

fn system_info() -> Result<Section, Report> {
    let local_version = local_version_info().context("Could note get local version information")?;

    let last_switch = &local_version.build_date.until(jiff::Zoned::now())?.round(
        jiff::SpanRound::new()
            .smallest(jiff::Unit::Minute)
            .mode(jiff::RoundMode::Trunc),
    )?;

    let last_switch = if last_switch.total(jiff::Unit::Hour)? < 24f64 {
        style(last_switch).green().bold()
    } else {
        style(last_switch).red().bold()
    };

    let os_version = if local_version.nixos_version.starts_with("dirty") {
        style(&local_version.nixos_version).red().bold()
    } else {
        style(&local_version.nixos_version).green()
    };
    Ok(section!(
        style("System:").underlined() => [
            "Kernel", local_version.kernel,
            "NixOS version", format!("{} Generation {}", os_version, local_version.current_generation),
            "Build date", format!("└─{}; {:#} ago",local_version.build_date, last_switch),
            "Variant", style(nix::nixos_variant_name()?).bold(),
            "Conf revision", &local_version.configuration_revision[..8],
            "Nixpkgs version", &local_version.nixpkgs_revision[..8],
        ]
    ))
}

#[derive(Serialize, Deserialize)]
pub struct LocalVersionInfo {
    pub kernel: String,
    pub nixos_version: String,
    pub build_date: jiff::civil::DateTime,
    pub variant: String,
    pub configuration_revision: String,
    pub nixpkgs_revision: String,
    pub current_generation: u32,
}

fn local_version_info() -> Result<LocalVersionInfo, Report> {
    let nixos_version = nix::nixos_version().context("Could not fetch nixos version")?;
    let nixos_generations =
        nix::nixos_generations().context("Could not fetch nixos generations")?;
    let generation = nixos_generations
        .into_iter()
        .find(|g| g.current)
        .unwrap_or_default();

    Ok(LocalVersionInfo {
        kernel: generation.kernel_version,
        nixos_version: generation.nixos_version,
        build_date: generation.date,
        variant: nix::nixos_variant_name()?,
        configuration_revision: generation.configuration_revision,
        nixpkgs_revision: nixos_version.nixpkgs_revision,
        current_generation: generation.generation,
    })
}

async fn server_status(url: &Url, httpsig_key: &Path) -> Result<Vec<api::Host>, Report> {
    server::status(url, &get_secret_key(httpsig_key)?).await
}

pub(crate) async fn status_string(url: &Url, httpsig_key: &Path) -> Result<String, Report> {
    let status = server_status(url, httpsig_key).await?;
    let rows = status
        .into_iter()
        .map(|host| display::host(&host))
        .collect::<Result<Vec<_>, Report>>()?;

    Ok(rows.join("\n"))
}

fn print_status(status: &[Section]) {
    let width = status
        .iter()
        .flat_map(|(_, k)| k)
        .map(|(k, _)| k.len())
        .max()
        .unwrap_or(0)
        + 1;
    for (section, items) in status {
        println!("{section}");

        for (key, value) in items {
            let value = value.to_string();
            // Test if it is a multiline
            if value.lines().count() > 1 {
                let mut lines = value.lines();
                // print first normally key: Value
                println!("{:>w$}: {}", key, lines.next().unwrap(), w = width);

                for line in lines {
                    println!("{:>w$}  {}", "", line, w = width);
                }
            } else {
                println!("{:>w$}: {}", key, value, w = width);
            }
        }
        println!(); // Blank line after section
    }
}
