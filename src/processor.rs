use crate::{error::SynchronizerError, instruction::{SynchronizerInstruction}};
use num_traits::FromPrimitive;
use solana_program::{account_info::{next_account_info, AccountInfo}, decode_error::DecodeError, entrypoint::ProgramResult, msg, program_error::{PrintProgramError, ProgramError}, pubkey::Pubkey};
use spl_token::instruction::{mint_to, transfer};
use spl_associated_token_account::get_associated_token_address;

// Synchronizer program_id
solana_program::declare_id!("9kyqhSRNj1C8jNBLM4KncjmXS1Tfac7p5o1ztdaJWMbz");

/// Checks that the supplied program ID is the correct
pub fn check_program_account(spl_token_program_id: &Pubkey) -> ProgramResult {
    if spl_token_program_id != &id() {
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}

pub const SCALE: u64 = 1_000_000_000; // 10^9

// Constant state initialized in deploy
pub struct Processor {
    // Synchronizer account key
    pub synchronizer_key : Pubkey,
    // USDC Token address
    pub collateral_token_key: Pubkey,
}

impl Processor {

pub fn process_buy_for(
    &self,
    accounts: &[AccountInfo],
    multiplier: u64,
    amount: u64,
    fee: u64,
    prices: &Vec<u64>
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let user_info = next_account_info(account_info_iter)?;
    let asset_mint_info = next_account_info(account_info_iter)?;
    // let mut last_oracle =
    for oracle in account_info_iter.as_slice() {
        // TODO: check oracle role
        // TODO: check oracle unique
    }

    let mut price = prices[0];
    for &p in prices {
        if p > price {
            price = p;
        }
    }

    let collateral_amount= ((amount * price) as f64  / SCALE as f64) as u64;
    let fee_amount = ((collateral_amount * fee) as f64 / SCALE as f64) as u64;

    // TODO:
    // remainingDollarCap = remainingDollarCap - (collateralAmount * multiplier);
    // withdrawableFeeAmount = withdrawableFeeAmount + feeAmount;

    // Find all token_associated_accounts
    let user_collateral_key = get_associated_token_address(user_info.key, &self.collateral_token_key);
    let user_asset_key = get_associated_token_address(user_info.key, asset_mint_info.key);
    let synchronizer_collateral_key = get_associated_token_address(&self.synchronizer_key, &self.collateral_token_key);

    // 1. User send collateral token to synchronizer
    if transfer(
        &spl_token::id(),
        &user_collateral_key,
        &synchronizer_collateral_key,
        user_info.key,
        &[],
        collateral_amount + fee_amount
    ).is_err() {
        return Err(SynchronizerError::FailedTransfer.into())
    }

    // 2. Synchronizer mint fiat asset to user associated token account
    if mint_to(
        &spl_token::id(),
        asset_mint_info.key,
        &user_asset_key,
        &self.synchronizer_key,
        &[],
        amount
    ).is_err() {
        return Err(SynchronizerError::FailedMint.into());
    }

    Ok(())
}

pub fn process_sell_for(
    &self,
    accounts: &[AccountInfo],
    multiplier: u64,
    amount: u64,
    fee: u64,
    prices: &Vec<u64>
) -> ProgramResult {
    // TODO: process instruction
    Ok(())
}

pub fn process_instruction(
    &self,
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    msg!("Synchronizer entrypoint");
    check_program_account(program_id)?;

    let instruction = SynchronizerInstruction::unpack(instruction_data)?;
    match instruction {
        // Public instructions
        SynchronizerInstruction::BuyFor {
            multiplier,
            amount,
            fee,
            ref prices
        } => {
            msg!("Instruction: BuyFor");
            self.process_buy_for(accounts, multiplier, amount, fee, prices)
        }
        SynchronizerInstruction::SellFor {
            multiplier,
            amount,
            fee,
            ref prices
        } => {
            msg!("Instruction: SellFor");
            self.process_sell_for(accounts, multiplier, amount, fee, prices)
        }

        // Admin Instructions
        SynchronizerInstruction::SetMinimumRequiredSignature => {
            msg!("Instruction: SetMinimumRequiredSignature");
            Ok(())
        }
        SynchronizerInstruction::SetCollateralToken => {
            msg!("Instruction: SetCollateralToken");
            Ok(())
        }
        SynchronizerInstruction::SetRemainingDollarCap => {
            msg!("Instruction: SetRemainingDollarCap");
            Ok(())
        }
        SynchronizerInstruction::WithdrawFee => {
            msg!("Instruction: WithdrawFee");
            Ok(())
        }
        SynchronizerInstruction::WithdrawCollateral => {
            msg!("Instruction: WithdrawCollateral");
            Ok(())
        }
    }
}

} // impl Processor

impl PrintProgramError for SynchronizerError {
    fn print<E>(&self)
    where
        E: 'static + std::error::Error + DecodeError<E> + PrintProgramError + FromPrimitive,
    {
        match self {
            SynchronizerError::InvalidInstruction => msg!("Error: Invalid instruction"),
            SynchronizerError::FailedMint => msg!("Error: Failed mint token"),
            SynchronizerError::FailedTransfer => msg!("Error: Failed transfer token"),
        }
    }
}

// Unit tests
#[cfg(test)]
mod test {
    use solana_program::program_error::ProgramError;
    use super::*;

    #[test]
    fn synchronizer_tests() {
        // TODO
    }

    #[test]
    fn test_print_error() {
        fn return_synchronizer_error_as_program_error() -> ProgramError {
            SynchronizerError::InvalidInstruction.into()
        }

        let error = return_synchronizer_error_as_program_error();
        error.print::<SynchronizerError>();
    }
}
