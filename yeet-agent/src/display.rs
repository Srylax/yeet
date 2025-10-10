use std::io::Write as _;

use anyhow::ensure;
use api::hash_hex;
use jiff::Zoned;

pub fn host(host: &api::Host) -> anyhow::Result<String> {
    let status_emoji = match host.status {
        api::HostState::New => "✨",
        api::HostState::Detached => "🫥",
        api::HostState::Provisioned(api::ProvisionState::UpToDate) => "✅",
        api::HostState::Provisioned(api::ProvisionState::NewVersionAvailable(_)) => "🔄",
    };

    let status = match host.status {
        api::HostState::New => " (New)",
        api::HostState::Detached => " (Detached)",
        api::HostState::Provisioned(api::ProvisionState::UpToDate) => " (Latest)",
        api::HostState::Provisioned(api::ProvisionState::NewVersionAvailable(_)) => {
            " (Will Update)"
        }
    };

    let mut w = Vec::new();
    writeln!(&mut w, "[{status_emoji}] {}{status}", host.name)?;

    if host.store_path.is_empty() {
        ensure!(host.last_ping == None);
        writeln!(&mut w, " • Version: Host not rolled out ⏳",)?;
    } else {
        writeln!(&mut w, " • Version: {}", hash_hex(&host.store_path))?;
    }

    if let api::HostState::Provisioned(api::ProvisionState::NewVersionAvailable(ref next_version)) =
        host.status
    {
        writeln!(
            &mut w,
            " • Next Version: {}",
            hash_hex(&next_version.store_path)
        )?;
    }

    writeln!(
        &mut w,
        " • Last Seen: {}",
        host.last_ping
            .clone()
            .map_or("Never ⏳".to_owned(), |zoned| {
                format!("{:#}", &Zoned::now() - &zoned)
            })
    )?;

    Ok(String::from_utf8(w)?)
}
