# ADR 0029: Source time from the ledger timestamp

- Status: Accepted
- Deciders: arisu6804

## Context

The YieldVault smart contract needs a clear, documented approach to "source time from the ledger timestamp" so the codebase stays consistent and auditable.

## Decision

We source time from the ledger timestamp as the standard for this contract, in line with Soroban best practices.

## Consequences

Improves clarity, testability, and maintainability, and gives future contributors a recorded rationale to build on.
