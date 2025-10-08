# Quickstart

Go to `https://app.cachix.org/cache`. Register if you have no account.
Create a new binary cache - note the name you are going to need it later.
Create a new authtoken on cachix.
`cachix authtoken <token>`


`nix run nixpkgs#yeet-server`

`nix flake init --template "github:srylax/yeet#blueprint"`
`nix develop`
`yeet vm my-nixos`
`yeet status`
*Edit the file my-nixos/configuration.nix and add ripgrep*:

`yeet publish --url localhost:3000 --cachix <name>`
`yeet monitor`
