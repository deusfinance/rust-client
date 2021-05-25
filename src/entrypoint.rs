//! Program entrypoint

#![cfg(not(feature = "no-entrypoint"))]

use std::str::FromStr;

use crate::{error::SynchronizerError, processor::Processor};
use solana_program::{
    account_info::AccountInfo, entrypoint, entrypoint::ProgramResult, pubkey::Pubkey, program_error::PrintProgramError
};


entrypoint!(process_instruction);
fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let processor = Processor {
        synchronizer_key: Pubkey::from_str("D2YHis8gk2wRHkMEY7bULLsFUk277KdodWFR1nJ9SRgb").unwrap(),
        collateral_token_key : Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap(),
    };

    // Processor::process_instruction(program_id, accounts, instruction_data)
    if let Err(error) = processor.process_instruction(program_id, accounts, instruction_data) {
        error.print::<SynchronizerError>();
        return Err(error);
    }

    Ok(())
}
