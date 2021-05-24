use crate::{error::SynchronizerError, instruction::{SynchronizerInstruction}};
use num_traits::FromPrimitive;
use solana_program::{account_info::{next_account_info, AccountInfo}, decode_error::DecodeError, entrypoint::ProgramResult, msg, native_token, program_error::{PrintProgramError, ProgramError}, pubkey::Pubkey};

pub const SCALE: u64 = 1_000_000_000; // 10^9

pub struct Processor {}
impl Processor {

pub fn process_buy_for(
    accounts: &[AccountInfo],
    multiplier: u64,
    amount: u64,
    fee: u64,
    prices: &Vec<u64>
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let user_info = next_account_info(account_info_iter)?;
    let mint_info = next_account_info(account_info_iter)?;
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

    let collateral_amount= amount as f64 * price as f64  / SCALE as f64;
    let fee_amount = collateral_amount as f64 * fee as f64 / SCALE as f64;

    // TODO:
    // remainingDollarCap = remainingDollarCap - (collateralAmount * multiplier);
    // withdrawableFeeAmount = withdrawableFeeAmount + feeAmount;

    // TODO:
    // collateralToken.transferFrom(msg.sender, address(this), collateralAmount + feeAmount); // dai transfer from user to syncronizer
	// Registrar(registrar).mint(_user, amount); // tesla mint to user by syncronizer

    Ok(())
}

pub fn process_sell_for(
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
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    msg!("Synchronizer entrypoint");

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
            Self::process_buy_for(accounts, multiplier, amount, fee, prices)
        }
        SynchronizerInstruction::SellFor {
            multiplier,
            amount,
            fee,
            ref prices
        } => {
            msg!("Instruction: SellFor");
            Self::process_sell_for(accounts, multiplier, amount, fee, prices)
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
            SynchronizerError::InvalidInstruction => msg!("Error: Invalid instruction")
        }
    }
}

// Unit tests
#[cfg(test)]
mod test {
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
