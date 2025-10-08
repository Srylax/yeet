{ pkgs }:
pkgs.mkShell {
  # Add build dependencies
  packages = [
    pkgs.yeet-server
    pkgs.yeet-agent
  ];

  # Add environment variables
  env = { };

  # Load custom bash code
  shellHook = ''

  '';
}
