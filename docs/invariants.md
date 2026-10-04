# Invariants

Deposit and withdrawal token movements are accepted only when **both** endpoint
balance changes equal the requested amount:

- Deposit: the sender loses exactly `amount` and the vault gains exactly `amount`.
- Withdrawal: the vault loses exactly the redeemed assets and the recipient gains
  exactly that amount. A matching vault debit alone does not establish delivery.
- A mismatch returns `TransferAmountMismatch`. Soroban invocation rollback must
  restore token balances, user shares, total shares and total assets, including
  the effects written before an outbound transfer.
- Repeating a rejected operation does not accumulate transfers or share changes;
  a later conforming transfer updates the balances exactly once.

The exact-delivery policy deliberately rejects fee-on-transfer or rebasing
behavior that changes either endpoint by a different amount. The additional
sender/recipient checks require two extra balance reads per operation. No public
entrypoint, storage layout or error code changes. These checks compare the
configured token's reported balances; they cannot make an arbitrary dishonest
token implementation trustworthy.

The existing atomic-transfer tests and the `test_exact_transfer_` cases in
`src/exact_transfer_test.rs` cover failure, rollback, repeated rejection and
subsequent recovery. See the README and `src/` for complete contract behavior.
