NETWORK ?= testnet
SOURCE ?= default
WASM = target/wasm32-unknown-unknown/release/yieldvault_contract.wasm
CONTRACT_ID ?=

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

## Verify that the on-chain WASM hash for CONTRACT_ID matches the local build.
## Usage: make verify-hash CONTRACT_ID=C... [NETWORK=mainnet] [WASM=path/to/file.wasm]
verify-hash:
	@if [ -z "$(CONTRACT_ID)" ]; then \
		echo "Usage: make verify-hash CONTRACT_ID=<contract-id> [NETWORK=<network>]"; \
		exit 2; \
	fi
	bash scripts/verify_wasm_hash.sh $(CONTRACT_ID) --network $(NETWORK) --wasm $(WASM)

## Run the bash test suite for the helper scripts in scripts/.
test-scripts:
	bash tests/test_verify_wasm_hash.sh

.PHONY: default build test fmt fmt-check lint doc check clean optimize deploy verify-hash test-scripts
