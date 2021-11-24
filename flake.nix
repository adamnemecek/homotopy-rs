{
  description = "homotopy.io rust edition";

  inputs = {
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
    flake-compat = {
      url = "github:edolstra/flake-compat";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (
      system:
        let
          overlays = [ (import rust-overlay) ];
          pkgs = import nixpkgs {
            inherit system overlays;
          };
          nativeBuildInputs = with pkgs; [
            (
              rust-bin.nightly."2021-11-01".default.override { # rust stable 1.56.1
                targets = [ "wasm32-unknown-unknown" ];
                extensions = [ "rust-src" ];
              }
            )
            wasm-pack
          ];
        in
          {
            devShell = with pkgs; mkShell {
              buildInputs = nativeBuildInputs ++ [
                cargo-make
                devserver
                rust-analyzer
              ];
              RUST_SRC_PATH = "${rust-bin.stable.latest.rust-src}/lib/rustlib/src/rust/library";
            };
          }
    );
}
