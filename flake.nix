{
  description = "overmask - Add a writeable overlay on top of read-only files";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    flake-parts = {
      url = "github:hercules-ci/flake-parts";
      inputs.nixpkgs-lib.follows = "nixpkgs";
    };

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    flake-parts,
    nixpkgs,
    rust-overlay,
    self,
    ...
  } @ inputs:
    flake-parts.lib.mkFlake {inherit inputs;} {
      systems = [
        "aarch64-linux"
        "x86_64-linux"
      ];

      perSystem = {
        system,
        pkgs,
        ...
      }: let
        rust = pkgs.rust-bin.nightly.latest.default.override {
          targets = [
            "x86_64-unknown-linux-gnu"
            "x86_64-unknown-linux-musl"
          ];
          extensions = [
            "rust-src"
            "rust-analyzer-preview"
          ];
        };
      in {
        _module.args.pkgs = import nixpkgs {
          inherit system;
          overlays = [rust-overlay.overlays.default];
        };

        devShells.default = pkgs.mkShell {
          name = "overmask";

          buildInputs = with pkgs; [
            pkg-config
            rust
            taplo
            udev
          ];

          RUST_BACKTRACE = 1;
        };

        packages = rec {
          overmask = pkgs.rustPlatform.buildRustPackage {
            pname = "overmask";
            version =
              self.shortRev
              or self.dirtyShortRev;

            src = pkgs.lib.cleanSource ./.;
            cargoLock.lockFile = ./Cargo.lock;

            nativeBuildInputs = with pkgs; [
              pkg-config
            ];

            buildInputs = with pkgs; [
              rust
              udev
            ];
          };
          default = overmask;
        };
      };
    };
}
