//! Pure share/asset conversion math for the YieldVault.
//!
//! All functions round down toward zero, which keeps rounding error in the
//! vault's favor and prevents depositors from extracting more value than they
//! contributed. Multiplication is checked to avoid silent overflow.

use crate::error::Error;

/// Computes `a * b / denominator`, rounding the result down toward zero.
///
/// Multiplication is checked so that an intermediate product exceeding the
/// `u128` range returns [`Error::MathOverflow`] rather than wrapping. A zero
/// `denominator` returns [`Error::DivisionByZero`].
pub fn mul_div(a: u128, b: u128, denominator: u128) -> Result<u128, Error> {
    if denominator == 0 {
        return Err(Error::DivisionByZero);
    }
    let product = a.checked_mul(b).ok_or(Error::MathOverflow)?;
    Ok(product / denominator)
}

/// Converts an amount of underlying `assets` into vault shares.
///
/// When the vault is empty (`total_shares == 0`), the first depositor receives
/// shares one-to-one with the assets supplied, bootstrapping the exchange rate.
/// Otherwise shares are minted proportionally: `assets * total_shares /
/// total_assets`, rounding down so the vault never mints more value than it
/// receives.
pub fn convert_to_shares(
    assets: u128,
    total_shares: u128,
    total_assets: u128,
) -> Result<u128, Error> {
    if total_shares == 0 || total_assets == 0 {
        return Ok(assets);
    }
    mul_div(assets, total_shares, total_assets)
}

/// Computes the value of a single share in underlying assets, scaled by
/// `scale` to preserve precision (since integer division would otherwise
/// truncate fractional share prices).
///
/// Returns `scale` when the vault is empty, reflecting the one-to-one
/// bootstrap exchange rate, and rounds down otherwise.
pub fn price_per_share(
    total_shares: u128,
    total_assets: u128,
    scale: u128,
) -> Result<u128, Error> {
    if total_shares == 0 {
        return Ok(scale);
    }
    mul_div(total_assets, scale, total_shares)
}

/// Computes what fraction of the vault `shares` represents, expressed in
/// basis points (`bps`, where `10_000` bps == 100%).
///
/// Returns zero when no shares exist, and rounds down otherwise so the figure
/// never overstates an account's claim on the vault.
pub fn share_fraction_bps(shares: u128, total_shares: u128, bps: u128) -> Result<u128, Error> {
    if total_shares == 0 {
        return Ok(0);
    }
    mul_div(shares, bps, total_shares)
}

/// Converts an amount of vault `shares` into the underlying assets they are
/// redeemable for: `shares * total_assets / total_shares`, rounding down.
///
/// When no shares exist the result is zero, since there is no claim on the
/// vault's assets.
pub fn convert_to_assets(
    shares: u128,
    total_shares: u128,
    total_assets: u128,
) -> Result<u128, Error> {
    if total_shares == 0 {
        return Ok(0);
    }
    mul_div(shares, total_assets, total_shares)
}
