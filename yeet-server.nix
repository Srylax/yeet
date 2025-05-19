{
  config,
  lib,
  pkgs,
  ...
}:
with lib; let
  cfg = config.services.yeet-server;
in {
  meta.maintainers = [lib.maintainers.Srylax];

  options.services.yeet-server = {
    enable = mkEnableOption "Yeet Server: https://github.com/Srylax/yeet";

    port = mkOption {
      type = types.port;
      description = "Yeet-API Port";
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

    package = mkPackageOption pkgs "yeet-server" {};
  };

  config = mkIf cfg.enable {
    systemd.services.yeet-agent = {
      description = "Yeet Server";
      after = ["network.target"];
      wantedBy = ["multi-user.target"];

      environment.YEET_PORT = "${cfg.port}";

      serviceConfig = {
        User = cfg.user;
        Group = cfg.group;
        ExecStart = "${lib.getExe cfg.package}";
      };
    };
  };
}
