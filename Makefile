NETWORK ?= testnet
SOURCE ?= default
WASM = target/wasm32-unknown-unknown/release/yieldvault_contract.wasm

default: build

build:
	cargo build --target wasm32-unknown-unknown --release

test:
	cargo test

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all --check

lint:
	cargo clippy --all-targets -- -D warnings

doc:
	cargo doc --no-deps

check:
	cargo check --all-targets

clean:
	cargo clean

optimize: build
	stellar contract optimize --wasm $(WASM)

deploy: build
	stellar contract deploy \
		--wasm $(WASM) \
		--source $(SOURCE) \
		--network $(NETWORK)

.PHONY: default build test fmt fmt-check lint doc check clean optimize deploy
