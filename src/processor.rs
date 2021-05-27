use crate::{
    error::SynchronizerError,
    instruction::{SynchronizerInstruction},
    state::SynchronizerData
};
use num_traits::FromPrimitive;
use solana_program::{account_info::{next_account_info, AccountInfo}, decode_error::DecodeError, entrypoint::ProgramResult, instruction::Instruction, msg, program_error::{PrintProgramError, ProgramError}, program_option::COption, program_pack::Pack, pubkey::Pubkey, rent::Rent, sysvar::Sysvar};
use spl_token::{instruction::{mint_to, burn, transfer}, processor::Processor as SPLTokenProcessor, state::Mint};
use std::collections::HashSet;

// Synchronizer program_id
solana_program::declare_id!("8nNo8sjfYvwouTPQXw5fJ2D6DWzcWsbeXQanDGELt4AG");

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
    // Set of Oracle pubkeys
    pub oracles_keys: HashSet<Pubkey>,
}

impl Processor {

fn process_token_instruction(
    &self,
    instruction: Instruction,
    instruction_account_infos: &[AccountInfo]
) -> ProgramResult {
    SPLTokenProcessor::process(&spl_token::id(), &instruction_account_infos, &instruction.data)
}

pub fn process_initialize_synchronizer_account(
    &self,
    accounts: &[AccountInfo],
    collateral_token_key: Pubkey,
    remaining_dollar_cap: u64,
    withdrawable_fee_amount: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let synchronizer_account_info = next_account_info(account_info_iter)?;
    let rent_account_info =next_account_info(account_info_iter)?;

    if !synchronizer_account_info.owner.eq(&id()) {
        return Err(SynchronizerError::AccessDenied.into());
    }

    if !synchronizer_account_info.key.eq(&self.synchronizer_key)  {
        return Err(SynchronizerError::AccessDenied.into());
    }

    let rent = &Rent::from_account_info(rent_account_info)?;
    let account_data_len = synchronizer_account_info.data_len();
    let mut synchronizer = SynchronizerData::unpack_unchecked(&synchronizer_account_info.data.borrow())?;
    if synchronizer.is_initialized {
        return Err(SynchronizerError::AlreadyInitialized.into());
    }

    if !rent.is_exempt(synchronizer_account_info.lamports(), account_data_len) {
        return Err(SynchronizerError::NotRentExempt.into());
    }

    synchronizer.is_initialized = true;
    synchronizer.remaining_dollar_cap = remaining_dollar_cap;
    synchronizer.collateral_token_key = collateral_token_key;
    synchronizer.withdrawable_fee_amount = withdrawable_fee_amount;

    SynchronizerData::pack(synchronizer, &mut synchronizer_account_info.data.borrow_mut())?;

    Ok(())
}

pub fn process_buy_for(
    &self,
    accounts: &[AccountInfo],
    multiplier: u64,
    amount: u64,
    fee: u64,
    prices: &Vec<u64>,
    oracles: &Vec<Pubkey>
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
        return Err(SynchronizerError::InvalidSigner.into());
    }

    for oracle in oracles {
        if !self.oracles_keys.contains(oracle) {
            return Err(SynchronizerError::BadOracle.into());
        }
    }

    let mut synchronizer = SynchronizerData::unpack_unchecked(&synchronizer_authority_info.data.borrow())?;
    if !synchronizer.is_initialized {
        return Err(SynchronizerError::NotInitialized.into());
    }

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

    synchronizer.remaining_dollar_cap = synchronizer.remaining_dollar_cap - (collateral_amount * multiplier);
    synchronizer.withdrawable_fee_amount = synchronizer.withdrawable_fee_amount + fee_amount;

    match Mint::unpack(&fiat_asset_mint_info.data.borrow_mut())?.mint_authority {
        COption::Some(authority) => {
            if !authority.eq(&self.synchronizer_key) {
                return Err(SynchronizerError::BadMintAuthority.into());
            }
        },
        COption::None => return Err(SynchronizerError::BadMintAuthority.into()),
    }

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

    SynchronizerData::pack(synchronizer, &mut synchronizer_authority_info.data.borrow_mut())?;

    Ok(())
}

