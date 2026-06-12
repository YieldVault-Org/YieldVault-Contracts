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

pub use error::Error;

use soroban_sdk::{contract, contractimpl, contractmeta, Address, Env};

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
}
