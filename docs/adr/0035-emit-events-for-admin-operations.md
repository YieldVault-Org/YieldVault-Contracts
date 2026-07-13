# ADR 0035: Emit events for admin operations

- Status: Accepted
- Deciders: arisu6804

## Context

The YieldVault smart contract needs a clear, documented approach to "emit events for admin operations" so the codebase stays consistent and auditable.

## Decision

We emit events for admin operations as the standard for this contract, in line with Soroban best practices.

## Consequences

Improves clarity, testability, and maintainability, and gives future contributors a recorded rationale to build on.
