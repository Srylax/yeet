{ pkgs, ... }:
let
  manifest = (pkgs.lib.importTOML ../yeet-agent/Cargo.toml).package;
in
pkgs.rustPlatform.buildRustPackage {
  pname = manifest.name;
  version = manifest.version;
  cargoLock.lockFile = ./Cargo.lock;
  src = ./.;
  buildAndTestSubdir = "yeet-agent";
  nativeBuildInputs = with pkgs; [
    pkg-config
  ];
  buildInputs = [
    pkgs.openssl
  ]
  ++ pkgs.lib.optionals pkgs.stdenv.isDarwin (
    with pkgs.darwin.apple_sdk.frameworks;
    [
      SystemConfiguration
      CoreServices
      Cocoa
    ]
  )
  ++ pkgs.lib.optionals pkgs.stdenv.isLinux [
    pkgs.dbus
  ];
}
