# ADR 0027: Return Result over panicking where possible

- Status: Accepted
- Deciders: arisu6804

## Context

The YieldVault smart contract needs a clear, documented approach to "return result over panicking where possible" so the codebase stays consistent and auditable.

## Decision

We return Result over panicking where possible as the standard for this contract, in line with Soroban best practices.

## Consequences

Improves clarity, testability, and maintainability, and gives future contributors a recorded rationale to build on.
