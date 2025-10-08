{
  config,
  pkgs,
  lib,
  ...
}:
with lib;
let
  cfg = config.services.yeet-agent;
in
{
  meta.maintainers = [ lib.maintainers.Srylax ];

  options.services.yeet-agent = {
    enable = mkEnableOption "Yeet Deploy Agent: https://github.com/Srylax/yeet";

    url = mkOption {
      type = types.str;
      description = "Yeet server url to use.";
    };

    sleep = mkOption {
      type = types.int;
      default = 30;
      description = "Seconds to wait between updates";
    };

    package = mkPackageOption pkgs "yeet-agent" { };
  };

  config = mkIf cfg.enable {
    systemd.services.yeet-agent = {
      description = "Yeet Deploy Agent";
      wants = [ "network-online.target" ];
      after = [ "network-online.target" ];
      path = [ config.nix.package ];
      wantedBy = [ "multi-user.target" ];

      environment.USER = "root";

      # don't stop the service if the unit disappears
      unitConfig.X-StopOnRemoval = false;

      serviceConfig = {
        # we don't want to kill children processes as those are deployments
        KillMode = "process";
        Restart = "always";
        RestartSec = 5;
        ExecStart = ''
          ${cfg.package}/bin/yeet-agent  --sleep ${toString cfg.sleep} --url ${cfg.url}
        '';
      };
    };
  };
}
