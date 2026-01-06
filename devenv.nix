{
  pkgs,
  ...
}:
{
  languages.rust = {
    enable = false;
  };

  cachix.enable = false;

  packages = [
    pkgs.openssl
    pkgs.rustup # because else we cannot use cargo +nightly fmt
    pkgs.openssl
    pkgs.gcc
    pkgs.pkg-config
  ];
}
