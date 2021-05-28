//! Error types

use num_derive::FromPrimitive;
use solana_program::{decode_error::DecodeError, program_error::ProgramError};
use thiserror::Error;

/// Errors that may be returned by the Synchronizer.
#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum SynchronizerError {
    #[error("Synchronizer account already initialized")]
    AlreadyInitialized,
    #[error("Synchronizer account is not initialized")]
    NotInitialized,
    #[error("Lamport balance below rent-exempt threshold")]
    NotRentExempt,
    #[error("Insufficient funds")]
    InsufficientFunds,
    #[error("Access denied")]
    AccessDenied,
    #[error("Signer is not an oracle")]
    BadOracle,
    #[error("Bad mint authority")]
    BadMintAuthority,
    #[error("Bad mint decimals")]
    BadDecimals,
    #[error("Invalid Signer")]
    InvalidSigner,
    #[error("Invalid instruction")]
    InvalidInstruction,
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
