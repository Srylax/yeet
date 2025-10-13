# Quickstart
`nix flake init --template github:srylax/yeet`


Go to `https://app.cachix.org/cache`. Register if you have no account.
Create a new binary cache - note the name you are going to need it later.
Create a new authtoken on cachix.
`cachix authtoken <token>`


`yeet-server`

You check out your current build with `yeet vm my-nixos` (only for nixos not for darwin)
`yeet status`
*Edit the file my-nixos/configuration.nix and add ripgrep*:

`yeet publish --cachix <name>`
`yeet monitor` [TODO]
`yeet log mynixos` [TODO]
