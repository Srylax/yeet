use console::{StyledObject, style};
use jiff::tz::TimeZone;
use yeet::display;

use crate::section::{ColoredDisplay, DisplaySection, DisplaySectionItem};

impl ColoredDisplay<&str> for api::ProvisionState {
    fn colored_display(&self) -> StyledObject<&'static str> {
        match self {
            api::ProvisionState::NotSet => style("Not set").blue(),
            api::ProvisionState::Detached(_) => style("Detached").yellow(),
            api::ProvisionState::Provisioned(_remote_store_path) => style("Provisioned").green(),
        }
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

impl DisplaySection for api::Host {
    fn as_section(&self) -> crate::section::Section {
        let mut items = Vec::new();

        if let api::ProvisionState::Provisioned(version) = &self.provision_state {
            let up_to_date = if &version.store_path == self.latest_store_path() {
                style("Yes").green().bold()
            } else {
                style("No").red().bold()
            };
            items.push(("Up to date".to_string(), up_to_date.to_string()));
        }

        items.push((
            "Mode".to_string(),
            self.provision_state.colored_display().bold().to_string(),
        ));

        items.push((
            "Current version".to_string(),
            self.latest_store_path().to_string(),
        ));

        if let api::ProvisionState::Provisioned(ref remote_store) = self.provision_state
            && remote_store.store_path != *self.latest_store_path()
        {
            items.push(("Next version".to_string(), remote_store.store_path.clone()));
        }

        {
            let last_seen = display::time_diff(
                &self.last_ping.with_time_zone(TimeZone::system()),
                jiff::Unit::Second,
                30_f64,
                jiff::Unit::Second,
            );
            items.push(("Last seen".to_string(), last_seen.to_string()));
        }

        (style(&self.name).underlined().to_string(), items)
    }
}
