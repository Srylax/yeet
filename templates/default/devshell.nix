{
  pkgs,
  perSystem,
}:
pkgs.mkShell {
  # Add build dependencies
  packages = [
    perSystem.yeet.yeet-server
    perSystem.yeet.yeet-agent
    pkgs.cachix
  ];

  # Add environment variables
  env = { };

  # Load custom bash code
  shellHook = ''

  '';
}
