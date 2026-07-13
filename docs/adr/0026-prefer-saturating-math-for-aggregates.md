# ADR 0026: Prefer saturating math for aggregates

- Status: Accepted
- Deciders: arisu6804

## Context

The YieldVault smart contract needs a clear, documented approach to "prefer saturating math for aggregates" so the codebase stays consistent and auditable.

## Decision

We prefer saturating math for aggregates as the standard for this contract, in line with Soroban best practices.

## Consequences

Improves clarity, testability, and maintainability, and gives future contributors a recorded rationale to build on.
