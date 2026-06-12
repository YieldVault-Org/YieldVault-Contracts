use soroban_sdk::{contracttype, Address};

/// The vault's advertised mock APY, in basis points (500 == 5.00%).
pub const MOCK_APY_BPS: u32 = 500;

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
