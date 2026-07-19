# ADR 0026: Prefer saturating math for aggregates

- Status: Accepted
- Deciders: arisu6804

## Context

The YieldVault smart contract needs a clear, documented approach to "prefer saturating math for aggregates" so the codebase stays consistent and auditable.

Aggregate values include `total_shares`, `total_assets`, and per-user share
balances—values that represent cumulative sums over time. These differ from
intermediate computation results (like the product in `mul_div`), where an
overflow genuinely signals a logic error that should halt execution.

## Decision

We prefer saturating math for aggregates as the standard for this contract,
in line with Soroban best practices.

Concretely, operations that update aggregate totals use:
- `saturating_add` instead of `checked_add` for additions, capping at `u128::MAX`.
- `saturating_sub` instead of `checked_sub` for subtractions, flooring at `0`.

This applies to the following state-changing operations in `lib.rs`:
- `deposit`: updating `total_shares`, `total_assets`, and user balance.
- `withdraw`: updating `total_shares`, `total_assets`, and user balance.
- `accrue_yield`: updating `total_assets`.

The internal `mul_div` helper in `math.rs` continues to use `checked_mul` for
its intermediate product, as that overflow represents a genuine arithmetic
error rather than a saturated aggregate.

## Consequences

- **Defensive safety:** Aggregate overflows silently cap rather than returning
  errors that could leave the vault in an inconsistent state.
- **No new error variants:** Since saturating operations never fail, no
  additional error codes are required.
- **Testability:** Boundary tests verify that saturating caps and floors work
  as expected at the extremes of the `u128` range.
- **Clarity:** The distinction between checked arithmetic (for computations)
  and saturating arithmetic (for aggregates) is now explicit and documented.
- **Backward compatibility:** The `Error::MathOverflow` variant is retained for
  use by `mul_div` and other non-aggregate computations. Deposit and withdraw
  flows no longer return `MathOverflow` from aggregate updates, though the
  underlying `mul_div` can still propagate it.
