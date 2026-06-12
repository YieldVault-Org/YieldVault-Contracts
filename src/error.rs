use soroban_sdk::contracterror;

/// Errors that can be returned by the YieldVault contract.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// The contract has already been initialized.
    AlreadyInitialized = 1,
    /// The contract has not been initialized yet.
    NotInitialized = 2,
    /// An arithmetic operation overflowed the supported integer range.
    MathOverflow = 3,
    /// A division by zero was attempted.
    DivisionByZero = 4,
}
