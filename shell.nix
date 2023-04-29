{ pkgs ? import <nixpkgs> {} }:
pkgs.mkShell {
  # nativeBuildInputs is usually what you want -- tools you need to run
  nativeBuildInputs = [
    # rust
    pkgs.rustc
    pkgs.rustfmt
    pkgs.cargo
    pkgs.clippy
  ];
}
