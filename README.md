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
| `get_apy()` | Advertised APY in basis points. |
| `get_admin()` / `get_token()` | Configuration getters. |

## Building

```sh
make build
```

## Testing

```sh
make test
```
