{
  pkgs,
  inputs,
  ...
}:
{
  imports = [ inputs.yeet.nixosModules.yeet-agent ];

  services.yeet-agent = {
    enable = true;
    url = "http://<your_hostname>:3000";
  };

  # Only needed when using VMs
  formatConfigs.vm =
    { ... }:
    {
      virtualisation.cores = 4;
      virtualisation.memorySize = 2048;
    };

  # for testing purposes only, remove on bootable hosts
  boot.loader.grub.enable = false;

  nixpkgs.hostPlatform = "x86_64-linux";
  system.stateVersion = "25.05";
}
