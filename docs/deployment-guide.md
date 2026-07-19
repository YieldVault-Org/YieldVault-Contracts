# Deployment Guide

This guide covers building, deploying, and verifying a YieldVault contract on
Stellar's Soroban platform.

## Prerequisites

- [Stellar CLI](https://github.com/stellar/stellar-cli) installed and on your `PATH`.
- A funded Stellar account configured as a named identity in the CLI
  (e.g. `stellar keys generate default`).
- The Rust toolchain pinned in `rust-toolchain.toml` (`rustup` installs it
  automatically on first use).

## Build

```sh
make build
```

The compiled WASM artefact is written to
`target/wasm32-unknown-unknown/release/yieldvault_contract.wasm`.

For a size-optimised binary (recommended for mainnet):

```sh
make optimize
```

This produces `…/yieldvault_contract.optimized.wasm` alongside the standard build.

## Deploy

```sh
make deploy [NETWORK=testnet] [SOURCE=default]
```

The CLI prints the deployed **contract ID** (a Stellar contract address starting
with `C`). Save this value — you will need it for the initialise step and for
hash verification.

## Initialise

After deployment, call `initialize` once to set the admin and underlying token:

```sh
stellar contract invoke \
  --id <CONTRACT_ID> \
  --source <SOURCE> \
  --network <NETWORK> \
  -- initialize \
  --admin <ADMIN_ADDRESS> \
  --token <TOKEN_ADDRESS>
```

## Verify the Deployed WASM Hash

Before interacting with a deployed contract it is strongly recommended to
confirm that the on-chain WASM matches your local build.  This catches
deployment errors and supply-chain integrity issues early.

```sh
make verify-hash CONTRACT_ID=<contract-id> [NETWORK=testnet]
```

Or invoke the script directly for additional options:

```sh
# Standard build
bash scripts/verify_wasm_hash.sh <CONTRACT_ID> --network testnet

# Optimised build
bash scripts/verify_wasm_hash.sh <CONTRACT_ID> --network mainnet --optimized

# Custom WASM path
bash scripts/verify_wasm_hash.sh <CONTRACT_ID> --wasm path/to/custom.wasm
```

Exit codes:
- **0** — hashes match; the on-chain contract is identical to the local artefact.
- **1** — hashes differ; investigate before proceeding.
- **2** — pre-condition failure (bad arguments, missing file, CLI not found).

See `docs/mainnet-checklist.md` for a full pre-launch checklist that includes
hash verification as a mandatory step.
