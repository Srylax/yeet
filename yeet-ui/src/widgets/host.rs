use egui::{Button, Color32, Frame, Stroke};
use jiff::{SignedDuration, Unit, Zoned};
use yeet_api::Host;

use crate::tools::NotifyFailure as _;

pub fn host_widget(ui: &mut egui::Ui, host: &Host) {
    // .horizontal(|ui| ping(ui, host));
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
            ui.add_enabled(false, Button::new(format!("{diff:#}")).fill(color))
        }
        None => ui.add_enabled(false, Button::new("Never")),
    }
}
