#! /usr/bin/env bash
set -euxo pipefail
cargo build
cargo build --features web3_compat
cargo +nightly-2018-11-08 build --no-default-features --features pwasm --target wasm32-unknown-unknown
