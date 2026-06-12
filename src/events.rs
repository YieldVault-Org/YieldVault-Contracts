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
