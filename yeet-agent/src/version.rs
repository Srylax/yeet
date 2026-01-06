use rootcause::{Report, prelude::ResultExt as _};
use std::fs::read_link;

pub fn get_active_version() -> Result<String, Report> {
    Ok(read_link("/run/current-system")
        .context("Current system has no `/run/current-system`")?
        .to_string_lossy()
        .to_string())
}
