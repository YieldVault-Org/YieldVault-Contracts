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
}
