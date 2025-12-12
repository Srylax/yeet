{
  pkgs,
  rustPlatform ? pkgs.rustPlatform,
  lib ? pkgs.lib,
  stdenv ? pkgs.stdenv,
  ...
}:
let
  manifest = (lib.importTOML ../yeet-agent/Cargo.toml).package;
in
rustPlatform.buildRustPackage {
  pname = manifest.name;
  version = manifest.version;
  cargoLock.lockFile = ../Cargo.lock;
  src = ../.;
  buildAndTestSubdir = "yeet-agent";
  nativeBuildInputs = [
    pkgs.pkg-config
    pkgs.makeWrapper
  ];
  buildInputs = [
    pkgs.openssl
  ]
  ++ lib.optionals stdenv.isDarwin [
    pkgs.apple-sdk
  ]
  ++ lib.optionals stdenv.isLinux [
    pkgs.dbus
  ];

  postInstall = ''
    wrapProgram $out/bin/yeet --prefix PATH : $ {
        lib.makeBinPath [
            pkgs.nixos-facter
            pkgs.cachix
            pkgs.nix
        ]
    }
  '';

  meta = {
    description = "A pull-based nix deployment tool";
    homepage = "https://github.com/srylaz/yeet";
    platforms = lib.platforms.all;
    license = lib.licenses.agpl3Plus;
    mainProgram = "yeet";
    maintainers = with lib.maintainers; [ Srylax ];
  };
}
