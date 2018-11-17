#! /usr/bin/env bash
set -euxo pipefail
cargo build
cargo build --features web3_compat
cargo +nightly-2018-10-15 build --target wasm32-unknown-unknown --no-default-features --features pwasm
cargo +nightly-2018-10-15 build --target wasm32-unknown-unknown --no-default-features --features pwasm,serialize2
