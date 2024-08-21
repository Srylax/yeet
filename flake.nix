{
  description = "Nix flake for configuring yeet Deploy Agent";

  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";
        rust-overlay.url = "github:oxalica/rust-overlay";

  };

  outputs = { self, nixpkgs, rust-overlay   }:
let
            # Systems supported
            allSystems = [
              "x86_64-linux" # 64-bit Intel/AMD Linux
              "aarch64-linux" # 64-bit ARM Linux
              "x86_64-darwin" # 64-bit Intel macOS
              "aarch64-darwin" # 64-bit ARM macOS
            ];

            # Helper to provide system-specific attributes
            forAllSystems = f: nixpkgs.lib.genAttrs allSystems (system: f {
              pkgs = import nixpkgs {
                inherit system;
                overlays = [
                  # Provides Nixpkgs with a rust-bin attribute for building Rust toolchains
                  rust-overlay.overlays.default
                  # Uses the rust-bin attribute to select a Rust toolchain
                  self.overlays.default
                ];
              };
            });

    in {

    overlays.default = final: prev: {
            # The Rust toolchain used for the package build
            rustToolchain = final.rust-bin.stable.latest.default;
          };

          packages = forAllSystems ({ pkgs }: {
            default =
              let
                rustPlatform = pkgs.makeRustPlatform {
                  cargo = pkgs.rustToolchain;
                  rustc = pkgs.rustToolchain;
                };
              in
              rustPlatform.buildRustPackage {
                name = "yeet";
                src = ./.;
                cargoLock = {
                  lockFile = ./Cargo.lock;
                };
                buildAndTestSubdir = "yeet-agent";
                    buildInputs = [] ++ ( pkgs.lib.optionals pkgs.stdenv.isDarwin (with pkgs.darwin.apple_sdk.frameworks; [
                      SystemConfiguration
                      CoreServices
                    ]
                  ));
};
          });
          nixosModules.default =  {pkgs,lib,...}: with lib; {

      options.services.yeet-agent = {
        enable = mkEnableOption "Yeet Deploy Agent";

        package = mkPackageOption nixpkgs "yeet" { };
      };

      config = mkIf cfg.enable {
        systemd.services.yeet-agent = {
          description = "yeet Deploy Agent";
          wants = [ "network-online.target" ];
          after = ["network-online.target"];
          path = [ config.nix.package ];
          wantedBy = [ "multi-user.target" ];

          # yeet requires $USER to be set
          environment.USER = "root";

          # don't stop the service if the unit disappears
          unitConfig.X-StopOnRemoval = false;

          serviceConfig = {
            # we don't want to kill children processes as those are deployments
            KillMode = "process";
            Restart = "always";
            RestartSec = 5;
            ExecStart = ''
              ${cfg.package}/bin/yeet
            '';
          };
        };
      };
      };
    };
}
