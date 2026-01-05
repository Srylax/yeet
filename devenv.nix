{
  pkgs,
  ...
}:
{
  languages.rust = {
    enable = true;
  };
  cachix.enable = false;

  packages = [
    pkgs.openssl
  ];
}
