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
        nightlyToolchain = pkgs.rust-bin.nightly."2026-09-19".minimal.override {
          extensions = [
            "clippy"
            "rustfmt"
            "rust-src"
          ];
          targets = [
            "wasm32-wasip2"
            "wasm32-wasip3"
          ];
        };
        cargoFiles = "(^|/)(Cargo\\.(toml|lock)|.*\\.rs|tests/fixtures/.*\\.json)$";
        wasip3Files = "^(crates/jevrs/src/(wasip3|wasi_common|lib|client)\\.rs|crates/jevrs/Cargo\\.toml|examples/wasip3/|xtask/|flake\\.nix|Cargo\\.lock|tests/fixtures/)";
        cargoHook =
          {
            name,
            text,
            runtimeInputs ? [ ],
            files ? cargoFiles,
          }:
          {
            enable = true;
            entry = "${
              pkgs.writeShellApplication {
                inherit name text;
                runtimeInputs = [ toolchain ] ++ runtimeInputs;
              }
            }/bin/${name}";
            inherit files;
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
          cargo-deny = cargoHook {
            name = "cargo-deny-hook";
            runtimeInputs = [ pkgs.cargo-deny ];
            files = "(^|/)(Cargo\\.(toml|lock)|deny\\.toml)$";
            text = "cargo deny check bans licenses sources";
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
          wasip2-smoke = cargoHook {
            name = "wasip2-smoke-hook";
            runtimeInputs = [ pkgs.wasmtime ];
            text = "cargo xtask wasi-smoke --target wasip2";
          };
          wasip3 = cargoHook {
            name = "wasip3-hook";
            runtimeInputs = [ pkgs.wasmtime ];
            files = wasip3Files;
            text = ''
              nix develop .#nightly -c cargo clippy -p jevrs --no-default-features --features wasip3 --target wasm32-wasip3 --all-targets --locked -- -D warnings
              cargo xtask wasi-smoke --target wasip3
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
            pkgs.cargo-deny
            pkgs.git-absorb
          ]
          ++ gitHooks.enabledPackages;
          inherit (gitHooks) shellHook;
        };

        devShells.nightly = pkgs.mkShell {
          packages = [
            nightlyToolchain
            pkgs.wasmtime
            pkgs.wasm-tools
          ];
        };
      }
    );
}
