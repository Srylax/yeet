use console::style;
use jiff::tz::TimeZone;
use yeet::display;

use crate::section::{ColoredDisplay, DisplaySectionItem};

impl ColoredDisplay for api::ProvisionState {
    fn colored_display(&self) -> String {
        match self {
            api::ProvisionState::NotSet => style("Not set").blue(),
            api::ProvisionState::Detached => style("Detached").yellow(),
            api::ProvisionState::Provisioned(_remote_store_path) => style("Provisioned").green(),
        }
        .to_string()
    }
}

impl DisplaySectionItem for api::Host {
    fn as_section_item(&self) -> (String, String) {
        let commit_sha = self
            .latest_store_path()
            .rfind('.')
            .map(|i| i + 1)
            .unwrap_or(0);

        let up_to_date = if let api::ProvisionState::Provisioned(version) = &self.provision_state {
            if &version.store_path == self.latest_store_path() {
                style("Up to date ").green()
            } else {
                style("Outdated   ").red()
            }
            .to_string()
        } else {
            String::new()
        };

        (
            self.name.clone(),
            format!(
                "{} ({}) {up_to_date}{}",
                self.provision_state.colored_display(),
                self.latest_store_path()[commit_sha..].to_owned(),
                display::time_diff(
                    &self.last_ping.with_time_zone(TimeZone::system()),
                    jiff::Unit::Second,
                    30_f64,
                    jiff::Unit::Second
                ),
            ),
        )
    }
}
