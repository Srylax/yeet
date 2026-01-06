{
  pkgs,
  ...
}:
{
  languages.rust = {
    enable = false;
  };

  cachix.enable = false;

  packages = with pkgs; [
    openssl
    rustup # because else we cannot use cargo +nightly fmt
    openssl
    gcc
    pkg-config
  ];
}
