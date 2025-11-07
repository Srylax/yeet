{
  config,
  lib,
  pkgs,
  ...
}:
with lib;
let
  cfg = config.services.yeetd;
in
{
  meta.maintainers = [ lib.maintainers.Srylax ];

  options.services.yeetd = {
    enable = mkEnableOption "Yeet Server: https://github.com/Srylax/yeet";

    port = mkOption {
      type = types.port;
      description = "Yeet-API Port";
    };

    host = lib.mkOption {
      type = lib.types.str;
      default = "localhost";
      description = "The listen host for HTTP API";
    };

    user = mkOption {
      type = types.str;
      default = "yeet";
      description = ''
        User to run the Yeet Server as.
      '';
    };

    group = mkOption {
      type = types.str;
      default = "yeet";
      description = ''
        Group to run the Yeet Server as.
      '';
    };

    package = mkPackageOption pkgs "yeetd" { };
  };

  config = mkIf cfg.enable {
    systemd.services.yeetd = {
      description = "Yeet Server";
      after = [ "network.target" ];
      wantedBy = [ "multi-user.target" ];

      environment.YEET_PORT = "${cfg.port}";
      environment.YEET_HOST = "${cfg.host}";

      serviceConfig = {
        User = cfg.user;
        Group = cfg.group;
        ExecStart = "${lib.getExe cfg.package}";
      };
    };
  };
}
