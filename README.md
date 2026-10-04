# YieldVault

A share-based (ERC4626-style) DeFi yield vault smart contract for the Soroban platform on Stellar.

Depositors supply an underlying token and receive vault *shares* representing a
proportional claim on the vault's assets. As the vault accrues yield, the value
of each share grows, so redeeming the same number of shares later returns more
of the underlying token.

## Features

- Share/asset conversion math (ERC4626-style), rounding down in the vault's favor.
- Empty-vault bootstrap: the first depositor mints shares one-to-one with assets.
- Overflow-safe arithmetic using checked operations.
- Admin-gated mock yield accrual to simulate returns.
- Events for initialize, deposit, withdraw, and yield accrual.
- Authorization enforced via `require_auth` on the relevant caller.

## Entrypoints

| Function | Description |
| --- | --- |
| `initialize(admin, token)` | One-time setup of the admin and underlying token. |
| `deposit(from, amount) -> shares` | Deposit assets, mint shares to `from`. |
| `withdraw(from, shares) -> assets` | Burn shares, return underlying assets to `from`. |
| `balance_of(user) -> shares` | A user's share balance. |
| `total_shares()` | Total shares minted. |
| `total_assets()` | Total underlying assets held. |
| `accrue_yield(amount)` | Admin-only mock yield accrual. |
| `convert_to_shares(assets)` | Preview shares for a given asset amount. |
| `convert_to_assets(shares)` | Preview assets for a given share amount. |
| `preview_deposit(assets)` | Deposit state and conversion checks; see preview scope below. |
| `preview_withdraw(shares)` | Withdrawal state and conversion checks, excluding per-user balance. |
| `price_per_share()` | Value of one share, scaled by `PRICE_SCALE`. |
| `max_withdraw(user)` | Assets redeemable for a user's full balance. |
| `max_redeem(user)` | Shares redeemable for a user (their balance). |
| `share_percentage(user)` | A user's share of the vault, in basis points. |
| `get_apy()` | Advertised APY in basis points. |
| `is_initialized()` | Whether the vault has been set up. |
| `is_paused()` | Whether deposits are currently paused. |
| `version()` | On-chain contract interface version. |
| `get_min_deposit()` | The smallest accepted deposit amount. |
| `get_admin()` / `get_token()` | Configuration getters. |

## Preview scope

The previews share their state and conversion helpers with the mutations:

- `preview_deposit(assets)` checks initialization, pause status, nonzero assets,
  minimum deposit, and nonzero shares after rounding down.
- `preview_withdraw(shares)` checks initialization, nonzero shares, and nonzero
  assets after rounding down. This branch's pause policy still permits withdrawals.

A successful preview establishes only those checks at the observed vault state.
Neither preview takes a user address, requires the user's authorization, or
executes the underlying token transfer. The mutations require `from`
authorization and perform that transfer; `withdraw` also checks the user's
share balance before running the shared conversion helper. A preview therefore
does not guarantee that a user's mutation will succeed. Read `balance_of(user)`
when inspecting the user's shares, and handle errors from the actual mutation.

For conversion math without the preview's state, zero, minimum, and dust guards,
use `convert_to_shares` or `convert_to_assets`. These methods still use checked
intermediate arithmetic and can return arithmetic errors. Interface version 3
makes `preview_*` stricter than the former conversion aliases; callers needing
only conversion math should use `convert_*` explicitly.

## Admin operations

The configured admin address authorizes the following privileged entrypoints:

| Function | Description |
| --- | --- |
| `accrue_yield(amount)` | Apply mock yield, raising the value of every share. |
| `set_paused(paused)` | Pause or resume new deposits; withdrawals stay open. |
| `set_min_deposit(amount)` | Set the minimum accepted deposit amount. |
| `set_admin(new_admin)` | Transfer the admin role to another address. |

While the vault is paused, `deposit` returns `Paused` but `withdraw` continues
to work so depositors can always exit their position.

## Architecture

The contract is split into focused modules:

- `lib.rs` — the `YieldVault` contract type and its entrypoints.
- `math.rs` — pure, overflow-checked share/asset conversion helpers.
- `storage.rs` — typed storage accessors and time-to-live management.
- `events.rs` — event publishing helpers for indexers.
- `error.rs` / `types.rs` — error codes, storage keys, and constants.

Vault configuration and aggregate totals live in instance storage, while
per-user share balances live in persistent storage and have their
time-to-live extended on access.

## Building

```sh
make build
```

## Testing

```sh
make test
```

To run tests and generate a code coverage report (requires `cargo-tarpaulin`):

```sh
make coverage
```

## Verifying a Deployed Contract

After deploying, confirm that the on-chain WASM matches your local build:

```sh
make verify-hash CONTRACT_ID=<contract-id> [NETWORK=testnet]
```

The script exits **0** if the hashes match and **1** if they differ.
See `scripts/verify_wasm_hash.sh --help` for the full option reference and
`docs/deployment-guide.md` for a complete deployment walkthrough.

### Signed token amount boundary

Checked deposit and withdrawal previews reject an underlying asset amount
above `i128::MAX` with the existing `MathOverflow` error. The same checked
conversion is used by the token transfers, preventing a positive `u128` from
wrapping into a negative token amount. Zero/minimum/paused/dust precedence
and pure `convert_*` arithmetic remain unchanged. The maximum positive signed
amount remains accepted. Authorization, user balance and actual token liquidity
remain mutation-only conditions; a preview is not a guarantee of settlement.

On base `7bb2380b5c8c47b8a91f16329db737d698c41b06`, the real-contract range regression
recorded **1 passed / 2 failed**. The correction preserves the valid maximum
and rejects the first unrepresentable value in both previews and mutations,
with stored balances unchanged on rejection.

Native `cargo test --locked --lib`: **test result: ok. 65 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 0.15s**
Toolchain: `rustc 1.99.0 (b940084d7 2026-09-28)`, `cargo 1.99.0 (5f94df478 2026-08-27)`.
This uses the actual Soroban runtime and Stellar Asset Contract with
synthetic records and mocked user authorization, not a live-chain deployment
or a payment/acceptance receipt. Existing ignored cases remain unchanged.
