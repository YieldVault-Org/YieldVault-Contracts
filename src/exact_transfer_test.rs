use crate::{YieldVault, YieldVaultClient};
use soroban_sdk::{testutils::Address as _, Address, Env};

// Configurable variant of the existing fee-on-transfer fixture. All effects
// still pass through the real token contract, vault entrypoints and host.
mod fee_on_transfer_token {
    use soroban_sdk::{contract, contractimpl, contracttype, Address, Env};

    #[contracttype]
    #[derive(Clone)]
    enum DataKey {
        Balance(Address),
        FeeEnabled,
        FeeFromSender,
    }

    #[contract]
    pub struct FeeOnTransferToken;

    #[contractimpl]
    impl FeeOnTransferToken {
        pub fn set_fee(env: Env, enabled: bool, from_sender: bool) {
            env.storage().instance().set(&DataKey::FeeEnabled, &enabled);
            env.storage()
                .instance()
                .set(&DataKey::FeeFromSender, &from_sender);
        }

        pub fn mint(env: Env, to: Address, amount: i128) {
            let key = DataKey::Balance(to.clone());
            let bal: i128 = env.storage().persistent().get(&key).unwrap_or(0);
            env.storage().persistent().set(&key, &(bal + amount));
        }

        pub fn balance(env: Env, id: Address) -> i128 {
            env.storage()
                .persistent()
                .get(&DataKey::Balance(id))
                .unwrap_or(0)
        }

        pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
            from.require_auth();
            let enabled = env
                .storage()
                .instance()
                .get(&DataKey::FeeEnabled)
                .unwrap_or(true);
            let from_sender = env
                .storage()
                .instance()
                .get(&DataKey::FeeFromSender)
                .unwrap_or(false);
            let fee = if enabled { amount / 100 } else { 0 };
            let debited = amount + if from_sender { fee } else { 0 };
            let credited = amount - if from_sender { 0 } else { fee };
            let from_key = DataKey::Balance(from.clone());
            let from_bal: i128 = env.storage().persistent().get(&from_key).unwrap_or(0);
            env.storage()
                .persistent()
                .set(&from_key, &(from_bal - debited));
            let to_key = DataKey::Balance(to.clone());
            let to_bal: i128 = env.storage().persistent().get(&to_key).unwrap_or(0);
            env.storage()
                .persistent()
                .set(&to_key, &(to_bal + credited));
        }
    }
}

#[test]
fn test_exact_transfer_rejects_short_credit_withdrawal_atomically() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let token_id = env.register(fee_on_transfer_token::FeeOnTransferToken, ());
    let token = fee_on_transfer_token::FeeOnTransferTokenClient::new(&env, &token_id);
    let vault_id = env.register(YieldVault, ());
    let vault = YieldVaultClient::new(&env, &vault_id);
    vault.initialize(&admin, &token_id);
    let user = Address::generate(&env);

    token.set_fee(&false, &false);
    token.mint(&user, &1_000);
    assert_eq!(vault.deposit(&user, &1_000), 1_000);
    token.set_fee(&true, &false);

    // The token debits 500 from the vault but credits only 495 to the user.
    // A vault-only delta check wrongly accepted this and burned 500 shares.
    for _ in 0..2 {
        assert_eq!(
            vault.try_withdraw(&user, &500),
            Err(Ok(crate::Error::TransferAmountMismatch))
        );
        assert_eq!(token.balance(&user), 0);
        assert_eq!(token.balance(&vault_id), 1_000);
        assert_eq!(vault.balance_of(&user), 1_000);
        assert_eq!(vault.total_shares(), 1_000);
        assert_eq!(vault.total_assets(), 1_000);
    }

    // Correct delivery after rejection succeeds once, without retained burns.
    token.set_fee(&false, &false);
    assert_eq!(vault.withdraw(&user, &500), 500);
    assert_eq!(token.balance(&user), 500);
    assert_eq!(token.balance(&vault_id), 500);
    assert_eq!(vault.balance_of(&user), 500);
    assert_eq!(vault.total_shares(), 500);
    assert_eq!(vault.total_assets(), 500);
}

#[test]
fn test_exact_transfer_rejects_extra_sender_debit_atomically() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let token_id = env.register(fee_on_transfer_token::FeeOnTransferToken, ());
    let token = fee_on_transfer_token::FeeOnTransferTokenClient::new(&env, &token_id);
    let vault_id = env.register(YieldVault, ());
    let vault = YieldVaultClient::new(&env, &vault_id);
    vault.initialize(&admin, &token_id);
    let user = Address::generate(&env);
    token.mint(&user, &2_000);
    token.set_fee(&true, &true);

    // The vault receives 1,000 but the depositor loses 1,010. Both endpoints
    // must match the requested amount; checking only vault credit misses this.
    for _ in 0..2 {
        assert_eq!(
            vault.try_deposit(&user, &1_000),
            Err(Ok(crate::Error::TransferAmountMismatch))
        );
        assert_eq!(token.balance(&user), 2_000);
        assert_eq!(token.balance(&vault_id), 0);
        assert_eq!(vault.balance_of(&user), 0);
        assert_eq!(vault.total_shares(), 0);
        assert_eq!(vault.total_assets(), 0);
    }

    token.set_fee(&false, &false);
    assert_eq!(vault.deposit(&user, &1_000), 1_000);
    assert_eq!(token.balance(&user), 1_000);
    assert_eq!(token.balance(&vault_id), 1_000);
    assert_eq!(vault.balance_of(&user), 1_000);
    assert_eq!(vault.total_shares(), 1_000);
    assert_eq!(vault.total_assets(), 1_000);
}
