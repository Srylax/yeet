use std::{
    collections::HashMap,
    io,
    path::Path,
    process::{Command, Stdio},
};

use anyhow::{Ok, anyhow, bail};
use serde_json::Value;

// This command is used to run the virtual machine of a particular system
// WARNING: currently is just shelling out. In future we need to valide if
// the system is in the flake or not
// TODO: split build and run into different parts
pub fn run_vm(flake_path: &Path, system: &str) -> anyhow::Result<()> {
    let flake_path = flake_path.canonicalize()?; // Maybe check if its a dir and if it contains a flake.nix
    let flake_path = flake_path.to_string_lossy();
    let build_output = Command::new("nix")
        .arg("build")
        .arg(format!(
            "{flake_path}#nixosConfigurations.{system}.config.formats.vm",
        ))
        .stderr(io::stderr())
        .stdout(io::stdout())
        .spawn()?
        .wait()?;
    if !build_output.success() {
        bail!("Could not build the Virtual Machine");
    }
    Command::new(format!("{flake_path}/result/run-nixos-vm"))
        .stderr(io::stderr())
        .stdout(io::stdout())
        .spawn()?
        .wait()?;
    Ok(())
}

// TODO: build multiple hosts at once
pub fn build_hosts(
    flake_path: &str,
    hosts: Vec<String>,
    darwin: bool,
) -> anyhow::Result<HashMap<String, String>> {
    let mut found_hosts = list_hosts(flake_path, darwin)?;
    // If empty build all
    if !hosts.is_empty() {
        found_hosts.retain_mut(|host| hosts.contains(host));
    }
    let mut closures = HashMap::with_capacity(found_hosts.len());
    for ref host in found_hosts {
        let system = if darwin {
            format!("{flake_path}#darwinConfigurations.{host}.system")
        } else {
            format!("{flake_path}#nixosConfigurations.{host}.config.system.build.toplevel")
        };
        let output = Command::new("nix")
            .args(["build", "--json", "--no-link", "--"])
            .arg(&system)
            .stdout(Stdio::piped())
            .spawn()?
            .wait_with_output()?;
        let build = serde_json::from_slice::<Value>(&output.stdout)?;
        let closure = build[0]["outputs"]["out"]
            .as_str()
            .ok_or(anyhow!("Build output did not contain a valid closure"))?;
        closures.insert(host.clone(), closure.to_owned());
    }
    Ok(closures)
}

pub fn list_hosts(flake_path: &str, darwin: bool) -> anyhow::Result<Vec<String>> {
    let flavor = if darwin {
        "darwinConfigurations"
    } else {
        "nixosConfigurations"
    };
    let output = Command::new("nix")
        .arg("eval")
        .arg(format!("{flake_path}#{flavor}",))
        .args(["--apply", "builtins.attrNames", "--json"])
        .stdout(Stdio::piped())
        .spawn()?
        .wait_with_output()?;

    Ok(serde_json::from_slice(&output.stdout)?)
}
