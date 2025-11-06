# Quickstart
`nix flake init --template github:srylax/yeet`


Go to `https://app.cachix.org/cache`. Register if you have no account.
Create a new binary cache - note the name you are going to need it later.
Create a new authtoken on cachix.
`cachix authtoken <token>`


`yeet-server`

You check out your current build with `yeet vm my-nixos` (only for nixos not for darwin)
`yeet publish --cachix <cache>` -> client are now listed as unverified 
*Get the code from your vm via*
`yeet approve aegis <code>` -> Client is now listed as verified
*Edit the file my-nixos/configuration.nix and add ripgrep*:
`yeet publish --cachix <cache>` -> client are now listed as unverified 
*Client automatically get the update*
`yeet log mynixos` [TODO]
