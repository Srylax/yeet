use std::{fmt::Display, os::linux::raw::stat, path::Path};

use api::key::get_secret_key;
use console::style;
use log::warn;
use rootcause::Report;
use url::Url;
use yeet::{display, nix, server};

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

pub async fn local_status(url: &Url, httpsig_key: &Path) -> Result<String, Report> {
    let nixos_version = nix::nixos_version()?;
    log::warn!("hi!");
    status!(
        style("Yeet:").underlined() => [
            "Up to date", style("yes").green().bold(),
            "Mode", format!("{} ({})",style("Provisioned").green().bold(),style("https://yeet.bsiag.com").underlined()),
            "Daemon", format!("{} since Mon 2026-01-05 14:59:13 CET; 15h ago", style("active (running)").green().bold()),
            "Version", "0.2.0",
        ],
        style("System:").underlined() => [
            "Kernel", "6.12.63",
            "Firmware Arch", "x64",
            "NixOS version", nixos_version.nixos_version,
            "Switch date", "└─2026-01-05 18:03:16; Xh ago",
            "Conf version", nixos_version.configuration_revision,
            "Nixpkgs version", nixos_version.nixpkgs_revision,
        ],
    );
    todo!()
}

fn local_version_info() -> Result<String, Report> {
    todo!()
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
            // Check for empty key (used for list continuation)
            if key.to_string().is_empty() {
                // {:>w$} prints empty padding of size 'w'
                // followed by 2 spaces (to replace ": ")
                println!("{:>w$}  {}", "", value, w = width);
            } else {
                println!("{:>w$}: {}", key, value, w = width);
            }
        }
        println!(); // Blank line after section
    }
}
