use std::{fmt::Display, path::Path};

use api::key::get_secret_key;
use console::style;
use jiff::tz::TimeZone;
use rootcause::{Report, prelude::ResultExt as _};
use serde::{Deserialize, Serialize};
use url::Url;
use yeet::{display, nix, server};

use crate::{
    section::{self, DisplaySection, Section, section},
    systemd,
    varlink::{self},
};

shadow_rs::shadow!(build);

pub async fn status(json: bool) -> Result<(), Report> {
    let yeet = yeet_info().await?;
    let system = system_info()?;
    if json {
        println!("{}", serde_json::to_string(&Status { system, yeet })?);
    } else {
        section::print_sections(&[yeet.as_section(), system.as_section()]);
    }
    Ok(())
}

#[derive(Serialize, Deserialize)]
struct Status {
    system: SystemInfo,
    yeet: YeetInfo,
}

#[derive(Serialize, Deserialize)]
struct YeetInfo {
    pub systemd_status: String,
    pub daemon_status: Option<varlink::DaemonStatus>,
    pub cli_version_short: String,
    pub cli_version_long: String,
}

impl Display for varlink::DaemonMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mode = match &self {
            Self::Provisioned => style("Provisioned").green().bold(),
            Self::Detached => style("Detached").yellow().bold(),
            Self::NetworkError => style("NetworkError").red().bold(),
            Self::Unverified => style("Unverified").red().bold(),
        };
        write!(f, "{mode}")
    }
}

impl Display for varlink::UpToDate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let up_to_date = match &self {
            Self::Yes => style("Yes").green().bold(),
            Self::No => style("No").red().bold(),
            Self::Detached => style("Detached").yellow().bold(),
        };
        write!(f, "{up_to_date}")
    }
}

impl DisplaySection for YeetInfo {
    fn as_section(&self) -> Section {
        let (up_to_date, mode, daemon_version) = match &self.daemon_status {
            Some(daemon_state) => {
                let up_to_date = daemon_state.up_to_date.to_string();
                let mode = format!(
                    "{} ({})",
                    daemon_state.mode,
                    style(daemon_state.server.to_string()).underlined()
                );
                (up_to_date, mode, daemon_state.version.clone())
            }
            None => {
                let no_con = style("No connection to daemon").red().bold().to_string();
                (no_con.clone(), no_con.clone(), no_con)
            }
        };

        let daemon_version = if daemon_version != self.cli_version_short {
            style(daemon_version).red().bold()
        } else {
            style(daemon_version)
        };

        section!(
            style("Yeet:").underlined() => [
                "Up to date", up_to_date,
                "Mode", mode,
                "Systemd Unit", self.systemd_status,
                "Daemon version", daemon_version,
                "CLI Version", format!("{}", self.cli_version_long),
            ]
        )
    }
}

async fn yeet_info() -> Result<YeetInfo, Report> {
    let daemon_status = match varlink::status().await {
        Ok(status) => Some(status),
        Err(err) => {
            log::error!("Could not get status from daemon:\n{err}");
            None
        }
    };

    Ok(YeetInfo {
        cli_version_short: String::from(build::PKG_VERSION),
        cli_version_long: String::from(build::CLAP_LONG_VERSION),
        daemon_status: daemon_status,
        systemd_status: systemd::systemd_status_value("Active", "yeet")?
            .unwrap_or("Service health not found".to_owned()),
    })
}

#[derive(Serialize, Deserialize)]
struct SystemInfo {
    pub kernel: String,
    pub nixos_version: String,
    pub build_date: jiff::civil::DateTime,
    pub variant: String,
    pub configuration_revision: String,
    pub nixpkgs_revision: String,
    pub current_generation: u32,
}

impl DisplaySection for SystemInfo {
    fn as_section(&self) -> Section {
        let build_date_span = {
            let zoned = self.build_date.to_zoned(TimeZone::system()).unwrap();
            let last_build = (&zoned - &jiff::Zoned::now())
                .round(
                    jiff::SpanRound::new()
                        .smallest(jiff::Unit::Minute)
                        .mode(jiff::RoundMode::Trunc),
                )
                .unwrap();

            if last_build.total(jiff::Unit::Hour).unwrap() < 24_f64 {
                style(last_build).green().bold()
            } else {
                style(last_build).red().bold()
            }
        };

        let os_version = if self.nixos_version.starts_with("dirty") {
            style(&self.nixos_version).red().bold()
        } else {
            style(&self.nixos_version).green()
        };

        let variant = if self.variant.starts_with("dirty") {
            style(&self.variant).red().bold()
        } else {
            style(&self.variant).bold()
        };

        section!(
            style("System:").underlined() => [
                "Kernel", self.kernel,
                "NixOS version", format!("{} Generation {}", os_version, style(self.current_generation).bold()),
                "Build date", format!("\u{2514}\u{2500}{}; {:#} ago",self.build_date, build_date_span),
                "Variant", variant,
                "Conf revision", self.configuration_revision[..8],
                "Nixpkgs version", self.nixpkgs_revision[..8],
            ]
        )
    }
}

fn system_info() -> Result<SystemInfo, Report> {
    let nixos_version = nix::nixos_version().context("Could not fetch nixos version")?;
    let nixos_generations =
        nix::nixos_generations().context("Could not fetch nixos generations")?;
    let generation = nixos_generations
        .into_iter()
        .find(|g| g.current)
        .unwrap_or_default();

    Ok(SystemInfo {
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
    let rows = {
        let status = server_status(url, httpsig_key).await?;
        status
            .into_iter()
            .map(|host| display::host(&host))
            .collect::<Result<Vec<_>, Report>>()?
    };

    Ok(rows.join("\n"))
}