pub fn process_sell_for(
    &self,
    accounts: &[AccountInfo],
    multiplier: u64,
    amount: u64,
    fee: u64,
    prices: &Vec<u64>,
    oracles: &Vec<Pubkey>
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
        return Err(SynchronizerError::InvalidSigner.into());
    }

    for oracle in oracles {
        if !self.oracles_keys.contains(oracle) {
            return Err(SynchronizerError::BadOracle.into());
        }
    }
    
    let mut synchronizer = SynchronizerData::unpack_unchecked(&synchronizer_authority_info.data.borrow())?;
    if !synchronizer.is_initialized {
        return Err(SynchronizerError::NotInitialized.into());
    }

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

    synchronizer.remaining_dollar_cap = synchronizer.remaining_dollar_cap + (collateral_amount * multiplier);
    synchronizer.withdrawable_fee_amount = synchronizer.withdrawable_fee_amount + fee_amount;

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

    SynchronizerData::pack(synchronizer, &mut synchronizer_authority_info.data.borrow_mut())?;

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
            ref prices,
            ref oracles
        } => {
            msg!("Instruction: BuyFor");
            self.process_buy_for(accounts, multiplier, amount, fee, prices, oracles)
        }
        SynchronizerInstruction::SellFor {
            multiplier,
            amount,
            fee,
            ref prices,
            ref oracles
        } => {
            msg!("Instruction: SellFor");
            self.process_sell_for(accounts, multiplier, amount, fee, prices, oracles)
        }

        // Admin Instructions
        SynchronizerInstruction::InitializeSynchronizerAccount {
            collateral_token_key,
            remaining_dollar_cap,
            withdrawable_fee_amount
        } => {
            msg!("Instruction: InitializeSynchronizerAccount");
            self.process_initialize_synchronizer_account(accounts, collateral_token_key, remaining_dollar_cap, withdrawable_fee_amount)
        }

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
            SynchronizerError::AlreadyInitialized => msg!("Error: Synchronizer account already initialized"),
            SynchronizerError::NotInitialized => msg!("Error: Synchronizer account is not initialized"),
            SynchronizerError::NotRentExempt => msg!("Error: Lamport balance below rent-exempt threshold"),
            SynchronizerError::AccessDenied => msg!("Error: Access Denied"),
            SynchronizerError::BadOracle => msg!("Error: signer is not an oracle"),
            SynchronizerError::BadMintAuthority => msg!("Error: Bad mint authority"),

            SynchronizerError::InvalidSigner => msg!("Error: Invalid transaction Signer"),
            SynchronizerError::InvalidInstruction => msg!("Error: Invalid instruction"),

            SynchronizerError::FailedMint => msg!("Error: Failed mint token"),
            SynchronizerError::FailedBurn => msg!("Error: Failed burn token"),
            SynchronizerError::FailedTransfer => msg!("Error: Failed transfer token"),
        }
    }
}

// Unit tests
#[cfg(test)]
mod test {
    use solana_program::{
        sysvar,
        account_info::IntoAccountInfo,
        program_error::ProgramError,
        program_pack::Pack
    };
    use solana_sdk::{
        account::{create_is_signer_account_infos,Account as SolanaAccount,create_account_for_test},
    };
    use spl_token::{
        instruction::{initialize_account, initialize_mint},
        state::{Account, Mint},
    };
    use super::*;

    fn mint_minimum_balance() -> u64 {
        Rent::default().minimum_balance(Mint::get_packed_len())
    }

    fn account_minimum_balance() -> u64 {
        Rent::default().minimum_balance(Account::get_packed_len())
    }

