use crate::{error::SynchronizerError, instruction::{SynchronizerInstruction}};
use num_traits::FromPrimitive;
use solana_program::{account_info::{next_account_info, AccountInfo}, decode_error::DecodeError, entrypoint::ProgramResult, instruction::Instruction, msg, program_error::{PrintProgramError, ProgramError}, pubkey::Pubkey};
use spl_token::{instruction::{mint_to, burn, transfer}};
use spl_token::processor::Processor as SPLTokenProcessor;

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
    // Syncronizer USDC Token associated address
}

impl Processor {

fn process_token_instruction(
    &self,
    instruction: Instruction,
    instruction_account_infos: &[AccountInfo]
) -> ProgramResult {
    SPLTokenProcessor::process(&spl_token::id(), &instruction_account_infos, &instruction.data)
}

pub fn process_buy_for(
    &self,
    accounts: &[AccountInfo],
    multiplier: u64,
    amount: u64,
    fee: u64,
    prices: &Vec<u64>
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let fiat_asset_mint_info = next_account_info(account_info_iter)?;
    let user_collateral_account_info = next_account_info(account_info_iter)?;
    let user_fiat_account_info = next_account_info(account_info_iter)?;
    let synchronizer_collateral_account_info = next_account_info(account_info_iter)?;
    let user_authority_info = next_account_info(account_info_iter)?;
    let synchronizer_authority_info = next_account_info(account_info_iter)?;

    if !user_authority_info.is_signer {
        return Err(SynchronizerError::InvalidSigner.into());
    }

    if !synchronizer_authority_info.is_signer {
        return Err(SynchronizerError::InvalidSigner.into());
    }

    if !synchronizer_authority_info.key.eq(&self.synchronizer_key)  {
        return Err(SynchronizerError::InvalidSynchronizerKey.into());
    }

    // TODO: check oracles vector
    let mut price = prices[0];
    for &p in prices {
        if p > price {
            price = p;
        }
    }

    // TODO: Fix calculations
    let collateral_amount= ((amount * price) as f64  / SCALE as f64) as u64;
    let fee_amount = ((collateral_amount * fee) as f64 / SCALE as f64) as u64;
    msg!("collateral_amount: {}, fee_amount: {}", collateral_amount, fee_amount);

    // TODO:
    // remainingDollarCap = remainingDollarCap - (collateralAmount * multiplier);
    // withdrawableFeeAmount = withdrawableFeeAmount + feeAmount;

    // User send collateral token to synchronizer
    let instruction = transfer(
        &spl_token::id(),
        &user_collateral_account_info.key,
        &synchronizer_collateral_account_info.key,
        &user_authority_info.key,
        &[],
        collateral_amount + fee_amount
    ).unwrap();
    let account_infos = [
        user_collateral_account_info.clone(),
        synchronizer_collateral_account_info.clone(),
        user_authority_info.clone(),
    ];
    if self.process_token_instruction(instruction, &account_infos).is_err() {
        return Err(SynchronizerError::FailedTransfer.into());
    }

    // Synchronizer mint fiat asset to user associated token account
    let instruction = mint_to(
        &spl_token::id(),
        &fiat_asset_mint_info.key,
        &user_fiat_account_info.key,
        &self.synchronizer_key,
        &[],
        amount
    ).unwrap();
    let account_infos = [
        fiat_asset_mint_info.clone(),
        user_fiat_account_info.clone(),
        synchronizer_authority_info.clone(),
    ];
    if self.process_token_instruction(instruction, &account_infos).is_err() {
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
    let account_info_iter = &mut accounts.iter();
    let fiat_asset_mint_info = next_account_info(account_info_iter)?;
    let user_collateral_account_info = next_account_info(account_info_iter)?;
    let user_fiat_account_info = next_account_info(account_info_iter)?;
    let synchronizer_collateral_account_info = next_account_info(account_info_iter)?;
    let user_authority_info = next_account_info(account_info_iter)?;
    let synchronizer_authority_info = next_account_info(account_info_iter)?;

    if !user_authority_info.is_signer {
        return Err(SynchronizerError::InvalidSigner.into());
    }

    if !synchronizer_authority_info.is_signer {
        return Err(SynchronizerError::InvalidSigner.into());
    }

    if !synchronizer_authority_info.key.eq(&self.synchronizer_key)  {
        return Err(SynchronizerError::InvalidSynchronizerKey.into());
    }

    // TODO: check oracles vector
    let mut price = prices[0];
    for &p in prices {
        if p < price {
            price = p;
        }
    }

    // TODO: Fix calculations
    let collateral_amount= ((amount * price) as f64  / SCALE as f64) as u64;
    let fee_amount = ((collateral_amount * fee) as f64 / SCALE as f64) as u64;
    msg!("collateral_amount: {}, fee_amount: {}", collateral_amount, fee_amount);

    // TODO:
    // remainingDollarCap = remainingDollarCap + (collateralAmount * multiplier);
    // withdrawableFeeAmount = withdrawableFeeAmount + feeAmount; // Добытый фи

    // Burn fiat asset from user
    let instruction = burn (
        &spl_token::id(),
        &user_fiat_account_info.key,
        &fiat_asset_mint_info.key,
        &user_authority_info.key,
        &[],
        amount
    ).unwrap();
    let account_infos = [
        user_fiat_account_info.clone(),
        fiat_asset_mint_info.clone(),
        user_authority_info.clone(),
    ];
    if self.process_token_instruction(instruction, &account_infos).is_err() {
        return Err(SynchronizerError::FailedBurn.into());
    }

    // Transfer collateral token from synchronizer to user
    let instruction = transfer(
        &spl_token::id(),
        &synchronizer_collateral_account_info.key,
        &user_collateral_account_info.key,
        &self.synchronizer_key,
        &[],
        collateral_amount - fee_amount
    ).unwrap();
    let account_infos = [
        synchronizer_collateral_account_info.clone(),
        user_collateral_account_info.clone(),
        synchronizer_authority_info.clone(),
    ];
    if self.process_token_instruction(instruction, &account_infos).is_err() {
        return Err(SynchronizerError::FailedTransfer.into());
    }

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
            SynchronizerError::InvalidSigner => msg!("Error: Invalid transaction Signer"),
            SynchronizerError::InvalidSynchronizerKey => msg!("Error: Invalid Synchronizer key"),
            SynchronizerError::FailedBurn => msg!("Error: Failed burn token"),
        }
    }
}

