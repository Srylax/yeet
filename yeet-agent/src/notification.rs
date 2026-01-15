use std::fs;

use rootcause::Report;
use tokio::process::Command;
use yeet::nix;

pub fn notify() -> Result<(), Report> {
    let variant = nix::nixos_variant_name()?;

    notify_rust::Notification::new()
        .summary("System Update")
        .body(&format!("System has been updated to `{variant}`"))
        .appname("Yeet")
        .show()?;
    Ok(())
}

pub fn notify_all() -> Result<(), Report> {
    let user_dirs = {
        let dirs = fs::read_dir("/run/user")?;
        dirs.flatten()
            // .into_iter()
            .map(|d| d.path())
            .flat_map(|p| p.file_name().map(|s| s.to_string_lossy().to_string()))
            .flat_map(|f| f.parse::<u32>())
    };

    for user in user_dirs {
        let dbus_address = format!("unix:path=/run/user/{user}/bus");
        let current_exe = std::env::current_exe().unwrap_or_else(|_| "yeet".into());
        let _ = Command::new(current_exe)
            .arg("notify")
            .uid(user)
            .env("DBUS_SESSION_BUS_ADDRESS", &dbus_address)
            // .env("DISPLAY", ":0")
            .spawn();
    }
    Ok(())
}