    fn init_acc_minimum_balance() -> u64 {
        Rent::default().minimum_balance(SynchronizerData::get_packed_len())
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
    fn test_init_synchronizer_account() {
        let program_id = id();
        let synchronizer_key = Pubkey::new_unique();
        let mut synchronizer_account = SolanaAccount::new(init_acc_minimum_balance(), SynchronizerData::get_packed_len(), &program_id);
        let rent_sysvar_key = sysvar::rent::id();
        let mut rent_sysvar_account = create_account_for_test(&Rent::default());

        let collateral_key = Pubkey::new_unique();

        let processor = Processor {
            synchronizer_key: synchronizer_key,
            oracles_keys: [
                Pubkey::new_unique(),
                Pubkey::new_unique(),
            ].iter().cloned().collect()
        };

        {
            let mut bad_sync_acc = SolanaAccount::new(init_acc_minimum_balance() - 100, SynchronizerData::get_packed_len(), &program_id);
            let accounts = vec![
                (&synchronizer_key, true, &mut bad_sync_acc).into_account_info(),
                (&rent_sysvar_key, false, &mut rent_sysvar_account).into_account_info(),
            ];
            assert_eq!(
                Err(SynchronizerError::NotRentExempt.into()),
                processor.process_initialize_synchronizer_account(&accounts, collateral_key, 0, 0)
            );
        }

        {
            let fake_program_id = Pubkey::new_unique();
            let mut bad_sync_acc = SolanaAccount::new(init_acc_minimum_balance(), SynchronizerData::get_packed_len(), &fake_program_id);
            let accounts = vec![
                (&synchronizer_key, true, &mut bad_sync_acc).into_account_info(),
                (&rent_sysvar_key, false, &mut rent_sysvar_account).into_account_info(),
            ];
            assert_eq!(
                Err(SynchronizerError::AccessDenied.into()), // cause of bad owner
                processor.process_initialize_synchronizer_account(&accounts, collateral_key, 0, 0)
            );
        }

        {
            let fake_sync_key = Pubkey::new_unique();
            let mut fake_sync_acc = SolanaAccount::new(init_acc_minimum_balance(), SynchronizerData::get_packed_len(), &program_id);
            let accounts = vec![
                (&fake_sync_key, true, &mut fake_sync_acc).into_account_info(),
                (&rent_sysvar_key, false, &mut rent_sysvar_account).into_account_info(),
            ];
            assert_eq!(
                Err(SynchronizerError::AccessDenied.into()), // cause of bad pubkey
                processor.process_initialize_synchronizer_account(&accounts, collateral_key, 0, 0)
            );
        }

        {
            let fake_sync_key = Pubkey::new_unique();
            let mut fake_sync_acc = SolanaAccount::new(init_acc_minimum_balance(), SynchronizerData::get_packed_len(), &fake_sync_key);
            let accounts = vec![
                (&fake_sync_key, true, &mut fake_sync_acc).into_account_info(),
                (&rent_sysvar_key, false, &mut rent_sysvar_account).into_account_info(),
            ];
            assert_eq!(
                Err(SynchronizerError::AccessDenied.into()),
                processor.process_initialize_synchronizer_account(&accounts, collateral_key, 0, 0)
            );
        }

        {
            let accounts = vec![
                (&synchronizer_key, true, &mut synchronizer_account).into_account_info(),
                (&rent_sysvar_key, false, &mut rent_sysvar_account).into_account_info(),
            ];
            processor.process_initialize_synchronizer_account(&accounts, collateral_key, 0, 0).unwrap()
        }

        {
            let accounts = vec![
                (&synchronizer_key, true, &mut synchronizer_account).into_account_info(),
                (&rent_sysvar_key, false, &mut rent_sysvar_account).into_account_info(),
            ];
            assert_eq!(
                Err(SynchronizerError::AlreadyInitialized.into()),
                processor.process_initialize_synchronizer_account(&accounts, collateral_key, 0, 0)
            );
        }
    }

    #[test]
    fn test_public_api() {
        let program_id = &id();
        let synchronizer_key = Pubkey::new_unique();
        let mut synchronizer_account = SolanaAccount::new(init_acc_minimum_balance(), SynchronizerData::get_packed_len(), &program_id);
        let rent_sysvar_key = sysvar::rent::id();
        let mut rent_sysvar = create_account_for_test(&Rent::default());
        let collateral_key = Pubkey::new_unique();
        let oracles = vec![Pubkey::new_unique(), Pubkey::new_unique()];

        let processor = Processor {
            synchronizer_key: synchronizer_key,
            oracles_keys: oracles.iter().cloned().collect()
        };

        {
            let accounts = vec![
                (&synchronizer_key, true, &mut synchronizer_account).into_account_info(),
                (&rent_sysvar_key, false, &mut rent_sysvar).into_account_info(),
            ];
            processor.process_initialize_synchronizer_account(&accounts, collateral_key, 0, 0).unwrap()
        }

        let user_key = Pubkey::new_unique();
        let mut user_account = SolanaAccount::default();

        // Create and init collateral token
        let mut collateral_asset_mint = SolanaAccount::new(mint_minimum_balance(), Mint::get_packed_len(), &spl_token::id());

        do_token_program(
            initialize_mint(&spl_token::id(), &collateral_key, &synchronizer_key, None, 6).unwrap(),
            vec![&mut collateral_asset_mint, &mut rent_sysvar],
        ).unwrap();

        // Create and init fiat asset token
        let fiat_asset_key = Pubkey::new_unique();
        let mut fiat_asset_mint = SolanaAccount::new(mint_minimum_balance(), Mint::get_packed_len(), &spl_token::id());
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
                processor.process_buy_for(&bad_accounts, 100, 100, 20, &vec![20, 30], &oracles)
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
                Err(SynchronizerError::InvalidSigner.into()),
                processor.process_buy_for(&bad_accounts, 100, 100, 20, &vec![20, 30], &oracles)
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
            processor.process_buy_for(&accounts, 100, 100, 20, &vec![20, 30], &oracles).unwrap();
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
                processor.process_sell_for(&bad_accounts, 100, 100, 20, &vec![20, 30], &oracles)
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
                Err(SynchronizerError::InvalidSigner.into()),
                processor.process_sell_for(&bad_accounts, 100, 100, 20, &vec![20, 30], &oracles)
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
            processor.process_sell_for(&accounts, 100, 100, 20, &vec![20, 30], &oracles).unwrap();
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
