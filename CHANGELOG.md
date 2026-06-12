# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `preview_deposit` and `preview_withdraw` view getters (ERC4626-style aliases).
- `max_redeem` view returning a user's redeemable share balance.
- `share_percentage` view reporting a user's share of the vault in basis points.
- Configurable minimum deposit with the `BelowMinimumDeposit` error and an
  admin `set_min_deposit` setter plus a `get_min_deposit` getter.
- Admin pause control (`set_paused` / `is_paused`) guarding deposits, the
  `Paused` error, and a `paused` event.
- Admin role transfer via `set_admin`, emitting a `set_admin` event.

## [0.1.0]

### Added

- Initial share-based (ERC4626-style) yield vault: deposit, withdraw, mock
  yield accrual, share/asset conversion views, events, and error codes.
