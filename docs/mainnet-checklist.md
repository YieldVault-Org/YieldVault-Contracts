# Mainnet Checklist

Use this checklist before promoting a YieldVault deployment to mainnet.
Every item should be checked off by at least one reviewer before the contract
is considered production-ready.

## Build & Artefact Integrity

- [ ] Built with the exact toolchain pinned in `rust-toolchain.toml`.
- [ ] Release binary compiled with `make build` (or `make optimize` for the
      size-optimised variant).
- [ ] **WASM hash verified** — run `make verify-hash CONTRACT_ID=<id>
      NETWORK=mainnet` and confirm the script exits 0 before proceeding.
      Record the verified hash in your deployment log.

## Deployment

- [ ] Contract deployed via `make deploy NETWORK=mainnet SOURCE=<identity>`.
- [ ] Contract ID recorded and stored securely.
- [ ] `initialize` called exactly once with the correct `admin` and `token`
      addresses.
- [ ] `is_initialized()` returns `true` on-chain.

## Access Control

- [ ] Admin address is a multi-sig or cold-wallet account, not a hot key.
- [ ] `get_admin()` confirms the expected admin address on-chain.

## State Validation

- [ ] `total_shares()` and `total_assets()` both return `0` immediately after
      initialisation (empty vault).
- [ ] `get_min_deposit()` reflects the intended minimum.
- [ ] `is_paused()` returns `false` unless a pause is deliberately intended.
- [ ] `version()` returns the expected contract version number.

## Security

- [ ] All entrypoints exercised in the testnet smoke-test (deposit, withdraw,
      accrue_yield, admin operations).
- [ ] No unexpected contract errors observed during smoke-test.
- [ ] Audit report reviewed and all findings addressed.

## Observability

- [ ] Event indexer configured to capture `initialize`, `deposit`, `withdraw`,
      `accrue_yield`, `paused`, and `set_admin` events.
- [ ] Monitoring alerts set up for admin operations and unusually large
      withdrawals.

## Documentation

- [ ] `CHANGELOG.md` updated with the release version and deployment date.
- [ ] Deployment log updated with contract ID, network, block/ledger number,
      transaction hash, and verified WASM hash.
