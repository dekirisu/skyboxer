# Skyboxer — WASM build & dev helpers
# Usage: make <target>

PORT ?= 8080
PACK   := skyboxer-wasm
WASM   := public/pkg/$(PACK)_bg.wasm
JS     := public/pkg/$(PACK).js
TS     := public/pkg/$(PACK).d.ts
PKG_JSON := public/pkg/package.json

.PHONY: build clean dev serve build-static fmt

# Build WASM to public/pkg/
build:
	RUSTFLAGS="-Z fmt-debug=none -Z location-detail=none" \
	wasm-pack build crates/skyboxer-wasm --out-dir ../../public/pkg --target web --release

# Clean build artifacts
clean:
	rm -rf public/pkg/
	rm -rf dist/
	cargo clean

# Build + serve via Rust server
dev: build
	cargo build -p server --release
	cd public && ../target/release/server

# Watch for source changes and rebuild
watch:
	RUSTFLAGS="-Z fmt-debug=none -Z location-detail=none" \
	wasm-pack build crates/skyboxer-wasm --out-dir ../../public/pkg --target web --release --watch

# Serve (assumes public/pkg/ already exists)
serve:
	@PORT ?= 8080
	@echo "🚀 Serving at http://localhost:$(PORT)"
	cd public && ../target/release/server

# Build static output for GitHub Pages
build-static: build
	@mkdir -p dist
	@cp -r public/* dist/
	@echo "✅ Static site ready in dist/"
	@echo "   Deploy dist/ contents to GitHub Pages"

# Type check (if tsc available)
check:
	@command -v tsc >/dev/null 2>&1 && tsc --noEmit public/pkg/$(TS) || echo "tsc not installed"

# Format
fmt:
	cargo fmt
	cargo clippy -- -D warnings
