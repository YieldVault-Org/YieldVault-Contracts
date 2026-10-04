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
| `withdraw(from, shares) -> assets` | Burn shares and return assets to `from`, subject to the configured withdrawal limits. |
| `withdraw_batch(from, shares_list) -> assets` | Redeem multiple legs for `from`; each leg and the combined period usage must fit the configured limits. |
| `balance_of(user) -> shares` | A user's share balance. |
| `total_shares()` | Total shares minted. |
| `total_assets()` | Total underlying assets held. |
| `accrue_yield(amount)` | Admin-only mock yield accrual. |
| `convert_to_shares(assets)` | Preview shares for a given asset amount. |
| `convert_to_assets(shares)` | Preview assets for a given share amount. |
| `preview_deposit(assets)` | ERC4626-style alias of `convert_to_shares`. |
| `preview_withdraw(shares)` | ERC4626-style alias of `convert_to_assets`. |
| `price_per_share()` | Value of one share, scaled by `PRICE_SCALE`. |
| `max_withdraw(user)` | Asset value of the user's full share balance; does not apply withdrawal caps. |
| `max_redeem(user)` | User's share balance; does not apply withdrawal caps. |
| `share_percentage(user)` | A user's share of the vault, in basis points. |
| `get_apy()` | Advertised APY in basis points. |
| `is_initialized()` | Whether the vault has been set up. |
| `is_paused()` | Whether deposits are currently paused. |
| `version()` | On-chain contract interface version. |
| `get_min_deposit()` | The smallest accepted deposit amount. |
| `get_admin()` / `get_token()` | Configuration getters. |

## Admin operations

The configured admin address authorizes the following privileged entrypoints:

| Function | Description |
| --- | --- |
| `accrue_yield(amount)` | Apply mock yield, raising the value of every share. |
| `set_paused(paused)` | Pause or resume new deposits; does not bypass withdrawal limits. |
| `set_min_deposit(amount)` | Set the minimum accepted deposit amount. |
| `set_admin(new_admin)` | Transfer the admin role to another address. |
| `set_withdraw_limits(max_per_op, max_per_period, period_secs)` | Configure asset caps and the period length; preserves existing period usage. |
| `reset_withdraw_period()` | Clear period usage and start a window at the current ledger timestamp. |
| `set_withdraw_limits_override(enabled)` | Enable or disable the emergency bypass of both withdrawal caps. |

While the vault is paused, `deposit` returns `Paused`. The pause flag itself
does not block `withdraw` or `withdraw_batch`, but their authorization, balance,
conversion, token-transfer and configured withdrawal-limit checks still apply.

## Withdrawal limits

Caps are `u128` amounts in underlying token base units. Both default to `0`
(unlimited); a positive period cap requires a nonzero `period_secs` or
configuration returns `InvalidWithdrawLimit`. A single withdrawal and each batch
leg must fit `max_per_op`. The sum of a batch counts against the same
`max_per_period` usage shared by all users; an empty batch returns `EmptyBatch`.
Over-cap requests return `WithdrawLimitExceeded` or
`WithdrawPeriodLimitExceeded` before any share burn or token transfer.

The period uses the ledger timestamp. On a successful withdrawal with period
enforcement active, an elapsed time greater than or equal to `period_secs`
starts a fresh window at that timestamp and clears the previous usage. Changing the cap or period length
does not itself clear usage. The admin can explicitly reset it with
`reset_withdraw_period()`. With the emergency override enabled, both cap checks
are skipped and usage is not recorded; usage is also not recorded while the
period cap is unlimited. Disabling the override does not itself reset stored
usage. These admin actions emit `wd_limits`, `wd_reset` and `wd_override` events.

Inspect configuration with `get_max_withdraw_per_op()`,
`get_max_withdraw_per_period()`, `get_withdraw_period_secs()` and
`is_withdraw_limits_override()`. `get_period_withdrawn()` and
`get_period_started_at()` return stored usage/window values; reading them does
not roll an elapsed window forward. `preview_withdraw`, `max_withdraw` and
`max_redeem` do not account for these caps, so their return values are not a
promise that a withdrawal will succeed.

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
