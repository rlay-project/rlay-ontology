BINARY_DIR = bin

CLIPPY_VERSION = 0.0.189
CLIPPY_TOOLCHAIN = $(shell cat rust-toolchain-clippy)

CARGO_WEB_VERSION = 0.6.8
CARGO_WEB_TOOLCHAIN = $(shell cat rlay_ontology_stdweb/rust-toolchain)

toolchain-clippy:
	rustup install $(CLIPPY_TOOLCHAIN)

toolchain-cargo-web:
	rustup install $(CARGO_WEB_TOOLCHAIN)
	rustup target add --toolchain $(CARGO_WEB_TOOLCHAIN) wasm32-unknown-unknown

toolchains: toolchain-clippy toolchain-cargo-web

$(BINARY_DIR)/cargo-clippy:
	cargo +$(CLIPPY_TOOLCHAIN) install clippy --root . --version $(CLIPPY_VERSION)

$(BINARY_DIR)/cargo-web:
	cargo +$(CARGO_WEB_TOOLCHAIN) install cargo-web --root . --version $(CARGO_WEB_VERSION)

cargo-binaries-clippy: $(BINARY_DIR)/cargo-clippy

cargo-binaries-cargo-web: $(BINARY_DIR)/cargo-web

cargo-binaries: cargo-binaries-clippy cargo-binaries-cargo-web

lint: cargo-binaries-clippy
	# rustup run $(CLIPPY_TOOLCHAIN) bin/cargo-clippy
	cd rlay_ontology && rustup run $(CLIPPY_TOOLCHAIN) ../bin/cargo-clippy
