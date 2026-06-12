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
