{
    description = "overmask development environment";

    inputs = {
        nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
        mozilla.url = "github:mozilla/nixpkgs-mozilla";
    };

    outputs = { self, nixpkgs, mozilla }:
    let
        system = "x86_64-linux";
        overlays = [ self.inputs.mozilla.overlays.rust ];
        pkgs = import nixpkgs { inherit overlays system; };
        pkgsStatic = pkgs.pkgsStatic;
        pkgsCross = pkgs.pkgsCross;
        channel = pkgs.rustChannelOf {
            date = "2023-09-16";
            channel = "nightly";
            sha256 = "sha256-FCEJYhy/e7g2X2f90Ek32bFJkyIKguIIvT/hqpoFuNI=";
        };
        rust = (channel.rust.override {
            targets = [
                "x86_64-unknown-linux-musl"
            ];
            extensions = [ "rust-src" ];
        });
    in
    {
        devShells.${system}.default = pkgs.mkShell {
            name = "rust-environment";
            buildInputs = [
                rust
                pkgs.pkg-config
                pkgs.libusb1
            ];

            PKG_CONFIG_ALLOW_CROSS = true;
            PKG_CONFIG_ALL_STATIC = true;
            LIBZ_SYS_STATIC = 1;
        };
    };
}
