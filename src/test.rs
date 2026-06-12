#![cfg(test)]

extern crate std;

use soroban_sdk::testutils::Address as _;
use soroban_sdk::token::{StellarAssetClient, TokenClient};
use soroban_sdk::{Address, Env};

use crate::{YieldVault, YieldVaultClient};

/// Bundles together the objects needed to exercise the vault in a test.
struct VaultTest<'a> {
    env: Env,
    admin: Address,
    token: TokenClient<'a>,
    token_admin: StellarAssetClient<'a>,
    vault: YieldVaultClient<'a>,
}

impl<'a> VaultTest<'a> {
    /// Sets up an initialized vault backed by a fresh Stellar Asset Contract
    /// token, with all authorizations mocked.
    fn setup() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);

        let issued = env.register_stellar_asset_contract_v2(admin.clone());
        let token_address = issued.address();
        let token = TokenClient::new(&env, &token_address);
        let token_admin = StellarAssetClient::new(&env, &token_address);

        let vault_address = env.register(YieldVault, ());
        let vault = YieldVaultClient::new(&env, &vault_address);
        vault.initialize(&admin, &token_address);

        VaultTest {
            env,
            admin,
            token,
            token_admin,
            vault,
        }
    }

    /// Mints `amount` of the underlying token to `user`.
    fn mint(&self, user: &Address, amount: i128) {
        self.token_admin.mint(user, &amount);
    }
}

#[test]
fn test_initialize_sets_admin_and_token() {
    let t = VaultTest::setup();
    assert_eq!(t.vault.get_admin(), t.admin);
    assert_eq!(t.vault.get_token(), t.token.address);
}

#[test]
fn test_double_initialize_fails() {
    let t = VaultTest::setup();
    let other = Address::generate(&t.env);
    let res = t.vault.try_initialize(&other, &t.token.address);
    assert_eq!(res, Err(Ok(crate::Error::AlreadyInitialized)));
}

#[test]
fn test_initial_state_is_empty() {
    let t = VaultTest::setup();
    assert_eq!(t.vault.total_shares(), 0);
    assert_eq!(t.vault.total_assets(), 0);
    let user = Address::generate(&t.env);
    assert_eq!(t.vault.balance_of(&user), 0);
    assert_eq!(t.vault.get_apy(), 500);
}

#[test]
fn test_first_deposit_mints_one_to_one() {
    let t = VaultTest::setup();
    let user = Address::generate(&t.env);
    t.mint(&user, 1_000);

    let shares = t.vault.deposit(&user, &1_000u128);

    // The first deposit bootstraps the exchange rate one-to-one.
    assert_eq!(shares, 1_000);
    assert_eq!(t.vault.balance_of(&user), 1_000);
    assert_eq!(t.vault.total_shares(), 1_000);
    assert_eq!(t.vault.total_assets(), 1_000);
    assert_eq!(t.token.balance(&user), 0);
    assert_eq!(t.token.balance(&t.vault.address), 1_000);
}

#[test]
fn test_deposit_then_full_withdraw_round_trip() {
    let t = VaultTest::setup();
    let user = Address::generate(&t.env);
    t.mint(&user, 1_000);

    let shares = t.vault.deposit(&user, &1_000u128);
    let assets = t.vault.withdraw(&user, &shares);

    // With no yield in between, the round trip returns the original assets.
    assert_eq!(assets, 1_000);
    assert_eq!(t.vault.balance_of(&user), 0);
    assert_eq!(t.vault.total_shares(), 0);
    assert_eq!(t.vault.total_assets(), 0);
    assert_eq!(t.token.balance(&user), 1_000);
    assert_eq!(t.token.balance(&t.vault.address), 0);
}

#[test]
fn test_yield_increases_share_value() {
    let t = VaultTest::setup();
    let user = Address::generate(&t.env);
    t.mint(&user, 1_000);

    let shares = t.vault.deposit(&user, &1_000u128);

    // Admin accrues 1_000 of mock yield, doubling assets without new shares.
    // Fund the vault so the eventual withdrawal can actually transfer out.
    t.mint(&t.vault.address, 1_000);
    t.vault.accrue_yield(&1_000u128);

    assert_eq!(t.vault.total_assets(), 2_000);
    assert_eq!(t.vault.total_shares(), 1_000);

    // The same shares now redeem for twice the assets.
    let preview = t.vault.convert_to_assets(&shares);
    assert_eq!(preview, 2_000);
}

#[test]
fn test_deposit_yield_withdraw_round_trip() {
    let t = VaultTest::setup();
    let user = Address::generate(&t.env);
    t.mint(&user, 1_000);

    let shares = t.vault.deposit(&user, &1_000u128);

    t.mint(&t.vault.address, 500);
    t.vault.accrue_yield(&500u128);

    let assets = t.vault.withdraw(&user, &shares);

    // User deposited 1_000, vault earned 500 yield, so withdrawal returns 1_500.
    assert_eq!(assets, 1_500);
    assert_eq!(t.token.balance(&user), 1_500);
    assert_eq!(t.vault.total_assets(), 0);
    assert_eq!(t.vault.total_shares(), 0);
}

#[test]
fn test_second_depositor_gets_fewer_shares_after_yield() {
    let t = VaultTest::setup();
    let alice = Address::generate(&t.env);
    let bob = Address::generate(&t.env);
    t.mint(&alice, 1_000);
    t.mint(&bob, 1_000);

    let alice_shares = t.vault.deposit(&alice, &1_000u128);
    assert_eq!(alice_shares, 1_000);

    // Yield doubles the share price before Bob deposits.
    t.mint(&t.vault.address, 1_000);
    t.vault.accrue_yield(&1_000u128);

    // Bob deposits the same assets but, since each share is now worth more,
    // receives half as many shares as Alice did.
    let bob_shares = t.vault.deposit(&bob, &1_000u128);
    assert_eq!(bob_shares, 500);
    assert_eq!(t.vault.total_shares(), 1_500);
    assert_eq!(t.vault.total_assets(), 3_000);
}

#[test]
fn test_withdraw_more_than_balance_fails() {
    let t = VaultTest::setup();
    let user = Address::generate(&t.env);
    t.mint(&user, 1_000);
    let shares = t.vault.deposit(&user, &1_000u128);

    let res = t.vault.try_withdraw(&user, &(shares + 1));
    assert_eq!(res, Err(Ok(crate::Error::InsufficientShares)));
}

#[test]
fn test_zero_deposit_fails() {
    let t = VaultTest::setup();
    let user = Address::generate(&t.env);
    let res = t.vault.try_deposit(&user, &0u128);
    assert_eq!(res, Err(Ok(crate::Error::ZeroAmount)));
}

#[test]
fn test_zero_withdraw_fails() {
    let t = VaultTest::setup();
    let user = Address::generate(&t.env);
    let res = t.vault.try_withdraw(&user, &0u128);
    assert_eq!(res, Err(Ok(crate::Error::ZeroShares)));
}