// Unit tests
#[cfg(test)]
mod test {
    use solana_program::account_info::IntoAccountInfo;
    use solana_program::instruction::Instruction;
    use solana_program::rent::Rent;
    use solana_program::{program_error::ProgramError, program_pack::Pack};
    use solana_sdk::account::create_is_signer_account_infos;
    use solana_sdk::account::{Account as SolanaAccount, create_account_for_test};
    use spl_token::instruction::initialize_account;
    use spl_token::instruction::initialize_mint;
    use spl_token::state::{Account, Mint};
    use spl_token::processor::Processor as SPLTokenProcessor;
    use super::*;

    fn mint_minimum_balance() -> u64 {
        Rent::default().minimum_balance(Mint::get_packed_len())
    }

    fn account_minimum_balance() -> u64 {
        Rent::default().minimum_balance(Account::get_packed_len())
    }

    fn do_token_program(
        instruction: Instruction,
        accounts: Vec<&mut SolanaAccount>,
    ) -> ProgramResult {
        let mut meta = instruction
            .accounts
            .iter()
            .zip(accounts)
            .map(|(account_meta, account)| (&account_meta.pubkey, account_meta.is_signer, account))
            .collect::<Vec<_>>();

        let account_infos = create_is_signer_account_infos(&mut meta);
        SPLTokenProcessor::process(&instruction.program_id, &account_infos, &instruction.data)
    }

