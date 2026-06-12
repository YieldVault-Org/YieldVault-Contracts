use soroban_sdk::{Address, Env, Symbol};

/// Publishes a `deposit` event recording that `from` supplied `assets` of the
/// underlying token in exchange for `shares` vault shares.
pub fn deposit(env: &Env, from: &Address, assets: u128, shares: u128) {
    let topics = (Symbol::new(env, "deposit"), from.clone());
    env.events().publish(topics, (assets, shares));
}
