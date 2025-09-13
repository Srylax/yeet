use std::ops::Rem;

use egui::{Color32, TextFormat, text::LayoutJob, util::hash};
use jiff::{SignedDuration, Unit, Zoned};
use yeet_api::{Host, VersionStatus};

use crate::tools::NotifyFailure as _;

pub fn host_widget(ui: &mut egui::Ui, host: &Host) {
    ui.group(|ui| {
        // ui.vertical_centered(|ui| ui.label(RichText::new("Name").heading()));
        ping(ui, host);

        let version = match host.status {
            yeet_api::VersionStatus::NewVersionAvailable(_) => {
                colored_pair("Version:", "New version available", Color32::YELLOW)
            }
            yeet_api::VersionStatus::UpToDate => {
                colored_pair("Version:", "Up to date", Color32::LIGHT_BLUE)
            }
        };

        ui.collapsing(version, |ui| {
            ui.label(format!(
                "Installed: {:x}",
                hash(&host.store_path).rem(10000)
            ));
            if let VersionStatus::NewVersionAvailable(ref new_version) = host.status {
                ui.label(format!(
                    "Latest: {:x}",
                    hash(&new_version.store_path).rem(10000)
                ));
                ui.label(format!("Nix Cache: {}", new_version.substitutor));
            } else {
                ui.label(format!("Latest: {:x}", hash(&host.store_path).rem(10000)));
            }
        })
    });
}

fn ping(ui: &mut egui::Ui, host: &Host) -> egui::Response {
    match host.last_ping {
        Some(ref ping) => {
            let diff = &Zoned::now().duration_since(ping);
            let diff = diff
                .round(Unit::Second)
                .toast()
                .unwrap_or(SignedDuration::from_secs(-1));
            let color = if diff.as_secs() < 30 {
                Color32::GREEN
            } else if diff.as_secs() < 60 {
                Color32::YELLOW
            } else {
                Color32::RED
            };
            ui.label(colored_pair(
                "Last Seen:",
                format!("{diff:#}").as_str(),
                color,
            ))
        }
        None => ui.label(colored_pair("Last Seen:", "Never", Color32::LIGHT_GRAY)),
    }
}

fn colored_pair(label: &str, value: &str, background: Color32) -> LayoutJob {
    let mut job = LayoutJob::default();
    job.append(label, 0.0, TextFormat::default());
    job.append(
        value,
        8.0,
        TextFormat {
            color: Color32::BLACK,
            background,
            ..Default::default()
        },
    );
    job
}
