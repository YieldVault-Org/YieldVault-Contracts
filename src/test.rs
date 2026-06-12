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

#[test]
fn test_mul_div_rounds_down() {
    use crate::math::mul_div;
    // 7 * 3 / 2 == 10.5, rounded down to 10.
    assert_eq!(mul_div(7, 3, 2), Ok(10));
}

#[test]
fn test_mul_div_division_by_zero() {
    use crate::math::mul_div;
    assert_eq!(mul_div(1, 1, 0), Err(crate::Error::DivisionByZero));
}

#[test]
fn test_mul_div_overflow() {
    use crate::math::mul_div;
    assert_eq!(mul_div(u128::MAX, 2, 1), Err(crate::Error::MathOverflow));
}

#[test]
fn test_price_per_share_helper() {
    use crate::math::price_per_share;
    // Empty vault reports the bootstrap price of exactly one scaled unit.
    assert_eq!(price_per_share(0, 0, 1_000), Ok(1_000));
    // With assets equal to shares the price is one scaled unit per share.
    assert_eq!(price_per_share(1_000, 1_000, 1_000), Ok(1_000));
    // Doubling assets without new shares doubles the per-share price.
    assert_eq!(price_per_share(1_000, 2_000, 1_000), Ok(2_000));
}

#[test]
fn test_price_per_share_view_tracks_yield() {
    let t = VaultTest::setup();
    let user = Address::generate(&t.env);
    t.mint(&user, 1_000);

    // Empty vault prices a share at exactly one whole scaled asset.
    assert_eq!(t.vault.price_per_share(), 1_000_000_000);

    t.vault.deposit(&user, &1_000u128);
    assert_eq!(t.vault.price_per_share(), 1_000_000_000);

    // Accrued yield doubles assets, so each share is worth twice as much.
    t.mint(&t.vault.address, 1_000);
    t.vault.accrue_yield(&1_000u128);
    assert_eq!(t.vault.price_per_share(), 2_000_000_000);
}

#[test]
fn test_convert_helpers_on_empty_vault() {
    use crate::math::{convert_to_assets, convert_to_shares};
    // First deposit bootstraps one-to-one; redeeming against no shares is zero.
    assert_eq!(convert_to_shares(100, 0, 0), Ok(100));
    assert_eq!(convert_to_assets(100, 0, 0), Ok(0));
}

#[test]
fn test_preview_matches_deposit() {
    let t = VaultTest::setup();
    let user = Address::generate(&t.env);
    t.mint(&user, 1_000);

    // The convert_to_shares preview should match the shares actually minted.
    let preview = t.vault.convert_to_shares(&400u128);
    let minted = t.vault.deposit(&user, &400u128);
    assert_eq!(preview, minted);
}

#[test]
fn test_partial_withdraw_keeps_remaining_shares() {
    let t = VaultTest::setup();
    let user = Address::generate(&t.env);
    t.mint(&user, 1_000);

    let shares = t.vault.deposit(&user, &1_000u128);
    let half = shares / 2;
    let assets = t.vault.withdraw(&user, &half);

    assert_eq!(assets, 500);
    assert_eq!(t.vault.balance_of(&user), half);
    assert_eq!(t.vault.total_shares(), half);
    assert_eq!(t.vault.total_assets(), 500);
    assert_eq!(t.token.balance(&user), 500);
}

#[test]
fn test_deposit_before_initialize_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let vault_address = env.register(YieldVault, ());
    let vault = YieldVaultClient::new(&env, &vault_address);

    let user = Address::generate(&env);
    let res = vault.try_deposit(&user, &100u128);
    assert_eq!(res, Err(Ok(crate::Error::NotInitialized)));
}

#[test]
fn test_is_initialized_reflects_setup_state() {
    let env = Env::default();
    let vault_address = env.register(YieldVault, ());
    let vault = YieldVaultClient::new(&env, &vault_address);

    // Not yet initialized.
    assert!(!vault.is_initialized());

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    vault.initialize(&admin, &token);

    // Now reports initialized and exposes the contract version.
    assert!(vault.is_initialized());
    assert_eq!(vault.version(), 1);
}

#[test]
fn test_max_withdraw_matches_share_value() {
    let t = VaultTest::setup();
    let user = Address::generate(&t.env);
    t.mint(&user, 1_000);
    t.vault.deposit(&user, &1_000u128);

    // With no yield the full balance redeems for the deposited assets.
    assert_eq!(t.vault.max_withdraw(&user), 1_000);

    t.mint(&t.vault.address, 500);
    t.vault.accrue_yield(&500u128);

    // After yield the redeemable amount grows with the share price.
    assert_eq!(t.vault.max_withdraw(&user), 1_500);
}

