//! Program entrypoint

#![cfg(not(feature = "no-entrypoint"))]

use crate::{error::SynchronizerError, processor::Processor};
use solana_program::{
    account_info::AccountInfo,
    entrypoint,
    entrypoint::ProgramResult,
    pubkey::Pubkey,
    program_error::PrintProgramError
};
use std::str::FromStr;


entrypoint!(process_instruction);
fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let processor = Processor {
        synchronizer_key: Pubkey::from_str("D2YHis8gk2wRHkMEY7bULLsFUk277KdodWFR1nJ9SRgb").unwrap(),
        oracles_keys : [
            Pubkey::from_str("oracle1_key").unwrap(),
            Pubkey::from_str("oracle2_key").unwrap(),
            Pubkey::from_str("oracle2_key").unwrap(),
        ].iter().cloned().collect(),
    };

    // Processor::process_instruction(program_id, accounts, instruction_data)
    if let Err(error) = processor.process_instruction(program_id, accounts, instruction_data) {
        error.print::<SynchronizerError>();
        return Err(error);
    }

    Ok(())
}
