{
  inputs = {
    crane.url = "github:ipetkov/crane";

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    flake-parts.url = "github:hercules-ci/flake-parts";

    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs =
    {
      crane,
      fenix,
      flake-parts,
      ...
    }@inputs:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "aarch64-linux"
        "x86_64-linux"
      ];

      perSystem =
        {
          pkgs,
          self',
          system,
          ...
        }:
        let
          craneLib = (crane.mkLib pkgs).overrideToolchain fenix.packages.${system}.complete.toolchain;
        in
        {
          devShells.default = craneLib.devShell {
            name = "overmask";

            inputsFrom = [ self'.packages.default ];
            buildInputs = with pkgs; [
              taplo
            ];

            RUST_BACKTRACE = 1;
          };

          packages = rec {
            default = overmask;
            overmask = pkgs.callPackage ./. { inherit craneLib; };
          };

          formatter = pkgs.nixfmt;
        };
    };

  nixConfig = {
    extra-substituters = [ "https://errornobinaries.cachix.org" ];
    extra-trusted-public-keys = [
      "errornobinaries.cachix.org-1:84oagGNCIsXxBTYmfTiP+lvWje7lIS294iqAtCpFsbU="
    ];
  };

  description = "Add a writeable overlay on top of read-only files";
}
