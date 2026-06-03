{
  description = "Rust development environment for Bara";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      nixpkgs,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        lib = pkgs.lib;
      in
      {
        devShells.default = pkgs.mkShell {
          packages =
            (with pkgs; [
              cargo
              cargo-audit
              cargo-deny
              cargo-nextest
              clippy
              ripgrep
              rust-analyzer
              rustc
              rustfmt

              clang
              lld
              llvm
              pkg-config
            ])
            ++ lib.optionals pkgs.stdenv.isDarwin (
              with pkgs;
              [
                libiconv
              ]
            );
        };
      }
    );
}
