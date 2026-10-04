# Withdrawal maximum parity — 4 October 2026

## Delivered behavior

Source commit `88e6464683def61c8e6196491c2c56aa5a19f1d0`, parent
`f7f7f3310571fb321295383c9d8979402083bb7c`, makes `max_withdraw` and
`max_redeem` respect the same per-operation and remaining-period caps as
`withdraw`. Both getters share a private calculation; enforcement, token
transfers, aggregate arithmetic, and stored period accounting are unchanged.

The maximum is a whole-share position. With 3 underlying assets and 2 total
shares, a 2-asset cap permits redeeming 1 share for 1 asset, not an impossible
2-asset redemption. The largest permitted share count obeys
`n * total_assets < (asset_limit + 1) * total_shares`, matching the existing
floor-rounded conversion. Checked arithmetic is retained. `max_redeem` keeps
its existing `u128` interface and conservatively returns zero when the full
position cannot be represented by the checked conversion or redeems zero assets.

Reading a maximum does not consume usage or commit a new period. At the exact
rollover boundary, the getter observes the newly available budget while the
next actual withdrawal performs the existing accounting update. Zero-valued
caps remain unlimited, emergency override remains effective, and deposit-only
pause does not prevent an exit.

## Actual execution

[Run 37192955958](https://github.com/woahwhattheheck/YieldVault-Contracts/actions/runs/37192955958)
completed successfully. It checked out the exact parent, appended four tests
to the existing `src/test.rs`, reproduced the failures, then applied the
repair and executed the same selected tests with the real repository package.

```sh
CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 \
  cargo test --locked --lib test_max_withdraw_limit_ -- --nocapture
```

| Existing-contract scenario | Parent result | Repaired result |
| --- | --- | --- |
| Active per-operation cap, consumed/lowered period cap, zero remaining budget and admin reset | Failed: reported 1000 rather than 400 | Passed |
| Another user's shared consumption, exact rollover boundary, read-only period accounting and actual withdrawal | Failed: reported 1000 rather than 100 | Passed |
| Fractional share price, rejected oversized redemption and actual redemption of the advertised maximum | Failed: reported 2 shares rather than 1 | Passed |
| Unlimited defaults, deposit-only pause, configured caps and emergency override | Failed: reported 1000 rather than 400 | Passed |

Before: exit 101, 0 passed / 4 failed. After: exit 0, 4 passed / 0 failed,
0 ignored; 64 other tests were filtered out. The after test execution reported
0.03 seconds; this is not a performance benchmark. Rust and Cargo were 1.99.0,
Soroban SDK 22.0.11, and Soroban host 22.1.3 on hosted Linux. The unchanged
repository `VaultTest` fixture uses a real Soroban test environment and Stellar
Asset Contract with mocked authorization; no live network, wallet, or chain
transaction was used.

## Source and retained evidence

- Executed source blob: `34305ca076111c4fa1d1d7c6d5dd8f747f1b02e4`.
- Executed test blob: `e24a280cbccb3c566876e75f54b9c987d397a9d8`.
- Candidate tree: `d455ffe5da2c61a8898631ebfebf88bf491bebc3`.
- Unchanged Cargo lock blob: `94d8cc982aa4094f1031b881fd7eca9f6fe44324`.

The [eight-file evidence archive](https://github.com/woahwhattheheck/YieldVault-Contracts/actions/runs/37192955958/artifacts/11300315145)
contains original identities, before/after output, exit status, source patch,
executed source/test files and the candidate commit receipt. The downloaded
archive matched SHA-256
`9a2a3bddf7d91a6d9568b9e1a7e78f43d58af394819e1a7f8c7e537e7bb993ff`.
Actions retention is 14 days; this report, the source, tests and the pinned
validation workflow commit `507c391b5c91e70d5e8b1c85cdb5c2e401e8e6f6` remain in
Git history. The validation workflow is not added to the product branch.

No full-suite, Wasm deployment, production wallet, live-chain acceptance or
bounty award is asserted by this focused execution.
