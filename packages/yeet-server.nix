{ pkgs, ... }:
let
  manifest = (pkgs.lib.importTOML ../yeet-server/Cargo.toml).package;
in
pkgs.rustPlatform.buildRustPackage {
  pname = manifest.name;
  version = manifest.version;
  cargoLock.lockFile = ./Cargo.lock;
  src = ./.;
  buildAndTestSubdir = "yeet-server";
}
