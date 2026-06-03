{
  description = "Rust development environment for Bara";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
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
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "bara";
          version = "0.1.0";
          src = lib.cleanSource ./.;
          cargoLock.lockFile = ./Cargo.lock;
          cargoBuildFlags = [
            "--workspace"
            "--all-targets"
          ];
          cargoTestFlags = [
            "--workspace"
          ];
        };

        checks.package = self.packages.${system}.default;

        devShells.default = pkgs.mkShell {
          packages =
            (with pkgs; [
              cargo
              cargo-audit
              cargo-deny
              cargo-nextest
              clippy
              jq
              python3
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
