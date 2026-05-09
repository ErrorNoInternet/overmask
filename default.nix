{
  craneLib,
  installShellFiles,
  lib,
  pkgs,
}:
craneLib.buildPackage {
  pname = "overmask";
  version = "0.1.0";

  src =
    let
      shellFilesFilter = path: _type: builtins.match ".*/completions/.*" path != null;
      shellOrCargo = path: type: (shellFilesFilter path type) || (craneLib.filterCargoSources path type);
    in
    lib.cleanSourceWith {
      src = ./.;
      filter = shellOrCargo;
      name = "source";
    };

  nativeBuildInputs = with pkgs; [
    installShellFiles
    pkg-config
  ];

  buildInputs = with pkgs; [
    udev
  ];

  postInstall = ''
    installShellCompletion \
      --bash completions/overmask.bash \
      --fish completions/overmask.fish \
      --zsh completions/_overmask
  '';
}
