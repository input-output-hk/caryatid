{
  description = "The Caryatid framework for cardano";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.11";
    flake-utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nix-community/naersk";
  };

  outputs =
    inputs@{ self
    , flake-utils
    , nixpkgs
    , naersk
    , ...
    }:
    flake-utils.lib.eachSystem flake-utils.lib.defaultSystems (system:
    let
      pkgs = nixpkgs.legacyPackages.${system};
    in
    {
      inherit inputs;

      # TODO: build rust packages using naersk

      devShells.default = pkgs.mkShell {
        buildInputs = [
          # Rust build tools
          pkgs.rustc
          pkgs.cargo
          pkgs.rust-analyzer
          # Haskell build tools
          pkgs.ghc
          pkgs.cabal-install
        ];
      };
    });
}
