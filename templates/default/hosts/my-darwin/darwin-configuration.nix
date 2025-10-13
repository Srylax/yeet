{
  pkgs,
  inputs,
  perSystem,
  ...
}:
{
  imports = [
    # inputs.yeet.nixosModules.yeet-agent
  ];

  # update nixos module to work with darwin
  # services.yeet-agent = {
  #   enable = true;
  #   package = perSystem.yeet.yeet-agent; # need to change that. maybe an overlay?
  #   url = "http://<your_hostname>:3000";
  # };

  nixpkgs.hostPlatform = "aarch64-darwin";

  system.stateVersion = 6; # initial nix-darwin state
}