    #[test]
    fn test_public_api() {
        let synchronizer_key = Pubkey::new_unique();
        let mut synchronizer_account = SolanaAccount::default();
        let user_key = Pubkey::new_unique();
        let mut user_account = SolanaAccount::default();

        // Create and init collateral token
        let collateral_key = Pubkey::new_unique();
        let mut collateral_asset_mint = SolanaAccount::new(mint_minimum_balance(), Mint::get_packed_len(), &spl_token::id());
        let mut rent_sysvar = create_account_for_test(&Rent::default());
        do_token_program(
            initialize_mint(&spl_token::id(), &collateral_key, &synchronizer_key, None, 6).unwrap(),
            vec![&mut collateral_asset_mint, &mut rent_sysvar],
        ).unwrap();

        // Create and init fiat asset token
        let fiat_asset_key = Pubkey::new_unique();
        let mut fiat_asset_mint = SolanaAccount::new(mint_minimum_balance(), Mint::get_packed_len(), &synchronizer_key);
        do_token_program(
            initialize_mint(&spl_token::id(), &fiat_asset_key, &synchronizer_key, None, 2).unwrap(),
            vec![&mut fiat_asset_mint, &mut rent_sysvar],
        ).unwrap();

        // Create and init token associated accounts for synchronizer
        let synchronizer_collateral_key = Pubkey::new_unique();
        let mut synchronizer_collateral_account = SolanaAccount::new(account_minimum_balance(), Account::get_packed_len(), &spl_token::id());
        do_token_program(
            initialize_account(&spl_token::id(), &synchronizer_collateral_key, &collateral_key, &synchronizer_key).unwrap(),
            vec![
                &mut synchronizer_collateral_account,
                &mut collateral_asset_mint,
                &mut synchronizer_account,
                &mut rent_sysvar,
            ],
        ).unwrap();

        // Create token associated accounts for user
        let user_collateral_key = Pubkey::new_unique();
        let mut user_collateral_account = SolanaAccount::new(account_minimum_balance(), Account::get_packed_len(), &spl_token::id());
        do_token_program(
            initialize_account(&spl_token::id(), &user_collateral_key, &collateral_key, &user_key).unwrap(),
            vec![
                &mut user_collateral_account,
                &mut collateral_asset_mint,
                &mut user_account,
                &mut rent_sysvar,
            ],
        ).unwrap();

        let user_fiat_key = Pubkey::new_unique();
        let mut user_fiat_account = SolanaAccount::new(account_minimum_balance(), Account::get_packed_len(), &spl_token::id());
        do_token_program(
            initialize_account(&spl_token::id(), &user_fiat_key, &fiat_asset_key, &user_key).unwrap(),
            vec![
                &mut user_fiat_account,
                &mut fiat_asset_mint,
                &mut user_account,
                &mut rent_sysvar,
            ],
        ).unwrap();

        // TODO need not null balances for token accounts

        // Test buy_for instruction
        let processor = Processor {
            synchronizer_key: synchronizer_key,
            collateral_token_key: collateral_key,
        };

        // TODO: more bad cases for buy_for
        { // Case: bad user signer
            let bad_accounts = vec![
                (&fiat_asset_key, &mut fiat_asset_mint).into_account_info(),
                (&user_collateral_key, &mut user_collateral_account).into_account_info(),
                (&user_fiat_key, &mut user_fiat_account).into_account_info(),
                (&synchronizer_collateral_key, &mut synchronizer_collateral_account).into_account_info(),
                (&user_key, false, &mut user_account).into_account_info(),
                (&synchronizer_key, true, &mut synchronizer_account).into_account_info(), // wrong sync acc
            ];
            assert_eq!(
                Err(SynchronizerError::InvalidSigner.into()),
                processor.process_buy_for(&bad_accounts, 100, 100, 20, &vec![20, 30])
            );
        }

        { // Case: bad synchronizer signer
            let fake_synchronizer_key = Pubkey::new_unique();
            let mut fake_synchronizer_account = SolanaAccount::default();
            let bad_accounts = vec![
                (&fiat_asset_key, &mut fiat_asset_mint).into_account_info(),
                (&user_collateral_key, &mut user_collateral_account).into_account_info(),
                (&user_fiat_key, &mut user_fiat_account).into_account_info(),
                (&synchronizer_collateral_key, &mut synchronizer_collateral_account).into_account_info(),
                (&user_key, true, &mut user_account).into_account_info(), // user is not signer
                (&fake_synchronizer_key, true, &mut fake_synchronizer_account).into_account_info(),
            ];
            assert_eq!(
                Err(SynchronizerError::InvalidSynchronizerKey.into()),
                processor.process_buy_for(&bad_accounts, 100, 100, 20, &vec![20, 30])
            );
        }

        {
            let accounts = vec![
                (&fiat_asset_key, &mut fiat_asset_mint).into_account_info(),
                (&user_collateral_key, &mut user_collateral_account).into_account_info(),
                (&user_fiat_key, &mut user_fiat_account).into_account_info(),
                (&synchronizer_collateral_key, &mut synchronizer_collateral_account).into_account_info(),
                (&user_key, true, &mut user_account).into_account_info(),
                (&synchronizer_key, true, &mut synchronizer_account).into_account_info(),
            ];
            processor.process_buy_for(&accounts, 100, 100, 20, &vec![20, 30]).unwrap();
        }

        // Test sell_for instruction
        // TODO: more bad cases
        { // Case: bad user signer
            let bad_accounts = vec![
                (&fiat_asset_key, &mut fiat_asset_mint).into_account_info(),
                (&user_collateral_key, &mut user_collateral_account).into_account_info(),
                (&user_fiat_key, &mut user_fiat_account).into_account_info(),
                (&synchronizer_collateral_key, &mut synchronizer_collateral_account).into_account_info(),
                (&user_key, false, &mut user_account).into_account_info(), // user is not signer
                (&synchronizer_key, true, &mut synchronizer_account).into_account_info(),
            ];
            assert_eq!(
                Err(SynchronizerError::InvalidSigner.into()),
                processor.process_sell_for(&bad_accounts, 100, 100, 20, &vec![20, 30])
            );
        }

        { // Case: bad synchronizer signer
            let fake_synchronizer_key = Pubkey::new_unique();
            let mut fake_synchronizer_account = SolanaAccount::default();
            let bad_accounts = vec![
                (&fiat_asset_key, &mut fiat_asset_mint).into_account_info(),
                (&user_collateral_key, &mut user_collateral_account).into_account_info(),
                (&user_fiat_key, &mut user_fiat_account).into_account_info(),
                (&synchronizer_collateral_key, &mut synchronizer_collateral_account).into_account_info(),
                (&user_key, true, &mut user_account).into_account_info(),
                (&fake_synchronizer_key, true, &mut fake_synchronizer_account).into_account_info(), // wrong sync acc
            ];
            assert_eq!(
                Err(SynchronizerError::InvalidSynchronizerKey.into()),
                processor.process_sell_for(&bad_accounts, 100, 100, 20, &vec![20, 30])
            );
        }

        assert_eq!(
            user_key,
            Account::unpack_unchecked(&user_fiat_account.data).unwrap().owner
        );

        {
            let accounts = vec![
                (&fiat_asset_key, &mut fiat_asset_mint).into_account_info(),
                (&user_collateral_key, &mut user_collateral_account).into_account_info(),
                (&user_fiat_key, &mut user_fiat_account).into_account_info(),
                (&synchronizer_collateral_key, &mut synchronizer_collateral_account).into_account_info(),
                (&user_key, true, &mut user_account).into_account_info(),
                (&synchronizer_key, true, &mut synchronizer_account).into_account_info(),
            ];
            processor.process_sell_for(&accounts, 100, 100, 20, &vec![20, 30]).unwrap();
        }
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
