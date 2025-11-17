{
  config,
  pkgs,
  lib,
  ...
}:
with lib;
let
  cfg = config.services.yeet;
in
{
  meta.maintainers = [ lib.maintainers.Srylax ];

  options.services.yeet = {
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

    package = mkPackageOption pkgs "yeet" { };
  };

  config = mkIf cfg.enable {
    systemd.services.yeet = {
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
          ${lib.getExe cfg.package} agent --sleep ${toString cfg.sleep} --url ${cfg.url}
        '';
      };
    };
  };
}
