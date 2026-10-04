# Property Tests

Deterministic fuzz / property coverage for the YieldVault math and state
transitions lives in `src/fuzz.rs` (issue #74).

## Harness

- Fixed seed: `fuzz::FUZZ_SEED` (`0x0059_5646_3734_2026`).
- PRNG: Numerical Recipes LCG; same seed → same inputs in CI.
- Failure reports retain the seed, iteration, named original inputs and
  observed values or error. Inline random draws are captured without changing
  their order.
- Generated pure-math failures also report a bounded reduced input. Reduction
  runs only after failure, keeps the same input domain and failure predicate
  (including the exact error variant), and never replaces the original input.
- Reduction visits tuple coordinates in order, trying zero and then decreasing
  values with halved subtraction steps. Its 512-predicate-call budget includes
  the initial replay. This is a deterministic bounded reduction, not a claim
  that the result is the smallest counterexample or a fixed point.
- Fixed regression fixtures retain their complete fixed inputs. Stateful vault
  failures retain raw deposit/yield inputs and available results; they do not
  run a reducer or create additional ledger snapshots. A captured context line
  before funding and deposit/yield/withdraw operations also identifies
  panics from those native calls.

The existing regression-fixture function checks that reduction is deterministic,
retains a positive divisor and the same overflow error, observes its budget, and
reports the seed, iteration, raw tuple and reduced tuple. No fuzz iteration or
stateful-case count is increased.

To inspect that focused reporter/reducer check:

```sh
cargo test --locked fuzz::fuzz_regression_fixtures_overflow_and_bounds -- --exact
```

## Invariants covered

| Suite | Property |
| --- | --- |
| `fuzz_mul_div_floor_or_safe_reject` | Floored `a*b/d`, or `DivisionByZero` / `MathOverflow` |
| `fuzz_convert_round_trip_conserves_value` | assets→shares→assets never mints value |
| `fuzz_convert_shares_round_trip_conserves` | shares→assets→shares never inflates shares |
| `fuzz_convert_to_shares_monotonic` | more assets ⇒ ≥ shares (fixed vault state) |
| `fuzz_convert_to_assets_monotonic` | more shares ⇒ ≥ assets |
| `fuzz_share_fraction_bps_bounded` | holder ≤ total shares ⇒ fraction ≤ `bps` (fee/rate bound) |
| `fuzz_price_per_share_monotonic_in_assets` | more assets ⇒ ≥ price per share |
| `fuzz_empty_vault_bootstrap_and_zero_totals` | empty-vault 1:1 bootstrap / zero redeem |
| `fuzz_regression_fixtures_overflow_and_bounds` | pinned overflow, floor, and 100% BPS edges |
| `fuzz_vault_deposit_withdraw_conserves_under_yield` | full redeem ≤ deposit + yield (solo depositor) |

No invariant is weakened to obtain a green run. Existing example-based tests
in `src/test.rs` remain the readable regression layer alongside this suite.

See the README and the sources under `src/` for the authoritative implementation.

## Verification of the counterexample reporting update

The [hosted validation run](https://github.com/woahwhattheheck/YieldVault-Contracts/actions/runs/37203561556) used Rust 1.99.0 and the locked dependency graph. The maintained suite passed with 62 tests passed, zero failed, and the same three pre-existing ignored tests. `cargo fmt --all -- --check`, `cargo clippy --lib --locked -- -D warnings`, and `cargo build --locked --release --target wasm32-unknown-unknown` also passed.

The runner started from candidate `774557b19c071d5f4df346948b1cd974aa452db2`, applied the formatter, and recorded the exact tested source: `src/fuzz.rs` blob `864d427d906deb680fca04e3b3092cc153abc5cb` and `src/storage.rs` blob `daf9c31b08a76f59c36886343c29d1d1763c6fc4`. The storage change only collapses two existing accessor chains onto single lines; it changes no behavior. The ten fuzz test functions, random draw sequence, iteration counts, and eight existing stateful cases remain unchanged. No new ledger snapshots are included in this update.