#[test]
fn test_set_admin_transfers_role() {
    let t = VaultTest::setup();
    let new_admin = Address::generate(&t.env);

    assert_eq!(t.vault.get_admin(), t.admin);

    // Transfer the admin role to a new address.
    t.vault.set_admin(&new_admin);
    assert_eq!(t.vault.get_admin(), new_admin);
}

#[test]
fn test_pause_blocks_deposit_but_allows_withdraw() {
    let t = VaultTest::setup();
    let user = Address::generate(&t.env);
    t.mint(&user, 2_000);

    let shares = t.vault.deposit(&user, &1_000u128);
    assert!(!t.vault.is_paused());

    // Admin pauses the vault.
    t.vault.set_paused(&true);
    assert!(t.vault.is_paused());

    // New deposits are rejected while paused.
    let res = t.vault.try_deposit(&user, &1_000u128);
    assert_eq!(res, Err(Ok(crate::Error::Paused)));

    // Withdrawals remain available so depositors can always exit.
    let assets = t.vault.withdraw(&user, &shares);
    assert_eq!(assets, 1_000);

    // Resuming the vault re-enables deposits.
    t.vault.set_paused(&false);
    assert!(!t.vault.is_paused());
    let again = t.vault.deposit(&user, &1_000u128);
    assert_eq!(again, 1_000);
}

#[test]
fn test_min_deposit_guard_rejects_small_deposits() {
    let t = VaultTest::setup();
    let user = Address::generate(&t.env);
    t.mint(&user, 1_000);

    // Admin raises the minimum deposit above a small amount.
    t.vault.set_min_deposit(&100u128);
    assert_eq!(t.vault.get_min_deposit(), 100);

    // A deposit under the minimum is rejected.
    let res = t.vault.try_deposit(&user, &50u128);
    assert_eq!(res, Err(Ok(crate::Error::BelowMinimumDeposit)));

    // A deposit at the minimum succeeds.
    let shares = t.vault.deposit(&user, &100u128);
    assert_eq!(shares, 100);
}

#[test]
fn test_share_fraction_bps_helper() {
    use crate::math::share_fraction_bps;
    // No shares means no claim, reported as zero.
    assert_eq!(share_fraction_bps(0, 0, 10_000), Ok(0));
    // Holding all shares is a full 100% (10_000 bps).
    assert_eq!(share_fraction_bps(1_000, 1_000, 10_000), Ok(10_000));
    // Holding a quarter of the shares is 2_500 bps.
    assert_eq!(share_fraction_bps(250, 1_000, 10_000), Ok(2_500));
}

#[test]
fn test_share_percentage_splits_between_depositors() {
    let t = VaultTest::setup();
    let alice = Address::generate(&t.env);
    let bob = Address::generate(&t.env);
    t.mint(&alice, 3_000);
    t.mint(&bob, 1_000);

    t.vault.deposit(&alice, &3_000u128);
    t.vault.deposit(&bob, &1_000u128);

    // Alice owns three quarters of the vault, Bob the remaining quarter.
    assert_eq!(t.vault.share_percentage(&alice), 7_500);
    assert_eq!(t.vault.share_percentage(&bob), 2_500);
}

#[test]
fn test_preview_getters_match_conversions() {
    let t = VaultTest::setup();
    let user = Address::generate(&t.env);
    t.mint(&user, 1_000);
    t.vault.deposit(&user, &1_000u128);

    // The preview aliases must agree with the underlying convert helpers.
    assert_eq!(
        t.vault.preview_deposit(&500u128),
        t.vault.convert_to_shares(&500u128)
    );
    assert_eq!(
        t.vault.preview_withdraw(&500u128),
        t.vault.convert_to_assets(&500u128)
    );
}

#[test]
fn test_max_redeem_returns_share_balance() {
    let t = VaultTest::setup();
    let user = Address::generate(&t.env);
    t.mint(&user, 1_000);
    let shares = t.vault.deposit(&user, &1_000u128);

    // max_redeem reports the caller's full share balance.
    assert_eq!(t.vault.max_redeem(&user), shares);

    // Yield grows the asset value but leaves the redeemable share count fixed.
    t.mint(&t.vault.address, 500);
    t.vault.accrue_yield(&500u128);
    assert_eq!(t.vault.max_redeem(&user), shares);
    assert_eq!(t.vault.max_withdraw(&user), 1_500);
}

#[test]
fn test_get_admin_before_initialize_fails() {
    let env = Env::default();
    let vault_address = env.register(YieldVault, ());
    let vault = YieldVaultClient::new(&env, &vault_address);

    let res = vault.try_get_admin();
    assert_eq!(res, Err(Ok(crate::Error::NotInitialized)));
}
