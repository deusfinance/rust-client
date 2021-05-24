//! Program entrypoint

#![cfg(not(feature = "no-entrypoint"))]

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
    // Processor::process_instruction(program_id, accounts, instruction_data)
    if let Err(error) = Processor::process_instruction(program_id, accounts, instruction_data) {
        error.print::<SynchronizerError>();
        return Err(error);
    }
    Ok(())
}
