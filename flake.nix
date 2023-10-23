{
  description = "kupli";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };

    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    # Currently only needed for shell.nix (integration with VSCode).
    flake-compat.url = "https://flakehub.com/f/edolstra/flake-compat/1.tar.gz";

    flake-utils.url = "github:numtide/flake-utils";

    # Nix language server used for VSCode.
    nil.url = "github:oxalica/nil";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.follows = "flake-utils";
      };
    };
  };

  outputs = { self, nixpkgs, advisory-db, crane, flake-utils, nil, rust-overlay, ... }:
    flake-utils.lib.eachDefaultSystem
      (system:
        let
          overlays = [ (import rust-overlay) ];
          pkgs = import nixpkgs {
            inherit system overlays;
          };

          craneLib = crane.lib.${system};
          src = craneLib.cleanCargoSource (craneLib.path ./.);
          commonArgs = {
            inherit src;
            strictDeps = true;
            buildInputs = with pkgs; [ openssl ];
            nativeBuildInputs = with pkgs; [ pkg-config ];
          };
          cargoArtifacts = craneLib.buildDepsOnly commonArgs;
          kupli-crate = craneLib.buildPackage (commonArgs // {
            inherit cargoArtifacts;
          });
        in
        {
          devShells.default = craneLib.devShell {
            checks = self.checks.${system};
            inputsFrom = [ kupli-crate ];
            packages = with pkgs; [
              git
              # Both of these used for VSCode.
              nixpkgs-fmt
              nil.packages.${system}.default
            ];
          };

          checks = {
            # Build the crate as part of `nix flake check` for convenience.
            inherit kupli-crate;

            # Run clippy (and deny all warnings) on the crate source, again, resuing the dependency artifacts from above. Note that this is done as a separate derivation so that we can block the CI if there are issues here, but not prevent downstream consumers from building our crate by itself.
            kupli-crate-clippy = craneLib.cargoClippy (commonArgs // {
              inherit cargoArtifacts;
              cargoClippyExtraArgs = "--all-targets -- --deny warnings";
            });

            kupli-crate-doc = craneLib.cargoDoc (commonArgs // {
              inherit cargoArtifacts;
            });

            kupli-crate-fmt = craneLib.cargoFmt {
              inherit src;
            };

            kupli-crate-audit = craneLib.cargoAudit {
              inherit src advisory-db;
            };

            kupli-crate-deny = craneLib.cargoDeny {
              inherit src;
            };

            kupli-crate-nextest = craneLib.cargoNextest (commonArgs // {
              inherit cargoArtifacts;
              partitions = 1;
              partitionType = "count";
            });
          };

          packages.default = kupli-crate;
          apps.default = flake-utils.lib.mkApp {
            drv = kupli-crate;
          };
        }
      );
}
