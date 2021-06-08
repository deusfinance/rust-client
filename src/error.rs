//! Error types

use num_derive::FromPrimitive;
use solana_program::{decode_error::DecodeError, program_error::ProgramError};
use thiserror::Error;

/// Errors that may be returned by the Synchronizer.
#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum SynchronizerError {
    /// Synchronizer account already initialized
    #[error("Synchronizer account already initialized")]
    AlreadyInitialized,
    /// Synchronizer account is not initialized
    #[error("Synchronizer account is not initialized")]
    NotInitialized,
    /// Lamport balance below rent-exempt threshold
    #[error("Lamport balance below rent-exempt threshold")]
    NotRentExempt,
    /// Insufficient funds
    #[error("Insufficient funds")]
    InsufficientFunds,
    /// Access denied
    #[error("Access denied")]
    AccessDenied,
    /// Not enough oracles
    #[error("Not enough oracles")]
    NotEnoughOracles,
    /// Signer is not an oracle
    #[error("Signer is not an oracle")]
    BadOracle,
    /// Bad mint authority
    #[error("Bad mint authority")]
    BadMintAuthority,
    /// Bad collateral mint
    #[error("Bad collateral mint")]
    BadCollateralMint,
    /// Bad mint decimals
    #[error("Bad mint decimals")]
    BadDecimals,
    /// Invalid Signer
    #[error("Invalid Signer")]
    InvalidSigner,
    /// Invalid instruction
    #[error("Invalid instruction")]
    InvalidInstruction,
    /// Exceed limit of maximum oracles
    #[error("Exceed limit of maximum oracles")]
    MaxOraclesExceed,
    /// Exceed limit of maximum signers
    #[error("Exceed limit of maximum signers")]
    MaxSignersExceed,
}

impl From<SynchronizerError> for ProgramError {
    fn from(e: SynchronizerError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl<T> DecodeError<T> for SynchronizerError {
    fn type_of() -> &'static str {
        "SynchronizerError"
    }
}
