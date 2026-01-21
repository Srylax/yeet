use console::style;
use rootcause::Report;
use yeet::server;

use crate::{
    cli_args::Config,
    section::{self, DisplaySection as _, DisplaySectionItem as _},
    sig::ssh,
    varlink,
};

pub async fn hosts(config: &Config, full: bool) -> Result<(), Report> {
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

    let hosts_section: Vec<(String, Vec<(String, String)>)> = {
        let mut hosts = server::get_hosts(&url, secret_key).await?;
        hosts.sort_by_key(|h| h.name.clone());

        if full {
            let hostnames = hosts.iter().map(|h| h.name.clone()).collect();
            let selected =
                inquire::MultiSelect::new("Which hosts do you want to display>", hostnames)
                    .prompt()?;
            hosts.retain(|h| selected.contains(&h.name));
        }

        if full {
            hosts.into_iter().map(|h| h.as_section()).collect()
        } else {
            vec![(
                style("Hosts:").underlined().to_string(),
                hosts.into_iter().map(|h| h.as_section_item()).collect(),
            )]
        }
    };

    section::print_sections(&hosts_section);

    Ok(())
}
