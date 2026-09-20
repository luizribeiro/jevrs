{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    nixpkgs,
    flake-utils,
    rust-overlay,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        overlays = [(import rust-overlay)];
        pkgs = import nixpkgs {inherit system overlays;};
        toolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = ["clippy" "rust-analyzer" "rust-src" "rustfmt"];
          targets = ["wasm32-wasip2"];
        };
      in {
        devShells.default = pkgs.mkShell {
          packages = [
            toolchain
            pkgs.wasmtime
            pkgs.wasm-tools
            pkgs.cargo-nextest
            pkgs.git-absorb
          ];
        };
      }
    );
}
