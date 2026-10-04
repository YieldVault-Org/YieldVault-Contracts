# TTL and Rent

YieldVault keeps rent predictable by classifying every storage entry and
bounding how often TTL may be extended.

## Inventory

| Key | Durability | Tier | Notes |
| --- | --- | --- | --- |
| `Admin`, `Token`, `TotalShares`, `TotalAssets`, `MinDeposit`, `Paused`, `ExpectedWasmHash` | Instance | Hot config / aggregates | Share one instance TTL with the contract code/instance. |
| `Balance(user)` | Persistent | Hot user ledger | Extended when an active user is read or written. |
| `TtlInstanceBumped`, `TtlBalanceBumped(user)` | Temporary | Per-ledger scratch | Key-level dedup; values are scoped by ledger sequence. |
| `TtlBumpCount` | Temporary | Invocation scratch | Reset at each TTL-touching public entrypoint; sequence prevents reuse across ledgers. |

There are no cold durable entries today. Cleanup stays bounded because scratch
keys carry no business state and the bump budget caps host `extend_ttl` calls.

## Bump rules

Constants live in `src/storage.rs`:

- Instance: bump to `INSTANCE_BUMP_AMOUNT` (~30 days) when below
  `INSTANCE_LIFETIME_THRESHOLD` (~29 days), **at most once per ledger**.
- Persistent balances: same ~30 / ~29 day window, **at most once per user key
  per ledger** (a read followed by a write of the same balance shares one bump).
- Hard cap: `MAX_TTL_BUMPS_PER_INVOCATION` (8) bumps per invocation.
  Further attempts no-op so rent work cannot grow unboundedly.

Dedup scratch is keyed by ledger sequence, so activity in ledger *N*
never suppresses a legitimate bump in ledger *N+1*. Within one ledger, extra
bumps are redundant because TTL is measured in ledgers. The extension counter,
however, resets at the start of each public invocation that can touch TTL. This
prevents seven earlier depositors from consuming the renewal allowance of
later depositors in the same ledger. Pure configuration, aggregate and
conversion views do not reset the counter. No business storage keys, entrypoint
signatures, authorization rules or accounting calculations change.

## Expiration behaviour

- **Instance archived:** the host rejects calls; no vault mutation is possible,
  so aggregates cannot corrupt.
- **Persistent balance archived / missing:** reads return `0`. Withdrawals fail
  with `InsufficientShares` and leave totals unchanged. A later deposit writes a
  fresh balance entry against the live aggregates.

## Tests

See `src/test.rs` cases prefixed `test_ttl_` for instrumentation of bump counts,
dedup across repeated reads, read/write coalescing, budget exhaustion, ledger
window reset, and expired-balance fail-safe withdrawals.

## Invocation-isolation regression

The unchanged contract at `096a5dc4416a6faa57fdd80f60952eb7dcd6e11e` failed the new real-Soroban
`test_ttl_separate_invocations_cannot_exhaust_other_users`: later depositors
received an unextended balance entry after earlier callers consumed the
shared allowance. With invocation initialization, twelve distinct depositors
and twelve later renewal reads receive their configured TTL in one ledger.
Aggregate balances and pure-view budget behavior are also checked.

Execution: `rustc 1.99.0 (b940084d7 2026-09-28)`, `cargo 1.99.0 (5f94df478 2026-08-27)`.
`cargo test --locked --lib`: **test result: ok. 61 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 0.36s**

Existing dedup, per-invocation exhaustion and expiration cases are retained
unchanged. This is native contract execution, not a live-chain deployment,
performance benchmark, sponsor acceptance or payment receipt.
