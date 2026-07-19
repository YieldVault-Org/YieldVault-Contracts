# ADR 0041: Add a script to verify the deployed WASM hash

- Status: Accepted
- Deciders: YieldVault Contributors

## Context

After a Soroban contract is deployed, there is no automated check that the
on-chain WASM blob matches the locally-built artefact.  A mismatch can arise
from deploying a stale binary, a misconfigured CI pipeline, or — in the worst
case — a supply-chain compromise.  Catching this early, before users interact
with the contract, is a minimal but high-value security control.

## Decision

We add `scripts/verify_wasm_hash.sh`, a self-contained Bash script that:

1. Accepts a Stellar contract ID, an optional network flag, and an optional
   path to the local WASM file.
2. Computes the SHA-256 of the local WASM artefact using `sha256sum` /
   `shasum` (no extra dependencies).
3. Queries the on-chain WASM hash via `stellar contract info`.
4. Compares the two digests (case-insensitively) and exits 0 on match or 1
   on mismatch, making it safe to use in CI pipelines.

A `make verify-hash` convenience target is added to the `Makefile`, and hash
verification is listed as a mandatory step in `docs/mainnet-checklist.md` and
documented in `docs/deployment-guide.md`.

The script depends only on the `stellar` CLI (already required for deployment)
and POSIX coreutils; it introduces no new language runtimes or package managers.

## Consequences

- Contributors and operators get a one-command integrity check after every
  deployment, lowering the risk of running an unintended binary on-chain.
- The `tests/test_verify_wasm_hash.sh` suite exercises the script logic in
  isolation (using a stub `stellar` binary) so the test does not require a
  live network connection.
- The `Makefile` gains two new phony targets: `verify-hash` and
  `test-scripts`.
