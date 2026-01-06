use std::{fmt::Display, path::Path};

use api::key::get_secret_key;
use console::style;
use rootcause::{Report, prelude::ResultExt};
use serde::{Deserialize, Serialize};
use url::Url;
use yeet::{display, nix, server};

shadow_rs::shadow!(build);

// warning: only displaying local information if you want status information run `yeet` with root permission
// System:
//  Kernel: 6.12.63
//  Firmware Arch: x86
//  NixOS version: 25.11..3202.30a3c519afcf Generation 26
//  Build-date:    └─2026-01-05 18:03:16
//  Configuration version: 6afe6c9ab638d92a9506e837395e399e6144f10a
//  Nixpkgs version: 30a3c519afcf3f99e2c6df3b359aec5692054d92
//  Up to date: yes
//  Mode: Provisioned (https://yeet.bsiag.com)
//  ✓ ✗

macro_rules! status {
    ( $( $title:expr => [ $( $k:expr, $v:expr ),* $(,)? ] ),* $(,)? ) => {
        let status: &[(&dyn Display, &[(&dyn Display, &dyn Display)])] = &[ $( ( &$title as &dyn Display, &[ $( (&$k as &dyn Display, &$v as &dyn Display) ),* ] ) ),* ];
        print_status(status);
    };
}

pub async fn local_status(url: &Url, httpsig_key: &Path) -> Result<(), Report> {
    let local_version = local_version_info().context("Could note get local version information")?;

    let last_switch = &local_version.build_date.until(jiff::Zoned::now())?.round(
        jiff::SpanRound::new()
            .smallest(jiff::Unit::Minute)
            .mode(jiff::RoundMode::Trunc),
    )?;

    let last_switch = if last_switch.total(jiff::Unit::Hour)? < 24f64 {
        style(last_switch).green()
    } else {
        style(last_switch).red()
    };

    let os_version = if local_version.nixos_version.starts_with("dirty") {
        style(&local_version.nixos_version).red()
    } else {
        style(&local_version.nixos_version).green()
    };

    log::warn!("hi!");
    status!(
        style("Yeet:").underlined() => [
            "Up to date!", style("yes").green().bold(),
            "Mode!", format!("{} ({})",style("Provisioned").green().bold(),style("https://yeet.bsiag.com").underlined()),
            "Daemon!", format!("{} since Mon 2026-01-05 14:59:13 CET; 15h ago", style("active (running)").green().bold()),
            "Version", format!("{}", build::CLAP_LONG_VERSION),
        ],
        style("System:").underlined() => [
            "Kernel", local_version.kernel,
            "NixOS version", format!("{} Generation {}", os_version, local_version.current_generation),
            "Build date", format!("└─{}; {:#} ago",local_version.build_date, last_switch),
            "Variant", &nix::nixos_variant_name()?,
            "Conf revision", &local_version.configuration_revision[..8],
            "Nixpkgs version", &local_version.nixpkgs_revision[..8],
        ],
    );
    Ok(())
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

fn print_status(status: &[(&dyn Display, &[(&dyn Display, &dyn Display)])]) {
    let width = status
        .iter()
        .flat_map(|(_, k)| *k)
        .map(|(k, _)| k.to_string().len())
        .max()
        .unwrap_or(0);
    for (section, items) in status {
        println!("{section}");

        for (key, value) in *items {
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
