{
  system ? builtins.currentSystem,
  sources ? import ./sources.nix,
  pkgs ? import sources.nixpkgs {
    inherit system;
  },
}:
let

in

{
  packages = {
    yeet = pkgs.callPackage ./packages/yeet.nix { };
    yeetd = pkgs.callPackage ./packages/yeetd.nix { };
  };
}
