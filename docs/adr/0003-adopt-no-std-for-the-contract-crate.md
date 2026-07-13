# ADR 0003: Adopt no_std for the contract crate

- Status: Accepted
- Deciders: arisu6804

## Context

The YieldVault smart contract needs a clear, documented approach to "adopt no_std for the contract crate" so the codebase stays consistent and auditable.

## Decision

We adopt no_std for the contract crate as the standard for this contract, in line with Soroban best practices.

## Consequences

Improves clarity, testability, and maintainability, and gives future contributors a recorded rationale to build on.
