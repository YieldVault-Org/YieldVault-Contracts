# Upgrade Strategy

This note documents the **upgrade-strategy** of the yieldvault-contract contract.

YieldVault is a Soroban smart contract on the Stellar network. This page is part of the
project's reference documentation and describes the upgrade-strategy in detail, covering the relevant
entrypoints, storage layout, and invariants where applicable.

See the README and the sources under src/ for the authoritative implementation.

## Upgrade Authority Model

The YieldVault contract is designed to be upgradeable to support future improvements, bug fixes, and feature additions. The upgrade authority model relies on the contract's configured `admin` address.

### The `upgrade` Entrypoint

The contract provides an `upgrade` entrypoint which allows the admin to update the contract's WebAssembly (Wasm) bytecode:

```rust
pub fn upgrade(env: Env, new_wasm_hash: BytesN<32>) -> Result<(), Error>
```

When invoked, the contract executes the following logic:
1. **Initialization Check**: Ensures the contract has been properly initialized.
2. **Authorization Check**: Requires cryptographic authorization from the current `admin` address. This is enforced using Soroban's `require_auth()` on the admin address.
3. **Wasm Update**: Uses the standard `env.deployer().update_current_contract_wasm(&new_wasm_hash)` host function to replace the running contract code with the bytecode identified by `new_wasm_hash`.
4. **Storage Extension**: Extends the instance storage TTL to ensure the contract's instance data remains active.
5. **Event Emission**: Emits an `upgrade` event containing the `new_wasm_hash`, allowing off-chain indexers and user interfaces to track contract upgrades.

### Security Invariants

* **Admin-Only**: Upgrades can exclusively be initiated by the address currently holding the `admin` role. No other user or contract can upgrade the vault.
* **Storage Continuity**: The `update_current_contract_wasm` mechanism preserves the contract's address and storage. All balances, total shares, and assets remain unchanged. The new Wasm bytecode must be fully compatible with the existing storage layout.

### Emitted Events

Upon a successful upgrade, the contract publishes an `upgrade` event:
* **Topic**: `Symbol::new(&env, "upgrade")`
* **Data**: `new_wasm_hash` (the `BytesN<32>` hash of the newly deployed Wasm bytecode)
