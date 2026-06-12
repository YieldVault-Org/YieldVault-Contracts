//! Event publishing helpers for the YieldVault.
//!
//! Each helper publishes a topic identifying the event kind (and, where
//! relevant, the affected account) together with a data payload of the amounts
//! involved, so off-chain indexers can track vault activity.

use soroban_sdk::{Address, Env, Symbol};

/// Publishes a `deposit` event recording that `from` supplied `assets` of the
/// underlying token in exchange for `shares` vault shares.
pub fn deposit(env: &Env, from: &Address, assets: u128, shares: u128) {
    let topics = (Symbol::new(env, "deposit"), from.clone());
    env.events().publish(topics, (assets, shares));
}

/// Publishes a `withdraw` event recording that `from` burned `shares` vault
/// shares to redeem `assets` of the underlying token.
pub fn withdraw(env: &Env, from: &Address, shares: u128, assets: u128) {
    let topics = (Symbol::new(env, "withdraw"), from.clone());
    env.events().publish(topics, (shares, assets));
}

/// Publishes an `init` event recording the configured `admin` and underlying
/// `token` addresses.
pub fn initialize(env: &Env, admin: &Address, token: &Address) {
    let topics = (Symbol::new(env, "init"),);
    env.events().publish(topics, (admin.clone(), token.clone()));
}

/// Publishes a `yield` event recording the `amount` of assets accrued to the
/// vault as mock yield, alongside the new total assets figure.
pub fn accrue_yield(env: &Env, amount: u128, total_assets: u128) {
    let topics = (Symbol::new(env, "yield"),);
    env.events().publish(topics, (amount, total_assets));
}
