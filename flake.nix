{
  description = "overmask - Add a writeable overlay on top of read-only files";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = {
    nixpkgs,
    flake-parts,
    rust-overlay,
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
      in rec {
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

        packages.overmask = pkgs.rustPlatform.buildRustPackage {
          pname = "overmask";
          version = "dev";

          cargoLock.lockFile = ./Cargo.lock;
          src = pkgs.lib.cleanSource ./.;

          nativeBuildInputs = with pkgs; [
            pkg-config
            rust
          ];

          buildInputs = with pkgs; [
            udev
          ];
        };
        packages.default = packages.overmask;
      };
    };
}
