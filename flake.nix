{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    git-hooks = {
      url = "github:cachix/git-hooks.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      nixpkgs,
      flake-utils,
      git-hooks,
      rust-overlay,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
        toolchain = pkgs.rust-bin.stable.latest.minimal.override {
          extensions = [
            "clippy"
            "rust-analyzer"
            "rust-src"
            "rustfmt"
          ];
          targets = [ "wasm32-wasip2" ];
        };
        cargoFiles = "(^|/)(Cargo\\.(toml|lock)|.*\\.rs)$";
        cargoHook =
          {
            name,
            text,
            runtimeInputs ? [ ],
          }:
          {
            enable = true;
            entry = "${
              pkgs.writeShellApplication {
                inherit name text;
                runtimeInputs = [ toolchain ] ++ runtimeInputs;
              }
            }/bin/${name}";
            files = cargoFiles;
            pass_filenames = false;
          };
        cargoHooks = {
          rustfmt = {
            enable = true;
            packageOverrides = {
              cargo = toolchain;
              rustfmt = toolchain;
            };
            settings.check = true;
          };
          clippy = {
            enable = true;
            packageOverrides = {
              cargo = toolchain;
              clippy = toolchain;
            };
            settings = {
              denyWarnings = true;
              extraArgs = "--workspace --all-targets --all-features --locked";
              offline = false;
            };
          };
          features = cargoHook {
            name = "features-hook";
            text = ''
              cargo check -p jevrs --no-default-features --locked
              cargo check -p jevrs --no-default-features --features reqwest --locked
              cargo check -p jevrs --no-default-features --features native-tls --locked
            '';
          };
          cargo-nextest = cargoHook {
            name = "cargo-nextest-hook";
            runtimeInputs = [ pkgs.cargo-nextest ];
            text = "cargo nextest run --workspace --all-features --locked";
          };
          doctests = cargoHook {
            name = "doctests-hook";
            text = "cargo test --doc --workspace --all-features --locked";
          };
          wasip2-build = cargoHook {
            name = "wasip2-build-hook";
            text = ''
              cargo build -p jevrs-core --target wasm32-wasip2 --locked
              cargo clippy -p jevrs --no-default-features --features wasip2 --target wasm32-wasip2 --all-targets --locked -- -D warnings
            '';
          };
          docs = cargoHook {
            name = "docs-hook";
            text = ''
              RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features --locked
            '';
          };
        };
        offlineHooks = {
          nixfmt-rfc-style.enable = true;
          deadnix.enable = true;
          statix.enable = true;
          taplo.enable = true;
          actionlint.enable = true;
          typos.enable = true;
          check-merge-conflicts.enable = true;
          end-of-file-fixer.enable = true;
          trim-trailing-whitespace.enable = true;
          check-yaml.enable = true;
          check-toml.enable = true;
        };
        hookDefinitions = offlineHooks // cargoHooks;
        gitHooks = git-hooks.lib.${system}.run {
          src = ./.;
          hooks = hookDefinitions;
        };
      in
      {
        checks.pre-commit = git-hooks.lib.${system}.run {
          src = ./.;
          hooks = offlineHooks;
        };

        devShells.default = pkgs.mkShell {
          packages = [
            toolchain
            pkgs.pkg-config
            pkgs.openssl
            pkgs.wasmtime
            pkgs.wasm-tools
            pkgs.cargo-nextest
            pkgs.git-absorb
          ]
          ++ gitHooks.enabledPackages;
          inherit (gitHooks) shellHook;
        };
      }
    );
}
