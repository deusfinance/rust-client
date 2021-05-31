use crate::{
    error::SynchronizerError,
    instruction::{SynchronizerInstruction},
    state::SynchronizerData
};
use num_traits::FromPrimitive;
use solana_program::{account_info::{next_account_info, AccountInfo}, decode_error::DecodeError, entrypoint::ProgramResult, instruction::Instruction, msg, program_error::{PrintProgramError, ProgramError}, program_option::COption, program_pack::Pack, pubkey::Pubkey, rent::Rent, sysvar::Sysvar};
use spl_token::{instruction::{mint_to, burn, transfer}, processor::Processor as SPLTokenProcessor, state::{Account, Mint}};

// Synchronizer program_id
solana_program::declare_id!("8nNo8sjfYvwouTPQXw5fJ2D6DWzcWsbeXQanDGELt4AG");

/// Checks that the supplied program ID is the correct
pub fn check_program_account(spl_token_program_id: &Pubkey) -> ProgramResult {
    if spl_token_program_id != &id() {
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}

pub struct Processor {}
impl Processor {
// Scale
pub const DEFAULT_DECIMALS: u8 = 9;

// Instruction handlers

fn process_token_instruction(
    instruction: Instruction,
    instruction_account_infos: &[AccountInfo]
) -> ProgramResult {
    SPLTokenProcessor::process(&spl_token::id(), &instruction_account_infos, &instruction.data)
}

pub fn process_buy_for(
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
    if !synchronizer_authority_info.owner.eq(&id()) {
        return Err(SynchronizerError::AccessDenied.into());
    }
    if !synchronizer_authority_info.is_signer { // TODO: signer?
        return Err(SynchronizerError::InvalidSigner.into());
    }

    let mut synchronizer = SynchronizerData::unpack_unchecked(&synchronizer_authority_info.data.borrow())?;
    if !synchronizer.is_initialized {
        return Err(SynchronizerError::NotInitialized.into());
    }

    // TODO: turn oracles into accounts list
    if oracles.len() < synchronizer.minimum_required_signature as usize {
        return Err(SynchronizerError::NotEnoughOracles.into());
    }
    if prices.len() < synchronizer.minimum_required_signature as usize {
        return Err(SynchronizerError::NotEnoughOracles.into());
    }

    let mut price = prices[0];
    for i in 0..synchronizer.minimum_required_signature as usize {
        // if !Self::oracle_keys().contains(&oracles[i]) {
        //     return Err(SynchronizerError::BadOracle.into());
        // }

        if prices[i] > price {
            price = prices[i];
        }
    }

    let synchronizer_collateral_account = Account::unpack(&synchronizer_collateral_account_info.data.borrow()).unwrap();
    let user_collateral_account = Account::unpack(&user_collateral_account_info.data.borrow()).unwrap();
    if !synchronizer_collateral_account.mint.eq(&synchronizer.collateral_token_key) {
        return Err(SynchronizerError::BadCollateralMint.into());
    }
    if !user_collateral_account.mint.eq(&synchronizer.collateral_token_key) {
        return Err(SynchronizerError::BadCollateralMint.into());
    }

    let fiat_mint = Mint::unpack(&fiat_asset_mint_info.data.borrow_mut()).unwrap();
    let decimals= fiat_mint.decimals;
    if decimals != Self::DEFAULT_DECIMALS {
        return Err(SynchronizerError::BadDecimals.into());
    }

    match fiat_mint.mint_authority {
        COption::Some(authority) => {
            if !authority.eq(&synchronizer_authority_info.key) {
                return Err(SynchronizerError::BadMintAuthority.into());
            }
        },
        COption::None => return Err(SynchronizerError::BadMintAuthority.into()),
    }

    msg!("Process buy_for, user fiat amount: {}, collateral price: {}", amount, price);

    let collateral_amount_ui= spl_token::amount_to_ui_amount(amount, decimals) * spl_token::amount_to_ui_amount(price, decimals);
    let fee_amount_ui = collateral_amount_ui * spl_token::amount_to_ui_amount(fee, decimals);
    msg!("collateral_amount_ui: {}, fee_amount_ui: {}", collateral_amount_ui, fee_amount_ui);

    let collateral_amount = spl_token::ui_amount_to_amount(collateral_amount_ui, decimals);
    let fee_amount = spl_token::ui_amount_to_amount(fee_amount_ui, decimals);
    msg!("collateral_amount: {}, fee_amount: {}", collateral_amount, fee_amount);

    if user_collateral_account.amount < (collateral_amount + fee_amount) {
        return Err(SynchronizerError::InsufficientFunds.into());
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
    Self::process_token_instruction(instruction, &account_infos).unwrap();
    msg!("Transfer {} collateral tokens from user to synchronizer", collateral_amount + fee_amount);

    // Synchronizer mint fiat asset to user associated token account
    let instruction = mint_to(
        &spl_token::id(),
        &fiat_asset_mint_info.key,
        &user_fiat_account_info.key,
        &synchronizer_authority_info.key,
        &[],
        amount
    ).unwrap();
    let account_infos = [
        fiat_asset_mint_info.clone(),
        user_fiat_account_info.clone(),
        synchronizer_authority_info.clone(),
    ];
    Self::process_token_instruction(instruction, &account_infos).unwrap();
    msg!("Mint {} fiat tokens to user_account", {amount});

    synchronizer.remaining_dollar_cap -= spl_token::ui_amount_to_amount(collateral_amount_ui * multiplier as f64, decimals);
    synchronizer.withdrawable_fee_amount += fee_amount;
    SynchronizerData::pack(synchronizer, &mut synchronizer_authority_info.data.borrow_mut())?;

    Ok(())
}

pub fn process_sell_for(
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
    if !synchronizer_authority_info.owner.eq(&id()) {
        return Err(SynchronizerError::AccessDenied.into());
    }
    if !synchronizer_authority_info.is_signer { // TODO: signer?
        return Err(SynchronizerError::InvalidSigner.into());
    }

    let mut synchronizer = SynchronizerData::unpack_unchecked(&synchronizer_authority_info.data.borrow())?;
    if !synchronizer.is_initialized {
        return Err(SynchronizerError::NotInitialized.into());
    }

    // TODO: turn oracles in accounts list
    if oracles.len() < synchronizer.minimum_required_signature as usize {
        return Err(SynchronizerError::NotEnoughOracles.into());
    }
    if prices.len() < synchronizer.minimum_required_signature as usize {
        return Err(SynchronizerError::NotEnoughOracles.into());
    }

    let mut price = prices[0];
    for i in 0..synchronizer.minimum_required_signature as usize {
        // if !Self::oracle_keys().contains(&oracles[i]) {
        //     return Err(SynchronizerError::BadOracle.into());
        // }

        if prices[i] < price {
            price = prices[i];
        }
    }

    let synchronizer_collateral_account = Account::unpack(&synchronizer_collateral_account_info.data.borrow()).unwrap();
    let user_collateral_account = Account::unpack(&user_collateral_account_info.data.borrow()).unwrap();
    if !synchronizer_collateral_account.mint.eq(&synchronizer.collateral_token_key) {
        return Err(SynchronizerError::BadCollateralMint.into());
    }
    if !user_collateral_account.mint.eq(&synchronizer.collateral_token_key) {
        return Err(SynchronizerError::BadCollateralMint.into());
    }

    let decimals= Mint::unpack(&fiat_asset_mint_info.data.borrow_mut()).unwrap().decimals;
    if decimals != Self::DEFAULT_DECIMALS {
        return Err(SynchronizerError::BadDecimals.into());
    }

    msg!("Process sell_for, user fiat amount: {}, collateral price: {}", amount, price);

    let collateral_amount_ui=spl_token::amount_to_ui_amount(amount, decimals) * spl_token::amount_to_ui_amount(price, decimals);
    let fee_amount_ui = collateral_amount_ui * spl_token::amount_to_ui_amount(fee, decimals);
    msg!("collateral_amount_ui: {}, fee_amount_ui: {}", collateral_amount_ui, fee_amount_ui);

    let collateral_amount = spl_token::ui_amount_to_amount(collateral_amount_ui, decimals);
    let fee_amount = spl_token::ui_amount_to_amount(fee_amount_ui, decimals);
    msg!("collateral_amount: {}, fee_amount: {}", collateral_amount, fee_amount);

    if synchronizer_collateral_account.amount < (collateral_amount - fee_amount) {
        return Err(SynchronizerError::InsufficientFunds.into());
    }

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
    Self::process_token_instruction(instruction, &account_infos).unwrap();
    msg!("Burn {} fiat assets from user_account", amount);

    // Transfer collateral token from synchronizer to user
    let instruction = transfer(
        &spl_token::id(),
        &synchronizer_collateral_account_info.key,
        &user_collateral_account_info.key,
        &synchronizer_authority_info.key,
        &[],
        collateral_amount - fee_amount
    ).unwrap();
    let account_infos = [
        synchronizer_collateral_account_info.clone(),
        user_collateral_account_info.clone(),
        synchronizer_authority_info.clone(),
    ];
    Self::process_token_instruction(instruction, &account_infos).unwrap();
    msg!("Transfer {} collateral asset from synchronizer to user", collateral_amount - fee_amount);

    synchronizer.remaining_dollar_cap += spl_token::ui_amount_to_amount(collateral_amount_ui * multiplier as f64, decimals);
    synchronizer.withdrawable_fee_amount += fee_amount;
    SynchronizerData::pack(synchronizer, &mut synchronizer_authority_info.data.borrow_mut())?;

    Ok(())
}

pub fn process_initialize_synchronizer_account(
    accounts: &[AccountInfo],
    collateral_token_key: Pubkey,
    remaining_dollar_cap: u64,
    withdrawable_fee_amount: u64,
    minimum_required_signature: u64
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let synchronizer_account_info = next_account_info(account_info_iter)?;
    let rent_account_info = next_account_info(account_info_iter)?;

    if !synchronizer_account_info.owner.eq(&id()) {
        return Err(SynchronizerError::AccessDenied.into());
    }

    if !synchronizer_account_info.is_signer {
        return Err(SynchronizerError::InvalidSigner.into());
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
    synchronizer.collateral_token_key = collateral_token_key;
    synchronizer.remaining_dollar_cap = remaining_dollar_cap;
    synchronizer.withdrawable_fee_amount = withdrawable_fee_amount;
    synchronizer.minimum_required_signature = minimum_required_signature;

    SynchronizerData::pack(synchronizer, &mut synchronizer_account_info.data.borrow_mut())?;

    Ok(())
}

pub fn process_set_minimum_required_signature(
    accounts: &[AccountInfo],
    minimum_required_signature: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let synchronizer_account_info = next_account_info(account_info_iter)?;

    if !synchronizer_account_info.owner.eq(&id()) {
        return Err(SynchronizerError::AccessDenied.into());
    }

    if !synchronizer_account_info.is_signer {
        return Err(SynchronizerError::InvalidSigner.into());
    }

    let mut synchronizer = SynchronizerData::unpack_unchecked(&synchronizer_account_info.data.borrow())?;
    if !synchronizer.is_initialized {
        return Err(SynchronizerError::NotInitialized.into());
    }

    synchronizer.minimum_required_signature = minimum_required_signature;
    SynchronizerData::pack(synchronizer, &mut synchronizer_account_info.data.borrow_mut())?;

    Ok(())
}

pub fn process_set_collateral_token(
    accounts: &[AccountInfo],
    collateral_token_key: Pubkey,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let synchronizer_account_info = next_account_info(account_info_iter)?;

    if !synchronizer_account_info.owner.eq(&id()) {
        return Err(SynchronizerError::AccessDenied.into());
    }

    if !synchronizer_account_info.is_signer {
        return Err(SynchronizerError::InvalidSigner.into());
    }

    let mut synchronizer = SynchronizerData::unpack_unchecked(&synchronizer_account_info.data.borrow())?;
    if !synchronizer.is_initialized {
        return Err(SynchronizerError::NotInitialized.into());
    }

    synchronizer.collateral_token_key = collateral_token_key;
    SynchronizerData::pack(synchronizer, &mut synchronizer_account_info.data.borrow_mut())?;

    Ok(())
}

pub fn process_set_remaining_dollar_cap(
    accounts: &[AccountInfo],
    remaining_dollar_cap: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let synchronizer_account_info = next_account_info(account_info_iter)?;

    if !synchronizer_account_info.owner.eq(&id()) {
        return Err(SynchronizerError::AccessDenied.into());
    }

    if !synchronizer_account_info.is_signer {
        return Err(SynchronizerError::InvalidSigner.into());
    }

    let mut synchronizer = SynchronizerData::unpack_unchecked(&synchronizer_account_info.data.borrow())?;
    if !synchronizer.is_initialized {
        return Err(SynchronizerError::NotInitialized.into());
    }

    synchronizer.remaining_dollar_cap = remaining_dollar_cap;
    SynchronizerData::pack(synchronizer, &mut synchronizer_account_info.data.borrow_mut())?;

    Ok(())
}

pub fn process_withdraw_fee(
    accounts: &[AccountInfo],
    amount: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let synchronizer_collateral_account_info = next_account_info(account_info_iter)?;
    let recipient_collateral_account_info = next_account_info(account_info_iter)?;
    let synchronizer_account_info = next_account_info(account_info_iter)?;

    if !synchronizer_account_info.owner.eq(&id()) {
        return Err(SynchronizerError::AccessDenied.into());
    }

    if !synchronizer_account_info.is_signer {
        return Err(SynchronizerError::InvalidSigner.into());
    }

    let mut synchronizer = SynchronizerData::unpack_unchecked(&synchronizer_account_info.data.borrow())?;
    if !synchronizer.is_initialized {
        return Err(SynchronizerError::NotInitialized.into());
    }

    if synchronizer.withdrawable_fee_amount < amount {
        return Err(SynchronizerError::InsufficientFunds.into());
    }

    let instruction = transfer(
        &spl_token::id(),
        &synchronizer_collateral_account_info.key,
        &recipient_collateral_account_info.key,
        &synchronizer_account_info.key,
        &[],
        amount
    ).unwrap();
    let account_infos = [
        synchronizer_collateral_account_info.clone(),
        recipient_collateral_account_info.clone(),
        synchronizer_account_info.clone(),
    ];
    Self::process_token_instruction(instruction, &account_infos).unwrap();
    msg!("Transfer {} collateral asset from synchronizer to recipient {}", amount, recipient_collateral_account_info.key);

    synchronizer.withdrawable_fee_amount -= amount;
    SynchronizerData::pack(synchronizer, &mut synchronizer_account_info.data.borrow_mut())?;

    Ok(())
}

pub fn process_withdraw_collateral(
    accounts: &[AccountInfo],
    amount: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let synchronizer_collateral_account_info = next_account_info(account_info_iter)?;
    let recipient_collateral_account_info = next_account_info(account_info_iter)?;
    let synchronizer_account_info = next_account_info(account_info_iter)?;

    if !synchronizer_account_info.owner.eq(&id()) {
        return Err(SynchronizerError::AccessDenied.into());
    }

    if !synchronizer_account_info.is_signer {
        return Err(SynchronizerError::InvalidSigner.into());
    }

    let synchronizer = SynchronizerData::unpack_unchecked(&synchronizer_account_info.data.borrow())?;
    if !synchronizer.is_initialized {
        return Err(SynchronizerError::NotInitialized.into());
    }

    let instruction = transfer(
        &spl_token::id(),
        &synchronizer_collateral_account_info.key,
        &recipient_collateral_account_info.key,
        &synchronizer_account_info.key,
        &[],
        amount
    ).unwrap();
    let account_infos = [
        synchronizer_collateral_account_info.clone(),
        recipient_collateral_account_info.clone(),
        synchronizer_account_info.clone(),
    ];
    Self::process_token_instruction(instruction, &account_infos).unwrap();
    msg!("Transfer {} collateral asset from synchronizer to recipient {}", amount, recipient_collateral_account_info.key);

    Ok(())
}

pub fn process_instruction(
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
            Self::process_buy_for(accounts, multiplier, amount, fee, prices, oracles)
        }
        SynchronizerInstruction::SellFor {
            multiplier,
            amount,
            fee,
            ref prices,
            ref oracles
        } => {
            msg!("Instruction: SellFor");
            Self::process_sell_for(accounts, multiplier, amount, fee, prices, oracles)
        }

        // Admin Instructions
        SynchronizerInstruction::InitializeSynchronizerAccount {
            collateral_token_key,
            remaining_dollar_cap,
            withdrawable_fee_amount,
            minimum_required_signature
        } => {
            msg!("Instruction: InitializeSynchronizerAccount");
            Self::process_initialize_synchronizer_account(accounts, collateral_token_key, remaining_dollar_cap, withdrawable_fee_amount, minimum_required_signature)
        }

        SynchronizerInstruction::SetMinimumRequiredSignature {
            minimum_required_signature
        } => {
            msg!("Instruction: SetMinimumRequiredSignature");
            Self::process_set_minimum_required_signature(accounts, minimum_required_signature)
        }

        SynchronizerInstruction::SetCollateralToken {
            collateral_token_key
        } => {
            msg!("Instruction: SetCollateralToken");
            Self::process_set_collateral_token(accounts, collateral_token_key)
        }

        SynchronizerInstruction::SetRemainingDollarCap {
            remaining_dollar_cap
        } => {
            msg!("Instruction: SetRemainingDollarCap");
            Self::process_set_remaining_dollar_cap(accounts, remaining_dollar_cap)
        }

        SynchronizerInstruction::WithdrawFee {
            amount
        } => {
            msg!("Instruction: WithdrawFee");
            Self::process_withdraw_fee(accounts, amount)
        }

        SynchronizerInstruction::WithdrawCollateral {
            amount
        } => {
            msg!("Instruction: WithdrawCollateral");
            Self::process_withdraw_collateral(accounts, amount)
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
            SynchronizerError::InsufficientFunds => msg!("Error: Insufficient funds"),
            SynchronizerError::AccessDenied => msg!("Error: Access Denied"),

            SynchronizerError::NotEnoughOracles => msg!("Error: Not enough oracles"),
            SynchronizerError::BadOracle => msg!("Error: signer is not an oracle"),
            SynchronizerError::BadMintAuthority => msg!("Error: Bad mint authority"),
            SynchronizerError::BadCollateralMint => msg!("Error: Bad collateral mint"),
            SynchronizerError::BadDecimals => msg!("Error: Bad mint decimals"),

            SynchronizerError::InvalidSigner => msg!("Error: Invalid transaction Signer"),
            SynchronizerError::InvalidInstruction => msg!("Error: Invalid instruction"),
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
    use spl_token::{instruction::{initialize_account, initialize_mint}, state::{Account, Mint}};
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

        {
            let mut bad_sync_acc = SolanaAccount::new(init_acc_minimum_balance() - 100, SynchronizerData::get_packed_len(), &program_id);
            let accounts = vec![
                (&synchronizer_key, true, &mut bad_sync_acc).into_account_info(),
                (&rent_sysvar_key, false, &mut rent_sysvar_account).into_account_info(),
            ];
            assert_eq!(
                Err(SynchronizerError::NotRentExempt.into()),
                Processor::process_initialize_synchronizer_account(&accounts, collateral_key, 0, 0, 2)
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
                Processor::process_initialize_synchronizer_account(&accounts, collateral_key, 0, 0, 2)
            );
        }

        // { TODO: fix or delete?
        //     let fake_sync_key = Pubkey::new_unique();
        //     let mut fake_sync_acc = SolanaAccount::new(init_acc_minimum_balance(), SynchronizerData::get_packed_len(), &program_id);
        //     let accounts = vec![
        //         (&fake_sync_key, true, &mut fake_sync_acc).into_account_info(),
        //         (&rent_sysvar_key, false, &mut rent_sysvar_account).into_account_info(),
        //     ];
        //     assert_eq!(
        //         Err(SynchronizerError::AccessDenied.into()), // cause of bad pubkey
        //         Processor::process_initialize_synchronizer_account(&accounts, collateral_key, 0, 0, 2)
        //     );
        // }

        {
            let fake_sync_key = Pubkey::new_unique();
            let mut fake_sync_acc = SolanaAccount::new(init_acc_minimum_balance(), SynchronizerData::get_packed_len(), &fake_sync_key);
            let accounts = vec![
                (&fake_sync_key, true, &mut fake_sync_acc).into_account_info(),
                (&rent_sysvar_key, false, &mut rent_sysvar_account).into_account_info(),
            ];
            assert_eq!(
                Err(SynchronizerError::AccessDenied.into()), // bad program_id
                Processor::process_initialize_synchronizer_account(&accounts, collateral_key, 0, 0, 2)
            );
        }

        {
            let accounts = vec![
                (&synchronizer_key, true, &mut synchronizer_account).into_account_info(),
                (&rent_sysvar_key, false, &mut rent_sysvar_account).into_account_info(),
            ];
            Processor::process_initialize_synchronizer_account(&accounts, collateral_key, 0, 0, 2).unwrap()
        }

        {
            let accounts = vec![
                (&synchronizer_key, true, &mut synchronizer_account).into_account_info(),
                (&rent_sysvar_key, false, &mut rent_sysvar_account).into_account_info(),
            ];
            assert_eq!(
                Err(SynchronizerError::AlreadyInitialized.into()),
                Processor::process_initialize_synchronizer_account(&accounts, collateral_key, 0, 0, 2)
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
        let user_key = Pubkey::new_unique();
        let mut user_account = SolanaAccount::default();

        // Infrastructure preparing
        // Create and init collateral token
        let decimals = Processor::DEFAULT_DECIMALS;
        let mut collateral_asset_mint = SolanaAccount::new(mint_minimum_balance(), Mint::get_packed_len(), &spl_token::id());
        do_token_program(
            initialize_mint(&spl_token::id(), &collateral_key, &synchronizer_key, None, decimals).unwrap(),
            vec![&mut collateral_asset_mint, &mut rent_sysvar],
        ).unwrap();

        // Create and init fiat asset token
        let fiat_asset_key = Pubkey::new_unique();
        let mut fiat_asset_mint = SolanaAccount::new(mint_minimum_balance(), Mint::get_packed_len(), &spl_token::id());
        do_token_program(
            initialize_mint(&spl_token::id(), &fiat_asset_key, &synchronizer_key, None, decimals).unwrap(),
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

        // Mint some collateral asset to synchronizer account
        let amount = spl_token::ui_amount_to_amount(500.0, decimals);
        do_token_program(
            mint_to(&spl_token::id(), &collateral_key, &synchronizer_collateral_key, &synchronizer_key, &[],amount).unwrap(),
            vec![&mut collateral_asset_mint, &mut synchronizer_collateral_account, &mut synchronizer_account],
        ).unwrap();

        // Mint some fiat asset to user account
        let amount = spl_token::ui_amount_to_amount(500.0, decimals);
        do_token_program(
            mint_to(&spl_token::id(), &fiat_asset_key, &user_fiat_key, &synchronizer_key, &[],amount).unwrap(),
            vec![&mut fiat_asset_mint, &mut user_fiat_account, &mut synchronizer_account],
        ).unwrap();

        // Prepare account_infos
        let synchronizer_account_info = (&synchronizer_key, true, &mut synchronizer_account).into_account_info();
        let user_account_info = (&user_key, true, &mut user_account).into_account_info();
        let fiat_asset_mint_info = (&fiat_asset_key, &mut fiat_asset_mint).into_account_info();
        let user_collateral_account_info = (&user_collateral_key, &mut user_collateral_account).into_account_info();
        let user_fiat_account_info = (&user_fiat_key, &mut user_fiat_account).into_account_info();
        let synchronizer_collateral_account_info = (&synchronizer_collateral_key, &mut synchronizer_collateral_account).into_account_info();
        let rent_sysvar_info = (&rent_sysvar_key, false, &mut rent_sysvar).into_account_info();

        // Initialize Syncronizer account
        let accounts = vec![
            synchronizer_account_info.clone(),
            rent_sysvar_info.clone(),
        ];
        Processor::process_initialize_synchronizer_account(&accounts, collateral_key, 0, 0, oracles.len() as u64).unwrap();

        // Parameters for sell/buy instructions
        let mul_stocks = 2;
        let fee = spl_token::ui_amount_to_amount(0.001, decimals);
        let prices = vec![
            spl_token::ui_amount_to_amount(0.5, decimals),
            spl_token::ui_amount_to_amount(0.4, decimals)
        ];

        // Test sell_for instruction
        let sell_fiat_amount = spl_token::ui_amount_to_amount(100.0, decimals);

        // BadCase: bad synchronizer signer
        let fake_synchronizer_key = Pubkey::new_unique();
        let mut fake_synchronizer_account = SolanaAccount::default();
        let bad_accounts = vec![
            fiat_asset_mint_info.clone(),
            user_collateral_account_info.clone(),
            user_fiat_account_info.clone(),
            synchronizer_collateral_account_info.clone(),
            user_account_info.clone(),
            (&fake_synchronizer_key, true, &mut fake_synchronizer_account).into_account_info(), // wrong sync acc
        ];
        assert_eq!(
            Err(SynchronizerError::AccessDenied.into()),
            Processor::process_sell_for(&bad_accounts, mul_stocks, sell_fiat_amount, fee, &prices, &oracles)
        );

        assert_eq!(
            user_key,
            Account::unpack_unchecked(&user_fiat_account_info.data.borrow()).unwrap().owner
        );

        let synchronizer = SynchronizerData::unpack(&synchronizer_account_info.data.borrow()).unwrap();
        let dollar_cap_before = synchronizer.remaining_dollar_cap;
        let withdrawable_fee_before = synchronizer.withdrawable_fee_amount;

        assert_eq!(dollar_cap_before, 0);
        assert_eq!(withdrawable_fee_before, 0);

        let sync_collateral_balance_before = Account::unpack_unchecked(&synchronizer_collateral_account_info.data.borrow()).unwrap().amount;
        let user_collateral_balance_before = Account::unpack_unchecked(&user_collateral_account_info.data.borrow()).unwrap().amount;
        let user_fiat_balance_before = Account::unpack_unchecked(&user_fiat_account_info.data.borrow()).unwrap().amount;
        let accounts = vec![
            fiat_asset_mint_info.clone(),
            user_collateral_account_info.clone(),
            user_fiat_account_info.clone(),
            synchronizer_collateral_account_info.clone(),
            user_account_info.clone(),
            synchronizer_account_info.clone(),
        ];
        Processor::process_sell_for(&accounts, mul_stocks, sell_fiat_amount, fee, &prices, &oracles).unwrap();

        // Check balances afet sell_for
        assert_eq!(
            Account::unpack_unchecked(&user_fiat_account_info.data.borrow()).unwrap().amount,
            user_fiat_balance_before - sell_fiat_amount
        );

        let collateral_amount: u64 = 40_000_000_000; // amount * price
        let collateral_fee: u64 = 40_000_000; // collateral_amount * fee
        assert_eq!(
            Account::unpack_unchecked(&synchronizer_collateral_account_info.data.borrow()).unwrap().amount,
            sync_collateral_balance_before - (collateral_amount - collateral_fee)
        );
        assert_eq!(
            Account::unpack_unchecked(&user_collateral_account_info.data.borrow()).unwrap().amount,
            user_collateral_balance_before + (collateral_amount - collateral_fee)
        );

        let synchronizer = SynchronizerData::unpack(&synchronizer_account_info.data.borrow()).unwrap();
        assert_eq!(synchronizer.remaining_dollar_cap, dollar_cap_before + collateral_amount * mul_stocks);
        assert_eq!(synchronizer.withdrawable_fee_amount, withdrawable_fee_before +  collateral_fee);

        // Test buy_for instruction
        let buy_fiat_amount = spl_token::ui_amount_to_amount(50.0, decimals);

        // Case: bad synchronizer signer
        let fake_synchronizer_key = Pubkey::new_unique();
        let mut fake_synchronizer_account = SolanaAccount::default();
        let bad_accounts = vec![
            fiat_asset_mint_info.clone(),
            user_collateral_account_info.clone(),
            user_fiat_account_info.clone(),
            synchronizer_collateral_account_info.clone(),
            user_account_info.clone(),
            (&fake_synchronizer_key, true, &mut fake_synchronizer_account).into_account_info(),
        ];
        assert_eq!(
            Err(SynchronizerError::AccessDenied.into()),
            Processor::process_buy_for(&bad_accounts, mul_stocks, buy_fiat_amount, fee, &prices, &oracles)
        );

        // Good case
        let synchronizer = SynchronizerData::unpack(&synchronizer_account_info.data.borrow()).unwrap();
        let dollar_cap_before = synchronizer.remaining_dollar_cap;
        let withdrawable_fee_before = synchronizer.withdrawable_fee_amount;

        let sync_collateral_balance_before = Account::unpack_unchecked(&synchronizer_collateral_account_info.data.borrow()).unwrap().amount;
        let user_collateral_balance_before = Account::unpack_unchecked(&user_collateral_account_info.data.borrow()).unwrap().amount;
        let user_fiat_balance_before = Account::unpack_unchecked(&user_fiat_account_info.data.borrow()).unwrap().amount;
        let accounts = vec![
            fiat_asset_mint_info.clone(),
            user_collateral_account_info.clone(),
            user_fiat_account_info.clone(),
            synchronizer_collateral_account_info.clone(),
            user_account_info.clone(),
            synchronizer_account_info.clone(),
        ];
        Processor::process_buy_for(&accounts, mul_stocks, buy_fiat_amount, fee, &prices, &oracles).unwrap();

        // Check balances afet buy_for
        assert_eq!(
            Account::unpack_unchecked(&user_fiat_account_info.data.borrow()).unwrap().amount,
            user_fiat_balance_before + buy_fiat_amount
        );

        let collateral_amount: u64 = 25_000_000_000; // amount * price
        let collateral_fee: u64 = 25_000_000; // collateral_amount * fee
        assert_eq!(
            Account::unpack_unchecked(&synchronizer_collateral_account_info.data.borrow()).unwrap().amount,
            sync_collateral_balance_before + (collateral_amount + collateral_fee)
        );
        assert_eq!(
            Account::unpack_unchecked(&user_collateral_account_info.data.borrow()).unwrap().amount,
            user_collateral_balance_before - (collateral_amount + collateral_fee)
        );

        let synchronizer = SynchronizerData::unpack(&synchronizer_account_info.data.borrow()).unwrap();
        assert_eq!(synchronizer.remaining_dollar_cap, dollar_cap_before - (collateral_amount * mul_stocks));
        assert_eq!(synchronizer.withdrawable_fee_amount, withdrawable_fee_before + collateral_fee);

        // BadCase: too big buy amount
        let buy_fiat_amount = spl_token::ui_amount_to_amount(999999.0, decimals);
        assert_eq!(
            Err(SynchronizerError::InsufficientFunds.into()),
            Processor::process_buy_for(&accounts, mul_stocks, buy_fiat_amount, fee, &prices, &oracles)
        );

        // BadCase: too big sell amount
        let sell_fiat_amount = spl_token::ui_amount_to_amount(999999.0, decimals);
        assert_eq!(
            Err(SynchronizerError::InsufficientFunds.into()),
            Processor::process_sell_for(&accounts, mul_stocks, sell_fiat_amount, fee, &prices, &oracles)
        );
    }

    #[test]
    fn test_withdraw() {
        let program_id = id();
        let synchronizer_key = Pubkey::new_unique();
        let mut synchronizer_account = SolanaAccount::new(init_acc_minimum_balance(), SynchronizerData::get_packed_len(), &program_id);
        let recipient_key = Pubkey::new_unique();
        let mut recipient_account = SolanaAccount::default();
        let rent_sysvar_key = sysvar::rent::id();
        let mut rent_sysvar = create_account_for_test(&Rent::default());
        let collateral_token_key = Pubkey::new_unique();
        let oracles = vec![Pubkey::new_unique(), Pubkey::new_unique()];

        // Infrastructure preparing
        // Create and init collateral token
        let decimals = Processor::DEFAULT_DECIMALS;
        let mut collateral_asset_mint = SolanaAccount::new(mint_minimum_balance(), Mint::get_packed_len(), &spl_token::id());
        do_token_program(
            initialize_mint(&spl_token::id(), &collateral_token_key, &synchronizer_key, None, decimals).unwrap(),
            vec![&mut collateral_asset_mint, &mut rent_sysvar],
        ).unwrap();

        // Create and init token associated accounts for synchronizer
        let synchronizer_collateral_key = Pubkey::new_unique();
        let mut synchronizer_collateral_account = SolanaAccount::new(account_minimum_balance(), Account::get_packed_len(), &spl_token::id());
        do_token_program(
            initialize_account(&spl_token::id(), &synchronizer_collateral_key, &collateral_token_key, &synchronizer_key).unwrap(),
            vec![
                &mut synchronizer_collateral_account,
                &mut collateral_asset_mint,
                &mut synchronizer_account,
                &mut rent_sysvar,
            ],
        ).unwrap();

        // Create token associated accounts for recipient
        let recipient_collateral_key = Pubkey::new_unique();
        let mut recipient_collateral_account = SolanaAccount::new(account_minimum_balance(), Account::get_packed_len(), &spl_token::id());
        do_token_program(
            initialize_account(&spl_token::id(), &recipient_collateral_key, &collateral_token_key, &recipient_key).unwrap(),
            vec![
                &mut recipient_collateral_account,
                &mut collateral_asset_mint,
                &mut recipient_account,
                &mut rent_sysvar,
            ],
        ).unwrap();

        // Mint some collateral asset to synchronizer account
        let amount = spl_token::ui_amount_to_amount(500.0, decimals);
        do_token_program(
            mint_to(&spl_token::id(), &collateral_token_key, &synchronizer_collateral_key, &synchronizer_key, &[], amount).unwrap(),
            vec![&mut collateral_asset_mint, &mut synchronizer_collateral_account, &mut synchronizer_account],
        ).unwrap();

        // Mint some collateral asset to recipient account
        let amount = spl_token::ui_amount_to_amount(100.0, decimals);
        do_token_program(
            mint_to(&spl_token::id(), &collateral_token_key, &recipient_collateral_key, &synchronizer_key, &[], amount).unwrap(),
            vec![&mut collateral_asset_mint, &mut recipient_collateral_account, &mut synchronizer_account],
        ).unwrap();

        // Initialize synchronizer account
        let synchronizer_account_info = (&synchronizer_key, true, &mut synchronizer_account).into_account_info();
        let synchronizer_collateral_info = (&synchronizer_collateral_key, false, &mut synchronizer_collateral_account).into_account_info();
        let recipient_collateral_info = (&recipient_collateral_key, false, &mut recipient_collateral_account).into_account_info();
        let rent_info = (&rent_sysvar_key, false, &mut rent_sysvar).into_account_info();

        let accounts = vec![
            synchronizer_account_info.clone(),
            rent_info.clone(),
        ];
        Processor::process_initialize_synchronizer_account(&accounts, collateral_token_key, spl_token::ui_amount_to_amount(500.0, decimals), spl_token::ui_amount_to_amount(250.0, decimals), oracles.len() as u64).unwrap();

        let accounts = vec![
            synchronizer_collateral_info.clone(),
            recipient_collateral_info.clone(),
            synchronizer_account_info.clone(),
        ];

        let amount = spl_token::ui_amount_to_amount(300.0, decimals);
        assert_eq!(
            Err(SynchronizerError::InsufficientFunds.into()),
            Processor::process_withdraw_fee(&accounts, amount)
        );

        assert_eq!(
            Account::unpack_unchecked(&synchronizer_collateral_info.data.borrow()).unwrap().amount,
            500000000000
        );
        assert_eq!(
            Account::unpack_unchecked(&recipient_collateral_info.data.borrow()).unwrap().amount,
            100000000000
        );
        assert_eq!(
            SynchronizerData::unpack_unchecked(&synchronizer_account_info.data.borrow()).unwrap().withdrawable_fee_amount,
            250000000000
        );

        let amount = spl_token::ui_amount_to_amount(50.0, decimals);
        Processor::process_withdraw_fee(&accounts, amount).unwrap();

        assert_eq!(
            Account::unpack_unchecked(&synchronizer_collateral_info.data.borrow()).unwrap().amount,
            450000000000
        );
        assert_eq!(
            Account::unpack_unchecked(&recipient_collateral_info.data.borrow()).unwrap().amount,
            150000000000
        );
        assert_eq!(
            SynchronizerData::unpack_unchecked(&synchronizer_account_info.data.borrow()).unwrap().withdrawable_fee_amount,
            200000000000
        );

        let amount = spl_token::ui_amount_to_amount(50.0, decimals);
        Processor::process_withdraw_collateral(&accounts, amount).unwrap();

        assert_eq!(
            Account::unpack_unchecked(&synchronizer_collateral_info.data.borrow()).unwrap().amount,
            400000000000
        );
        assert_eq!(
            Account::unpack_unchecked(&recipient_collateral_info.data.borrow()).unwrap().amount,
            200000000000
        );
        assert_eq!(
            SynchronizerData::unpack_unchecked(&synchronizer_account_info.data.borrow()).unwrap().withdrawable_fee_amount,
            200000000000
        );
    }

    #[test]
    fn test_admin_setters() {
        let program_id = id();
        let synchronizer_key = Pubkey::new_unique();
        let mut synchronizer_account = SolanaAccount::new(init_acc_minimum_balance(), SynchronizerData::get_packed_len(), &program_id);
        let rent_sysvar_key = sysvar::rent::id();
        let mut rent_sysvar_account = create_account_for_test(&Rent::default());
        let start_collateral_token_key = Pubkey::new_unique();
        let oracles = vec![Pubkey::new_unique(), Pubkey::new_unique()];

        let synchronizer_account_info = (&synchronizer_key, true, &mut synchronizer_account).into_account_info();
        let rent_info = (&rent_sysvar_key, false, &mut rent_sysvar_account).into_account_info();

        let start_remaining_dollar_cap: u64 = 10;
        let start_minimum_required_signature: u64 = oracles.len() as u64;

        let mut fake_acc = SolanaAccount::default();
        let bad_accounts = vec![
            (&synchronizer_key, true, &mut fake_acc).into_account_info(),
            rent_info.clone(),
        ];
        assert_eq!(Err(SynchronizerError::AccessDenied.into()), Processor::process_set_minimum_required_signature(&bad_accounts, 123456));
        assert_eq!(Err(SynchronizerError::AccessDenied.into()), Processor::process_set_remaining_dollar_cap(&bad_accounts, 123456));
        assert_eq!(Err(SynchronizerError::AccessDenied.into()), Processor::process_set_collateral_token(&bad_accounts, Pubkey::new_unique()));

        let accounts = vec![
            synchronizer_account_info.clone(),
            rent_info.clone(),
        ];

        assert_eq!(Err(SynchronizerError::NotInitialized.into()), Processor::process_set_minimum_required_signature(&accounts, 123456));
        assert_eq!(Err(SynchronizerError::NotInitialized.into()), Processor::process_set_remaining_dollar_cap(&accounts, 123456));
        assert_eq!(Err(SynchronizerError::NotInitialized.into()), Processor::process_set_collateral_token(&accounts, Pubkey::new_unique()));

        Processor::process_initialize_synchronizer_account(&accounts, start_collateral_token_key, start_remaining_dollar_cap, 0, start_minimum_required_signature).unwrap();

        let sync_data = SynchronizerData::unpack(&synchronizer_account_info.data.borrow()).unwrap();
        assert_eq!(sync_data.minimum_required_signature, start_minimum_required_signature);
        assert_eq!(sync_data.remaining_dollar_cap, start_remaining_dollar_cap);
        assert_eq!(sync_data.collateral_token_key, start_collateral_token_key);

        let minimum_required_signature = 3;
        Processor::process_set_minimum_required_signature(&accounts, minimum_required_signature).unwrap();
        let sync_data = SynchronizerData::unpack(&synchronizer_account_info.data.borrow()).unwrap();
        assert_eq!(sync_data.minimum_required_signature, minimum_required_signature);

        let remaining_dollar_cap: u64 = 123456;
        Processor::process_set_remaining_dollar_cap(&accounts, remaining_dollar_cap).unwrap();
        let sync_data = SynchronizerData::unpack(&synchronizer_account_info.data.borrow()).unwrap();
        assert_eq!(sync_data.remaining_dollar_cap, remaining_dollar_cap);

        let collateral_token_key = Pubkey::new_unique();
        Processor::process_set_collateral_token(&accounts, collateral_token_key).unwrap();
        let sync_data = SynchronizerData::unpack(&synchronizer_account_info.data.borrow()).unwrap();
        assert_eq!(sync_data.collateral_token_key, collateral_token_key);
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
