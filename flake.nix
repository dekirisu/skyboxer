{
  description = "Skyboxer - Equirectangular panorama to skybox converter";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };

        rust = pkgs.rust-bin.selectLatestNightlyWith (toolchain: toolchain.default.override {
          extensions = [ "rust-src" "rustfmt" "clippy" ];
          targets = [ "wasm32-unknown-unknown" ];
        });
      in
      {
        devShells.default = pkgs.mkShell {
          nativeBuildInputs = [
            pkgs.wasm-pack
            pkgs.binaryen
            rust
            pkgs.lld
          ];

          shellHook = ''
            echo "🚀 Skyboxer"
            echo ""
            echo "Usage:"
            echo "  make dev     - Build WASM + serve"
            echo "  make serve   - Serve without rebuild"
            echo "  make build-static - Build static output"
            echo "  make clean   - Remove build artifacts"
            echo "  make fmt     - Format code"
            echo ""
          '';
        };
      }
    );
}
