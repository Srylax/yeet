# Quickstart

`nix run nixpkgs#yeet-server`

`nix flake init --template "github:srylax/yeet#blueprint"`
`nix develop`
`yeet vm my-nixos`
`yeet status`
*Edit the file my-nixos/configuration.nix and add ripgrep*:

`yeet publish --url localhost:3000`
`yeet monitor`
