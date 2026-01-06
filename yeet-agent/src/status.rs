use std::{fmt::Display, path::Path};

use api::{AgentAction, key::get_secret_key};
use console::style;
use httpsig_hyper::prelude::SecretKey;
use jiff::tz::TimeZone;
use rootcause::{Report, prelude::ResultExt as _};
use serde::{Deserialize, Serialize};
use url::Url;
use yeet::{display, nix, server};

use crate::{
    section::{DisplaySection, Section, section},
    systemd,
    varlink::{self, YeetProxy},
    version,
};

shadow_rs::shadow!(build);

pub async fn status(url: &Url, key: &SecretKey, json: bool, local: bool) -> Result<(), Report> {
    let a = varlink::client().await?.status().await?;
    println!("{a:?}");

    // let yeet = yeet_info(url, key, local).await?;
    // let system = system_info()?;
    // if json {
    //     println!("{}", serde_json::to_string(&Status { system, yeet })?);
    // } else {
    //     section::print_sections(&[yeet.as_section(), system.as_section()]);
    // }
    Ok(())
}

#[derive(Serialize, Deserialize)]
struct Status {
    system: SystemInfo,
    yeet: YeetInfo,
}

#[derive(Serialize, Deserialize)]
struct YeetInfo {
    pub up_to_date: UpToDate,
    pub server: url::Url,
    pub mode: YeetClientMode,
    pub version_short: String,
    pub version_long: String,
    pub daemon_status: String,
}

#[derive(Serialize, Deserialize)]
enum YeetClientMode {
    Provisioned,
    Detached,
    Unknown,
}

impl Display for YeetClientMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mode = match &self {
            YeetClientMode::Provisioned => style("Provisioned").green().bold(),
            YeetClientMode::Detached => style("Detached").yellow().bold(),
            YeetClientMode::Unknown => style("Unknown").red().bold(),
        };
        write!(f, "{mode}")
    }
}

#[derive(Serialize, Deserialize)]
enum UpToDate {
    Yes,
    No(api::StorePath),
    Detached,
    Unknown,
}
impl Display for UpToDate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let up_to_date = match &self {
            UpToDate::Yes => style("Yes").green().bold(),
            UpToDate::No(_) => style("No").red().bold(),
            UpToDate::Detached => style("Detached").yellow().bold(),
            UpToDate::Unknown => style("Unknown").red().bold(),
        };
        write!(f, "{up_to_date}")
    }
}

impl DisplaySection for YeetInfo {
    fn as_section(&self) -> Section {
        section!(
            style("Yeet:").underlined() => [
                "Up to date", self.up_to_date,
                "Mode", format!("{} ({})", self.mode, style(&self.server).underlined()),
                "Daemon", self.daemon_status,
                "Version", format!("{}", self.version_long),
            ]
        )
    }
}

async fn yeet_info(url: &Url, key: &SecretKey, local: bool) -> Result<YeetInfo, Report> {
    let mode;
    let up_to_date;
    if local {
        mode = YeetClientMode::Unknown;
        up_to_date = UpToDate::Unknown;
    } else {
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

        up_to_date = match &remote_state {
            Ok(AgentAction::Nothing) => UpToDate::Yes,
            Ok(AgentAction::Detach) => UpToDate::Detached,
            Ok(AgentAction::SwitchTo(store)) => UpToDate::No(store.store_path.clone()),
            Err(_) => UpToDate::Unknown,
        };

        mode = match &remote_state {
            Ok(AgentAction::Nothing) => YeetClientMode::Provisioned,
            Ok(AgentAction::SwitchTo(_)) => YeetClientMode::Provisioned,
            Ok(AgentAction::Detach) => YeetClientMode::Detached,
            Err(_) => YeetClientMode::Unknown,
        };
    }

    Ok(YeetInfo {
        up_to_date,
        server: url.clone(),
        mode,
        version_short: String::from(build::PKG_VERSION),
        version_long: String::from(build::CLAP_LONG_VERSION),
        daemon_status: systemd::systemd_status_value("Active", "yeet")?
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
        let zoned = self.build_date.to_zoned(TimeZone::system()).unwrap();

        let last_switch = (&zoned - &jiff::Zoned::now())
            .round(
                jiff::SpanRound::new()
                    .smallest(jiff::Unit::Minute)
                    .mode(jiff::RoundMode::Trunc),
            )
            .unwrap();

        let last_switch = if last_switch.total(jiff::Unit::Hour).unwrap() < 24_f64 {
            style(last_switch).green().bold()
        } else {
            style(last_switch).red().bold()
        };

        let os_version = if self.nixos_version.starts_with("dirty") {
            style(&self.nixos_version).red().bold()
        } else {
            style(&self.nixos_version).green()
        };

        section!(
            style("System:").underlined() => [
                "Kernel", self.kernel,
                "NixOS version", format!("{} Generation {}", os_version, style(self.current_generation).bold()),
                "Build date", format!("\u{2514}\u{2500}{}; {:#} ago",self.build_date, last_switch),
                "Variant", style(&self.variant).bold(),
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
    let status = server_status(url, httpsig_key).await?;
    let rows = status
        .into_iter()
        .map(|host| display::host(&host))
        .collect::<Result<Vec<_>, Report>>()?;

    Ok(rows.join("\n"))
}
