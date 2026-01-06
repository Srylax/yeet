use std::process::{Command, Stdio};

use rootcause::{Report, prelude::ResultExt as _};

pub fn systemd_status_value(
    value: impl AsRef<str>,
    service: impl AsRef<str>,
) -> Result<Option<String>, Report> {
    let output = Command::new("systemctl")
        .arg("status")
        .arg("--no-pager")
        .arg(service.as_ref())
        .env("SYSTEMD_COLORS", "1")
        .stdout(Stdio::piped())
        .spawn()
        .context("Could not spawn `grep`")?
        .wait_with_output()?;

    let prefix = format!("{}: ", value.as_ref());

    let output = String::from_utf8_lossy(&output.stdout).to_string();
    let line = output
        .lines()
        .into_iter()
        .map(|l| l.trim())
        .filter_map(|l| l.strip_prefix(prefix.as_str()))
        .next();
    Ok(line.map(|s| s.to_owned()))
}
