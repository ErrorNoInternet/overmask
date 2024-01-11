{
  description = "overmask - add a writeable overlay on top of read-only files";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    mozilla.url = "github:mozilla/nixpkgs-mozilla";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    self,
    nixpkgs,
    mozilla,
    flake-utils,
  }: (flake-utils.lib.eachDefaultSystem (
    system: let
      overlays = [mozilla.overlays.rust];
      pkgs = import nixpkgs {inherit system overlays;};
      channel = pkgs.rustChannelOf {
        date = "2023-11-08";
        channel = "nightly";
        sha256 = "sha256-FCEJYhy/e7g2X2f90Ek32bFJkyIKguIIvT/hqpoFuNI=";
      };
      rust = channel.rust.override {
        targets = [
          "x86_64-unknown-linux-gnu"
        ];
        extensions = ["rust-src"];
      };
    in rec
    {
      devShells.default = pkgs.mkShell {
        name = "rust-environment";
        nativeBuildInputs = [
          pkgs.pkg-config
        ];
        buildInputs = [
          rust
          pkgs.udev
        ];

        PKG_CONFIG_ALLOW_CROSS = true;
        PKG_CONFIG_ALL_STATIC = true;
        LIBZ_SYS_STATIC = 1;
      };

      packages.overmask = pkgs.rustPlatform.buildRustPackage {
        pname = "overmask";
        version = "0.2.1";
        cargoLock.lockFile = ./Cargo.lock;
        src = pkgs.lib.cleanSource ./.;
        nativeBuildInputs = [pkgs.pkg-config];
        buildInputs = [pkgs.udev];
      };
      defaultPackage = packages.overmask;
    }
  ));
}
