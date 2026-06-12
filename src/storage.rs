use soroban_sdk::{Address, Env};

use crate::types::DataKey;

/// Number of ledgers in roughly one day (assuming ~5 second ledgers).
const DAY_IN_LEDGERS: u32 = 17280;

/// Time-to-live bump amount for instance storage entries.
const INSTANCE_BUMP_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
/// Threshold at which instance storage entries are extended.
const INSTANCE_LIFETIME_THRESHOLD: u32 = INSTANCE_BUMP_AMOUNT - DAY_IN_LEDGERS;

/// Time-to-live bump amount for persistent storage entries.
const PERSISTENT_BUMP_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
/// Threshold at which persistent storage entries are extended.
const PERSISTENT_LIFETIME_THRESHOLD: u32 = PERSISTENT_BUMP_AMOUNT - DAY_IN_LEDGERS;

/// Extend the time-to-live of the instance storage so the contract stays live.
pub fn extend_instance(env: &Env) {
    env.storage()
        .instance()
        .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
}
