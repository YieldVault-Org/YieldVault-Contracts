//! Shared types and compile-time constants for the YieldVault contract.
//!
//! This module collects the storage [`DataKey`] enum together with the tunable
//! constants (mock APY, contract version, price scale) so they live in one
//! place and can be referenced consistently across the contract.

use soroban_sdk::{contracttype, Address};

/// The vault's advertised mock APY, in basis points (500 == 5.00%).
pub const MOCK_APY_BPS: u32 = 500;

/// The on-chain contract version, bumped on each released interface change.
pub const VERSION: u32 = 1;

/// Fixed-point scale used when reporting the price of a single share, so that
/// fractional share prices survive integer division (1e9 == one whole asset).
pub const PRICE_SCALE: u128 = 1_000_000_000;

/// Keys used to address values in contract storage.
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    /// The vault administrator address (instance storage).
    Admin,
    /// The underlying asset token address (instance storage).
    Token,
    /// The total number of shares minted by the vault (instance storage).
    TotalShares,
    /// The total amount of underlying assets held by the vault (instance storage).
    TotalAssets,
    /// A user's share balance, keyed by their address (persistent storage).
    Balance(Address),
}
