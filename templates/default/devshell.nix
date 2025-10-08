{
  pkgs,
  perSystem,
}:
pkgs.mkShell {
  # Add build dependencies
  packages = [
    perSystem.yeet.yeet-server
    perSystem.yeet.yeet-agent
  ];

  # Add environment variables
  env = { };

  # Load custom bash code
  shellHook = ''

  '';
}
