# Issue 76: exact transfer endpoint follow-up

The follow-up in `cb469ddd854171ed3e12b3c4abe5169aadfa26c4` strengthens the existing atomic deposit/withdraw submission, PR #86.

## Behavior repaired

Checking only the vault's balance change did not establish correct delivery to or debit from its owner. A fee-token withdrawal could debit 500 from the vault but credit only 495 to its recipient while burning 500 shares. A deposit could credit 1,000 to the vault while charging its sender 1,010.

`pull_token_exact` now requires both exact sender debit and vault credit. `push_token_exact` requires both exact vault debit and recipient credit. Mismatches return the existing `TransferAmountMismatch`; the Soroban host rolls back token movements and vault accounting together. No public entrypoint, storage layout, or error code changed. The two additional balance reads per operation implement the exact-delivery policy in `invariants.md`.

## Completed focused execution

[GitHub Actions run 37192856549](https://github.com/woahwhattheheck/YieldVault-Contracts/actions/runs/37192856549), job `111408531707`, explicitly checked out `cb469ddd854171ed3e12b3c4abe5169aadfa26c4` and completed successfully on October 4, 2026.

```sh
cargo test --locked --lib test_exact_transfer_ -- --nocapture
# 2 passed; 0 failed; 0 ignored; 61 filtered out; finished in 0.02s
```

The two added cases in `src/exact_transfer_test.rs` execute the real vault entrypoints and a controlled fee-token contract inside the Soroban test host. Each checks repeated rejection, preservation of both token balances, user shares, total shares and total assets, and successful exactly-once recovery after disabling the fee. Existing tests remain unchanged.

Environment: Ubuntu 24.04.5; rustc 1.99.0 (`b940084d7`, September 28, 2026); cargo 1.99.0 (`5f94df478`); soroban-sdk 22.0.11; unchanged Cargo.lock.

SHA-256 identities from the executed checkout:

```text
4b16fff9feb41b602a65a689dc8d10669eeb66810c4d7c7307b49cf459a5836f  Cargo.lock
fc9b51a628e9c5b8e68af7655e827a92df227a22108d3adf258a9ef9630a64f2  src/lib.rs
ba73b1fccb2f47e7f622e4e43d4ecaf006f3d52efc12e93a86fa75948c42a612  src/exact_transfer_test.rs
```

The execution-only workflow is on the separate `validation/kestrel1004-exact-transfer-76` branch at `5f139ff9594778a76477c8ba358c83c731597038`, not in the submission branch. This report is documentation only; the measured product source remains the commit above.

## Evidence limits

This was one focused host-level execution, not a new full-suite, formatter, clippy, deployment or live-chain run. The original submission's 58-pass result belongs to its earlier source `9732c2d1260257682504f5149dd780d516a2b74a`; it is not a fresh full-suite result for this follow-up. No pre-patch execution is claimed here.

The exact-delivery policy intentionally rejects fee-on-transfer or rebasing behavior that changes either endpoint by a different amount. Balance checks rely on the configured token's reported balances; they do not make an arbitrary dishonest token trustworthy. No real token, account credential or fund movement was involved, and no award or payment is asserted.
