# ADR 0032: Validate all entrypoint inputs

- Status: Accepted
- Deciders: arisu6804

## Context

The YieldVault smart contract needs a clear, documented approach to "validate all entrypoint inputs" so the codebase stays consistent and auditable.

## Decision

We validate all entrypoint inputs as the standard for this contract, in line with Soroban best practices.

## Consequences

Improves clarity, testability, and maintainability, and gives future contributors a recorded rationale to build on.
