{
  description = "Nix flake for configuring yeet Deploy Agent";

  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = {
    self,
    nixpkgs,
    rust-overlay,
  }: {
    overlays.default = final: prev: {
      # The Rust toolchain used for the package build
      rustToolchain = final.rust-bin.stable.latest.default;
      yeet-agent = final.rustPlatform.buildRustPackage {
        name = "yeet-agent";
        src = ./.;
        cargoLock = {
          lockFile = ./Cargo.lock;
        };
        buildAndTestSubdir = "yeet-agent";
        buildInputs =
          []
          ++ (final.lib.optionals final.stdenv.isDarwin (
            with final.darwin.apple_sdk.frameworks; [
              SystemConfiguration
              CoreServices
              Cocoa
            ]
          ));
      };
    };

    nixosModules.default = {
      pkgs,
      lib,
      config,
      ...
    }:
      with lib; let
        cfg = config.services.yeet-agent;
        systemdConfig = mkIf pkgs.stdenv.isLinux {
          systemd.services.yeet-agent = {
            description = "yeet Deploy Agent";
            wants = ["network-online.target"];
            after = ["network-online.target"];
            path = [config.nix.package];
            wantedBy = ["multi-user.target"];

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
                ${cfg.package}/bin/yeet-agent
              '';
            };
          };
        };

        launchdConfig = mkIf pkgs.stdenv.isDarwin {
          launchd.daemons.yeet-agent = {
            script = ''
              exec ${cfg.package}/bin/yeet-agent
            '';

            path = [config.nix.package pkgs.coreutils config.environment.systemPath];

            environment = {
              USER = "root";
            };

            serviceConfig.KeepAlive = true;
            serviceConfig.RunAtLoad = true;
            serviceConfig.ProcessType = "Interactive";
            serviceConfig.StandardErrorPath = cfg.logFile;
            serviceConfig.StandardOutPath = cfg.logFile;
          };
        };
      in {
        options.services.yeet-agent = {
          enable = mkEnableOption "Yeet Deploy Agent";

          package = mkPackageOption pkgs "yeet-agent" {};

          logFile = mkOption {
            type = types.nullOr types.path;
            default =
              if pkgs.stdenv.isDarwin
              then /var/root/.cache/yeet/yeet-agent.log
              else /root/.cache/yeet/yeet-agent.log;
            description = "Absolute path to log all stderr and stdout";
          };
        };
        config = mkIf cfg.enable (systemdConfig // launchdConfig);
      };
  };
}
