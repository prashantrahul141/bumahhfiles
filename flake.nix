{
  description = "Rust env";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/release-26.05";

    crane.url = "github:ipetkov/crane";

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    inputs@{
      self,
      nixpkgs,
      crane,
      fenix,
      flake-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ fenix.overlays.default ];
        };

        inherit (pkgs) lib;

        toolchain = pkgs.fenix.fromToolchainFile {
          file = ./rust-toolchain.toml;
          sha256 = "sha256-mvUGEOHYJpn3ikC5hckneuGixaC+yGrkMM/liDIDgoU=";
        };

        craneLib = (crane.mkLib pkgs).overrideToolchain toolchain;
        src = ./.;

        commonArgs = { inherit src; };
        GIT_HASH = builtins.substring 0 7 (
          builtins.replaceStrings [ "-dirty" ] [ "" ] (
            if self ? rev then
              self.rev
            else if self ? dirtyRev then
              self.dirtyRev
            else
              "dirty"
          )
        );

        cargoArtifacts = craneLib.buildDepsOnly commonArgs;
        package = craneLib.buildPackage {
          inherit cargoArtifacts src GIT_HASH;
          doCheck = false;
        };
      in
      {
        apps.default = flake-utils.lib.mkApp {
          drv = package;
        };

        packages = {
          inherit package;
          default = package;
          checks = {
            clippy = craneLib.cargoClippy (
              commonArgs
              // {
                inherit cargoArtifacts;
              }
            );

            fmt = craneLib.cargoFmt {
              inherit src;
            };

            doc = craneLib.cargoDoc (
              commonArgs
              // {
                inherit cargoArtifacts;
              }
            );

            nextest = craneLib.cargoNextest (
              commonArgs
              // {
                inherit cargoArtifacts;
                partitions = 1;
                partitionType = "count";
              }
            );
          };
        };

        devShells.default = pkgs.mkShell {
          inherit GIT_HASH;
          nativeBuildInputs = [ toolchain ];
        };

        formatter = pkgs.nixfmt-tree;
      }
    );
}
