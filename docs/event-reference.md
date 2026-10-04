# Event Reference

Lifecycle events (`deposit`, `withdraw`, `yield`) use a versioned schema so
indexers can evolve without silent meaning changes.

## Schema version

`EVENT_SCHEMA_VERSION` (currently **1**) is published as topic index 1 on every
lifecycle event. Bump it when field order or units change incompatibly. It is
independent of the on-chain `version()` integer.

## Topics

`(kind: Symbol, schema_version: u32, actor: Address)`

| Index | Field | Meaning |
| --- | --- | --- |
| 0 | `kind` | `"deposit"`, `"withdraw"`, or `"yield"` |
| 1 | `schema_version` | Schema id (`EVENT_SCHEMA_VERSION`) |
| 2 | `actor` | Authorizing party (depositor, withdrawer, or admin) |

## Data payload (schema v1)

`(asset, amount_assets, amount_shares, total_assets, total_shares, correlation, outcome)`

| Field | Type | Units / meaning |
| --- | --- | --- |
| `asset` | `Address` | Underlying SEP-41 token contract |
| `amount_assets` | `u128` | Underlying token **base units** (no hidden scale) |
| `amount_shares` | `u128` | Vault share units (no hidden scale) |
| `total_assets` | `u128` | Vault aggregate assets after the mutation |
| `total_shares` | `u128` | Vault aggregate shares after the mutation |
| `correlation` | `u32` | Ledger sequence at emission (reconstruction key) |
| `outcome` | `Symbol` | `"ok"` on success (failed txs emit nothing) |

### Kind-specific amount semantics

- **deposit** — `amount_assets` deposited, `amount_shares` minted
- **withdraw** — `amount_shares` burned, `amount_assets` redeemed
- **yield** — `amount_assets` credited as yield, `amount_shares` = 0

## Migration from pre-v1 payloads

| Event | Legacy topics | Legacy data | v1 change |
| --- | --- | --- | --- |
| deposit | `(deposit, from)` | `(assets, shares)` | schema version + actor in topics; asset, totals, correlation, outcome added |
| withdraw | `(withdraw, from)` | `(shares, assets)` | same expansion; amount order normalized to `(assets, shares)` in the shared layout via `amount_assets` / `amount_shares` |
| yield | `(yield,)` | `(amount, total_assets)` | actor + schema version in topics; asset, shares (=0), total_shares, correlation, outcome added |

Indexers should key off `schema_version == 1` and reject unknown versions
(fail closed). See the fixture parser tests in `src/test.rs` for a reference
decoder that fails on legacy two-field payloads and foreign schema versions.

## Non-lifecycle events

`init`, `paused`, `set_admin`, and `upgrade` keep their existing compact
payloads and are not covered by `EVENT_SCHEMA_VERSION`.

## Saturation and actual credited yield

The schema-v1 yield amount_assets field is the actual increase in stored
total_assets, not the requested mock-yield amount. When saturating arithmetic
reaches u128::MAX, a partially credited request emits only the credited delta;
a successful request at the cap emits a zero amount. The existing one-event
behavior, successful outcome, schema layout/version, actor, asset, correlation
and share totals are unchanged. No accounting policy or migration is added.

At original source `3ed5f5a13be1320120597bc07490b4113e7eb1a8`, the new actual-contract
regression failed: a request for7 at u128::MAX-2 emitted7 even though only2
were credited. The corrected case additionally covers ordinary credit and
a zero-delta call at the cap, decoding all topic/payload fields.

Native `cargo test --locked --lib`: **test result: ok. 57 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 0.18s**
Toolchain: `rustc 1.99.0 (b940084d7 2026-09-28)`, `cargo 1.99.0 (5f94df478 2026-08-27)`.
Existing emission, fixture parser, compatibility and lifecycle tests are
preserved. This uses real Soroban with synthetic records and mocked
authorization; it is not a live-chain, indexer deployment or payment receipt.
