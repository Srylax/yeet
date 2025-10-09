use std::{io, path::Path, process::Command};

use anyhow::{Ok, bail};

// This command is used to run the virtual machine of a particular system
// WARNING: currently is just shelling out. In future we need to valide if
// the system is in the flake or not
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
