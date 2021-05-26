//! Error types

use num_derive::FromPrimitive;
use solana_program::{decode_error::DecodeError, program_error::ProgramError};
use thiserror::Error;

/// Errors that may be returned by the Synchronizer.
#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum SynchronizerError {
    #[error("Invalid Synchronizer key")]
    InvalidSynchronizerKey,
    #[error("Invalid Signer")]
    InvalidSigner,
    #[error("Invalid instruction")]
    InvalidInstruction,
    #[error("Failed mint token")]
    FailedMint,
    #[error("Failed burn token")]
    FailedBurn,
    #[error("Failed transfer token")]
    FailedTransfer,
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
