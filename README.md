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
| `preview_deposit(assets)` | ERC4626-style alias of `convert_to_shares`. |
| `preview_withdraw(shares)` | ERC4626-style alias of `convert_to_assets`. |
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

## Admin operations

Each entrypoint uses the authorization shown below. Proposal inspection is
read-only and does not require either administrator's authorization.

| Function | Required authorization | Description |
| --- | --- | --- |
| `accrue_yield(amount)` | Current admin | Apply mock yield, raising the value of every share. |
| `set_paused(paused)` | Current admin | Pause or resume new deposits; withdrawals stay open. |
| `set_min_deposit(amount)` | Current admin | Set the minimum accepted deposit amount. |
| `propose_admin(new_admin)` | Current admin | Stage or replace a two-step admin rotation (7-day TTL); rejects the current admin as the nominee. |
| `accept_admin()` | Pending admin | Accept the proposal, take control, and clear the staged proposal. |
| `cancel_admin_proposal()` | Current admin | Cancel an unaccepted proposal, including an expired one. |
| `get_pending_admin()` / `get_admin_proposal_expiry()` | None | Inspect the staged nominee and expiry timestamp. |

On-chain interface version 3 removes immediate `set_admin`. Integrators must
first call `propose_admin(new_admin)` with current-admin authorization, then
call `accept_admin()` with the nominated account's authorization. Proposing
alone leaves the active admin unchanged; only successful acceptance transfers
the role.

Expiry uses the ledger timestamp. Acceptance fails with `AdminProposalExpired`
when the ledger time is greater than the proposal's expiry timestamp and leaves
the active admin unchanged. An expired proposal remains visible to the getters
until the current admin cancels it or overwrites it with a new proposal; a
nonempty pending-admin getter alone does not establish that acceptance is still
allowed.

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
