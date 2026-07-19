# ADR 0009: Use checked arithmetic for all math

- Status: Superseded by [ADR 0026](./0026-prefer-saturating-math-for-aggregates.md) for aggregate operations
- Deciders: arisu6804

## Context

The YieldVault smart contract needs a clear, documented approach to "use checked arithmetic for all math" so the codebase stays consistent and auditable.

## Decision

We use checked arithmetic for all math as the standard for this contract, in line with Soroban best practices.

**Update:** Per [ADR 0026](./0026-prefer-saturating-math-for-aggregates.md), aggregate totals (such as `total_shares`, `total_assets`, and user balances) now use **saturating arithmetic** (`saturating_add`/`saturating_sub`) instead of checked arithmetic. This ensures that overflow on aggregates caps at `u128::MAX` and underflow floors at `0` rather than returning an error, which is more defensive for state variables that represent cumulative sums. The internal `mul_div` computation in `math.rs` continues to use checked arithmetic to detect overflow in intermediate products.
