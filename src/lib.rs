#![no_std]

//! # YieldVault
//!
//! A share-based (ERC4626-style) DeFi yield vault for the Soroban platform.
//!
//! Depositors supply an underlying token and receive vault shares that track
//! their proportional claim on the vault's assets. As the vault accrues yield
//! the value of each share grows, so withdrawing the same number of shares
//! later returns more of the underlying token.

mod error;
mod events;
mod math;
mod storage;
mod types;

#[cfg(test)]
mod test;

pub use error::Error;

use soroban_sdk::{contract, contractimpl, contractmeta, token, Address, Env};

contractmeta!(key = "Description", val = "Share-based ERC4626-style yield vault");

/// The YieldVault contract type.
#[contract]
pub struct YieldVault;

#[contractimpl]
impl YieldVault {
    /// Initializes the vault with its `admin` and the `token` it accepts as the
    /// underlying asset.
    ///
    /// Can only be called once; a second call returns
    /// [`Error::AlreadyInitialized`].
    pub fn initialize(env: Env, admin: Address, token: Address) -> Result<(), Error> {
        if storage::has_admin(&env) {
            return Err(Error::AlreadyInitialized);
        }
        storage::set_admin(&env, &admin);
        storage::set_token(&env, &token);
        storage::extend_instance(&env);
        events::initialize(&env, &admin, &token);
        Ok(())
    }

    /// Returns the vault administrator address.
    pub fn get_admin(env: Env) -> Result<Address, Error> {
        if !storage::has_admin(&env) {
            return Err(Error::NotInitialized);
        }
        Ok(storage::get_admin(&env))
    }

    /// Returns the underlying asset token address.
    pub fn get_token(env: Env) -> Result<Address, Error> {
        if !storage::has_admin(&env) {
            return Err(Error::NotInitialized);
        }
        Ok(storage::get_token(&env))
    }

    /// Returns the total number of shares minted by the vault.
    pub fn total_shares(env: Env) -> u128 {
        storage::get_total_shares(&env)
    }

    /// Returns the total amount of underlying assets held by the vault.
    pub fn total_assets(env: Env) -> u128 {
        storage::get_total_assets(&env)
    }

    /// Returns the share balance of `user`.
    pub fn balance_of(env: Env, user: Address) -> u128 {
        storage::get_balance(&env, &user)
    }

    /// Previews how many shares would be minted for depositing `assets` at the
    /// current exchange rate.
    pub fn convert_to_shares(env: Env, assets: u128) -> Result<u128, Error> {
        let total_shares = storage::get_total_shares(&env);
        let total_assets = storage::get_total_assets(&env);
        math::convert_to_shares(assets, total_shares, total_assets)
    }

    /// Previews how many underlying assets `shares` would redeem for at the
    /// current exchange rate.
    pub fn convert_to_assets(env: Env, shares: u128) -> Result<u128, Error> {
        let total_shares = storage::get_total_shares(&env);
        let total_assets = storage::get_total_assets(&env);
        math::convert_to_assets(shares, total_shares, total_assets)
    }

    /// Deposits `amount` of the underlying token from `from` into the vault,
    /// minting and returning the number of shares credited to `from`.
    ///
    /// Requires authorization from `from`. The underlying tokens are pulled
    /// from `from` into the vault via the token contract's `transfer`.
    pub fn deposit(env: Env, from: Address, amount: u128) -> Result<u128, Error> {
        if !storage::has_admin(&env) {
            return Err(Error::NotInitialized);
        }
        from.require_auth();

        if amount == 0 {
            return Err(Error::ZeroAmount);
        }

        let total_shares = storage::get_total_shares(&env);
        let total_assets = storage::get_total_assets(&env);
        let shares = math::convert_to_shares(amount, total_shares, total_assets)?;
        if shares == 0 {
            return Err(Error::ZeroShares);
        }

        let token_address = storage::get_token(&env);
        let client = token::Client::new(&env, &token_address);
        client.transfer(
            &from,
            &env.current_contract_address(),
            &(amount as i128),
        );

        let new_total_shares = total_shares
            .checked_add(shares)
            .ok_or(Error::MathOverflow)?;
        let new_total_assets = total_assets
            .checked_add(amount)
            .ok_or(Error::MathOverflow)?;
        let user_balance = storage::get_balance(&env, &from)
            .checked_add(shares)
            .ok_or(Error::MathOverflow)?;

        storage::set_total_shares(&env, new_total_shares);
        storage::set_total_assets(&env, new_total_assets);
        storage::set_balance(&env, &from, user_balance);
        storage::extend_instance(&env);

        events::deposit(&env, &from, amount, shares);
        Ok(shares)
    }

    /// Burns `shares` from `from` and returns the corresponding amount of
    /// underlying assets, transferring them back to `from`.
    ///
    /// Requires authorization from `from`. Returns [`Error::InsufficientShares`]
    /// if `from` does not hold enough shares.
    pub fn withdraw(env: Env, from: Address, shares: u128) -> Result<u128, Error> {
        if !storage::has_admin(&env) {
            return Err(Error::NotInitialized);
        }
        from.require_auth();

        if shares == 0 {
            return Err(Error::ZeroShares);
        }

        let user_balance = storage::get_balance(&env, &from);
        if user_balance < shares {
            return Err(Error::InsufficientShares);
        }

        let total_shares = storage::get_total_shares(&env);
        let total_assets = storage::get_total_assets(&env);
        let assets = math::convert_to_assets(shares, total_shares, total_assets)?;
        if assets == 0 {
            return Err(Error::ZeroAmount);
        }

        let new_total_shares = total_shares
            .checked_sub(shares)
            .ok_or(Error::MathOverflow)?;
        let new_total_assets = total_assets
            .checked_sub(assets)
            .ok_or(Error::MathOverflow)?;
        let new_user_balance = user_balance
            .checked_sub(shares)
            .ok_or(Error::MathOverflow)?;

        storage::set_total_shares(&env, new_total_shares);
        storage::set_total_assets(&env, new_total_assets);
        storage::set_balance(&env, &from, new_user_balance);

        let token_address = storage::get_token(&env);
        let client = token::Client::new(&env, &token_address);
        client.transfer(
            &env.current_contract_address(),
            &from,
            &(assets as i128),
        );

        storage::extend_instance(&env);
        events::withdraw(&env, &from, shares, assets);
        Ok(assets)
    }

    /// Mocks yield accrual by increasing the vault's total assets by `amount`
    /// without minting new shares, raising the value of every existing share.
    ///
    /// Admin-only: requires authorization from the configured admin address.
    pub fn accrue_yield(env: Env, amount: u128) -> Result<(), Error> {
        if !storage::has_admin(&env) {
            return Err(Error::NotInitialized);
        }
        let admin = storage::get_admin(&env);
        admin.require_auth();

        if amount == 0 {
            return Err(Error::ZeroAmount);
        }

        let total_assets = storage::get_total_assets(&env)
            .checked_add(amount)
            .ok_or(Error::MathOverflow)?;
        storage::set_total_assets(&env, total_assets);
        storage::extend_instance(&env);

        events::accrue_yield(&env, amount, total_assets);
        Ok(())
    }

    /// Returns the vault's advertised annual percentage yield, expressed in
    /// basis points (1% == 100 basis points).
    ///
    /// This is a fixed mock figure for demonstration purposes; a production
    /// vault would derive it from observed yield over time.
    pub fn get_apy(_env: Env) -> u32 {
        types::MOCK_APY_BPS
    }
}
