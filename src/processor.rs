//! Program state processor

use crate::{error::SynchronizerError, instruction::{MAX_ORACLES, MAX_SIGNERS, SynchronizerInstruction}, state::SynchronizerData};
use num_traits::FromPrimitive;
use solana_program::{account_info::{next_account_info, AccountInfo}, decode_error::DecodeError, entrypoint::ProgramResult, msg, program::{invoke}, program_error::{PrintProgramError, ProgramError}, program_option::COption, program_pack::Pack, pubkey::Pubkey, rent::Rent, sysvar::Sysvar};
use spl_token::{error::TokenError, state::{Account, Mint}};

// Synchronizer program_id
solana_program::declare_id!("urNhxed8ocNiFApoooLSAJ1xnWSMUiC9S6fKcRon1rk");

/// Checks that the supplied program ID is the correct
pub fn check_program_account(program_id: &Pubkey) -> ProgramResult {
    if program_id != &id() {
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}

pub struct Processor {}
impl Processor {
/// Default Scale
pub const DEFAULT_DECIMALS: u8 = 9;

// Instructions handlers

pub fn process_buy_for(
    accounts: &[AccountInfo],
    multiplier: u64,
    amount: u64,
    fee: u64,
    prices: &Vec<u64>,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let fiat_asset_mint_info = next_account_info(account_info_iter)?;
    let user_collateral_account_info = next_account_info(account_info_iter)?;
    let user_fiat_account_info = next_account_info(account_info_iter)?;
    let synchronizer_collateral_account_info = next_account_info(account_info_iter)?;
    let user_authority_info = next_account_info(account_info_iter)?;
    let synchronizer_authority_info = next_account_info(account_info_iter)?;
    let spl_token_info = next_account_info(account_info_iter)?;

    if !synchronizer_authority_info.owner.eq(&id()) {
        return Err(SynchronizerError::AccessDenied.into());
    }
    if !synchronizer_authority_info.is_signer {
        return Err(SynchronizerError::InvalidSigner.into());
    }
    if !user_authority_info.is_signer {
        return Err(SynchronizerError::InvalidSigner.into());
    }

    let mut synchronizer = SynchronizerData::unpack_unchecked(&synchronizer_authority_info.data.borrow())?;
    if !synchronizer.is_initialized {
        return Err(SynchronizerError::NotInitialized.into());
    }

    let oracles_infos = account_info_iter.as_slice();
    if oracles_infos.len() < synchronizer.minimum_required_signature as usize {
        return Err(SynchronizerError::NotEnoughOracles.into());
    }
    if prices.len() < synchronizer.minimum_required_signature as usize {
        return Err(SynchronizerError::NotEnoughOracles.into());
    }

    let mut price = prices[0];
    for i in 0..synchronizer.minimum_required_signature as usize {
        let oracle = oracles_infos.iter().next().unwrap();
        if !synchronizer.oracles.contains(&oracle.key) || !oracle.is_signer {
            return Err(SynchronizerError::BadOracle.into());
        }

        if prices[i] > price {
            price = prices[i];
        }
    }

    let synchronizer_collateral_account = Account::unpack(&synchronizer_collateral_account_info.data.borrow()).unwrap();
    let user_collateral_account = Account::unpack(&user_collateral_account_info.data.borrow()).unwrap();
    if !synchronizer_collateral_account.mint.eq(&synchronizer.collateral_token_key) {
        return Err(SynchronizerError::BadCollateralMint.into());
    }
    if !synchronizer_collateral_account.owner.eq(synchronizer_authority_info.key) {
        return Err(TokenError::OwnerMismatch.into());
    }
    if !user_collateral_account.mint.eq(&synchronizer.collateral_token_key) {
        return Err(SynchronizerError::BadCollateralMint.into());
    }
    if !user_collateral_account.owner.eq(user_authority_info.key) {
        return Err(TokenError::OwnerMismatch.into());
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

    if !Account::unpack(&user_fiat_account_info.data.borrow()).unwrap().owner.eq(user_authority_info.key) {
        return Err(TokenError::OwnerMismatch.into())
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
    let instruction = spl_token::instruction::transfer(
        &spl_token::id(),
        &user_collateral_account_info.key,
        &synchronizer_collateral_account_info.key,
        &user_authority_info.key,
        &[],
        collateral_amount + fee_amount
    ).unwrap();
    let account_infos = [
        spl_token_info.clone(),
        user_collateral_account_info.clone(),
        synchronizer_collateral_account_info.clone(),
        user_authority_info.clone(),
    ];
    invoke(&instruction, &account_infos)?;
    msg!("Transfer {} collateral tokens from user to synchronizer", collateral_amount + fee_amount);

    // Synchronizer mint fiat asset to user associated token account
    let instruction = spl_token::instruction::mint_to(
        &spl_token::id(),
        &fiat_asset_mint_info.key,
        &user_fiat_account_info.key,
        &synchronizer_authority_info.key,
        &[],
        amount
    ).unwrap();
    let account_infos = [
        spl_token_info.clone(),
        fiat_asset_mint_info.clone(),
        user_fiat_account_info.clone(),
        synchronizer_authority_info.clone(),
    ];
    invoke(&instruction, &account_infos)?;
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
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let fiat_asset_mint_info = next_account_info(account_info_iter)?;
    let user_collateral_account_info = next_account_info(account_info_iter)?;
    let user_fiat_account_info = next_account_info(account_info_iter)?;
    let synchronizer_collateral_account_info = next_account_info(account_info_iter)?;
    let user_authority_info = next_account_info(account_info_iter)?;
    let synchronizer_authority_info = next_account_info(account_info_iter)?;
    let spl_token_info = next_account_info(account_info_iter)?;

    if !synchronizer_authority_info.owner.eq(&id()) {
        return Err(SynchronizerError::AccessDenied.into());
    }
    if !synchronizer_authority_info.is_signer {
        return Err(SynchronizerError::InvalidSigner.into());
    }
    if !user_authority_info.is_signer {
        return Err(SynchronizerError::InvalidSigner.into());
    }

    let mut synchronizer = SynchronizerData::unpack_unchecked(&synchronizer_authority_info.data.borrow())?;
    if !synchronizer.is_initialized {
        return Err(SynchronizerError::NotInitialized.into());
    }

    let oracles_infos = account_info_iter.as_slice();
    if oracles_infos.len() < synchronizer.minimum_required_signature as usize {
        return Err(SynchronizerError::NotEnoughOracles.into());
    }
    if prices.len() < synchronizer.minimum_required_signature as usize {
        return Err(SynchronizerError::NotEnoughOracles.into());
    }

    let mut price = prices[0];
    for i in 0..synchronizer.minimum_required_signature as usize {
        let oracle = oracles_infos.iter().next().unwrap();
        if !synchronizer.oracles.contains(&oracle.key) || !oracle.is_signer {
            return Err(SynchronizerError::BadOracle.into());
        }

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
    if !synchronizer_collateral_account.owner.eq(synchronizer_authority_info.key) {
        return Err(TokenError::OwnerMismatch.into());
    }
    if !user_collateral_account.owner.eq(user_authority_info.key) {
        return Err(TokenError::OwnerMismatch.into());
    }

    let user_fiat_account = Account::unpack(&user_fiat_account_info.data.borrow()).unwrap();
    if !user_fiat_account.owner.eq(user_authority_info.key) {
        return Err(TokenError::OwnerMismatch.into());
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

    if user_fiat_account.amount < amount {
        return Err(SynchronizerError::InsufficientFunds.into());
    }
    if synchronizer_collateral_account.amount < (collateral_amount - fee_amount) {
        return Err(SynchronizerError::InsufficientFunds.into());
    }

    // Burn fiat asset from user
    let instruction = spl_token::instruction::burn(
        &spl_token::id(),
        &user_fiat_account_info.key,
        &fiat_asset_mint_info.key,
        &user_authority_info.key,
        &[],
        amount
    ).unwrap();
    let account_infos = [
        spl_token_info.clone(),
        user_fiat_account_info.clone(),
        fiat_asset_mint_info.clone(),
        user_authority_info.clone(),
    ];
    invoke(&instruction, &account_infos)?;
    msg!("Burn {} fiat assets from user_account", amount);

    // Transfer collateral token from synchronizer to user
    let instruction = spl_token::instruction::transfer(
        &spl_token::id(),
        &synchronizer_collateral_account_info.key,
        &user_collateral_account_info.key,
        &synchronizer_authority_info.key,
        &[],
        collateral_amount - fee_amount
    )?;
    let account_infos = [
        spl_token_info.clone(),
        synchronizer_collateral_account_info.clone(),
        user_collateral_account_info.clone(),
        synchronizer_authority_info.clone(),
    ];
    invoke(&instruction, &account_infos)?;
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
    minimum_required_signature: u8,
    oracles: Vec<Pubkey>,
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

    if oracles.len() > MAX_ORACLES {
        return Err(SynchronizerError::MaxOraclesExceed.into());
    }

    if minimum_required_signature > MAX_SIGNERS {
        return Err(SynchronizerError::MaxSignersExceed.into());
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
    for (i, oracle) in oracles.iter().enumerate() {
        synchronizer.oracles[i] = *oracle;
    }

    SynchronizerData::pack(synchronizer, &mut synchronizer_account_info.data.borrow_mut())?;

    Ok(())
}

pub fn process_set_minimum_required_signature(
    accounts: &[AccountInfo],
    minimum_required_signature: u8,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let synchronizer_account_info = next_account_info(account_info_iter)?;

    if !synchronizer_account_info.owner.eq(&id()) {
        return Err(SynchronizerError::AccessDenied.into());
    }

    if !synchronizer_account_info.is_signer {
        return Err(SynchronizerError::InvalidSigner.into());
    }

    if minimum_required_signature > MAX_ORACLES as u8 {
        return Err(SynchronizerError::MaxOraclesExceed.into());
    }

    if minimum_required_signature > MAX_SIGNERS {
        return Err(SynchronizerError::MaxSignersExceed.into());
    }

    let mut synchronizer = SynchronizerData::unpack_unchecked(&synchronizer_account_info.data.borrow())?;
    if !synchronizer.is_initialized {
        return Err(SynchronizerError::NotInitialized.into());
    }

    msg!("Set minimum required signature {}", minimum_required_signature);
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

    msg!("Set collateral token key {}", collateral_token_key);
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

    msg!("Set remaining dollar cap {}", remaining_dollar_cap);
    synchronizer.remaining_dollar_cap = remaining_dollar_cap;
    SynchronizerData::pack(synchronizer, &mut synchronizer_account_info.data.borrow_mut())?;

    Ok(())
}

pub fn process_set_oracles(
    accounts: &[AccountInfo],
    oracles: Vec<Pubkey>,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let synchronizer_account_info = next_account_info(account_info_iter)?;

    if !synchronizer_account_info.owner.eq(&id()) {
        return Err(SynchronizerError::AccessDenied.into());
    }

    if !synchronizer_account_info.is_signer {
        return Err(SynchronizerError::InvalidSigner.into());
    }

    if oracles.len() > MAX_ORACLES {
        return Err(SynchronizerError::MaxOraclesExceed.into());
    }

    let mut synchronizer = SynchronizerData::unpack_unchecked(&synchronizer_account_info.data.borrow())?;
    if !synchronizer.is_initialized {
        return Err(SynchronizerError::NotInitialized.into());
    }

    msg!("Set oracles {:?}", oracles);
    for i in 0..MAX_ORACLES {
        synchronizer.oracles[i] = Pubkey::default();
    }
    for (i, oracle) in oracles.iter().enumerate() {
        synchronizer.oracles[i] = *oracle;
    }

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
    let spl_token_info = next_account_info(account_info_iter)?;

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

    let instruction = spl_token::instruction::transfer(
        &spl_token::id(),
        &synchronizer_collateral_account_info.key,
        &recipient_collateral_account_info.key,
        &synchronizer_account_info.key,
        &[],
        amount
    ).unwrap();
    let account_infos = [
        spl_token_info.clone(),
        synchronizer_collateral_account_info.clone(),
        recipient_collateral_account_info.clone(),
        synchronizer_account_info.clone(),
    ];
    invoke(&instruction, &account_infos)?;
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
    let spl_token_info = next_account_info(account_info_iter)?;

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

    if Account::unpack(&synchronizer_collateral_account_info.data.borrow()).unwrap().amount < amount {
        return Err(SynchronizerError::InsufficientFunds.into());
    }

    let instruction = spl_token::instruction::transfer(
        &spl_token::id(),
        &synchronizer_collateral_account_info.key,
        &recipient_collateral_account_info.key,
        &synchronizer_account_info.key,
        &[],
        amount
    ).unwrap();
    let account_infos = [
        spl_token_info.clone(),
        synchronizer_collateral_account_info.clone(),
        recipient_collateral_account_info.clone(),
        synchronizer_account_info.clone(),
    ];
    invoke(&instruction, &account_infos)?;
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
        } => {
            msg!("Instruction: BuyFor");
            Self::process_buy_for(accounts, multiplier, amount, fee, prices)
        }
        SynchronizerInstruction::SellFor {
            multiplier,
            amount,
            fee,
            ref prices,
        } => {
            msg!("Instruction: SellFor");
            Self::process_sell_for(accounts, multiplier, amount, fee, prices)
        }

        // Admin Instructions
        SynchronizerInstruction::InitializeSynchronizerAccount {
            collateral_token_key,
            remaining_dollar_cap,
            withdrawable_fee_amount,
            minimum_required_signature,
            oracles
        } => {
            msg!("Instruction: InitializeSynchronizerAccount");
            Self::process_initialize_synchronizer_account(accounts, collateral_token_key, remaining_dollar_cap, withdrawable_fee_amount, minimum_required_signature, oracles)
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

        SynchronizerInstruction::SetOracles {
            oracles
        } => {
            msg!("Instruction: SetOracles");
            Self::process_set_oracles(accounts, oracles)
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
            SynchronizerError::MaxOraclesExceed => msg!("Error: Exceed limit of maximum oracles"),
            SynchronizerError::MaxSignersExceed => msg!("Error: Exceed limit of maximum signers"),
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
    use solana_program::{instruction::Instruction, program_error::ProgramError, program_pack::Pack};
    use solana_sdk::{
        account::{create_is_signer_account_infos,Account as SolanaAccount,create_account_for_test},
    };
    use spl_token::{processor::Processor as SPLTokenProcessor, state::{Account, Mint}, ui_amount_to_amount};
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

    fn do_process(
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
        Processor::process_instruction(&instruction.program_id, &account_infos, &instruction.data)
    }

    #[test]
    fn test_init_synchronizer_account() {
        let program_id = id();
        let synchronizer_key = Pubkey::new_unique();
        let mut synchronizer_account = SolanaAccount::new(init_acc_minimum_balance(), SynchronizerData::get_packed_len(), &program_id);
        let mut rent_sysvar_account = create_account_for_test(&Rent::default());
        let collateral_key = Pubkey::new_unique();
        let oracles = vec![Pubkey::new_unique(), Pubkey::new_unique()];

        let mut bad_sync_acc = SolanaAccount::new(init_acc_minimum_balance() - 100, SynchronizerData::get_packed_len(), &program_id);
        assert_eq!(
            Err(SynchronizerError::NotRentExempt.into()),
            do_process(
                crate::instruction::initialize_synchronizer_account(
                    &id(),
                    &collateral_key,
                    0,
                    0,
                    2,
                    &oracles,
                    &synchronizer_key,
                ).unwrap(),
                vec![&mut bad_sync_acc, &mut rent_sysvar_account]
            )
        );

        let fake_program_id = Pubkey::new_unique();
        let mut bad_sync_acc = SolanaAccount::new(init_acc_minimum_balance(), SynchronizerData::get_packed_len(), &fake_program_id);
        assert_eq!(
            Err(SynchronizerError::AccessDenied.into()), // cause of bad owner
            do_process(
                crate::instruction::initialize_synchronizer_account(
                    &id(),
                    &collateral_key,
                    0,
                    0,
                    2,
                    &oracles,
                    &synchronizer_key
                ).unwrap(),
                vec![&mut bad_sync_acc, &mut rent_sysvar_account]
            )
        );

        let fake_sync_key = Pubkey::new_unique();
        let mut fake_sync_acc = SolanaAccount::new(init_acc_minimum_balance(), SynchronizerData::get_packed_len(), &fake_sync_key);
        assert_eq!(
            Err(SynchronizerError::AccessDenied.into()), // bad program_id
            do_process(
                crate::instruction::initialize_synchronizer_account(
                    &id(),
                    &collateral_key,
                    0,
                    0,
                    2,
                    &oracles,
                    &fake_sync_key
                ).unwrap(),
                vec![&mut fake_sync_acc, &mut rent_sysvar_account]
            )
        );

        do_process(
            crate::instruction::initialize_synchronizer_account(
                &id(),
                &collateral_key,
                0,
                0,
                2,
                &oracles,
                &synchronizer_key
            ).unwrap(),
            vec![&mut synchronizer_account, &mut rent_sysvar_account]
        ).unwrap();

        assert_eq!(
            Err(SynchronizerError::AlreadyInitialized.into()),
            do_process(
                crate::instruction::initialize_synchronizer_account(
                    &id(),
                    &collateral_key,
                    0,
                    0,
                    2,
                    &oracles,
                    &synchronizer_key
                ).unwrap(),
                vec![&mut synchronizer_account, &mut rent_sysvar_account]
            )
        );
    }

    #[test]
    fn test_public_api() {
        let program_id = &id();
        let synchronizer_key = Pubkey::new_unique();
        let mut synchronizer_account = SolanaAccount::new(init_acc_minimum_balance(), SynchronizerData::get_packed_len(), &program_id);
        let mut rent_sysvar = create_account_for_test(&Rent::default());
        let mut spl_token_account = SolanaAccount::default();
        let collateral_key = Pubkey::new_unique();
        let oracles = vec![Pubkey::new_unique(), Pubkey::new_unique()];
        let mut oracle1_acc = SolanaAccount::default();
        let mut oracle2_acc = SolanaAccount::default();
        let user_key = Pubkey::new_unique();
        let mut user_account = SolanaAccount::default();

        // Infrastructure preparing
        // Create and init collateral token
        let decimals = Processor::DEFAULT_DECIMALS;
        let mut collateral_asset_mint = SolanaAccount::new(mint_minimum_balance(), Mint::get_packed_len(), &spl_token::id());
        do_token_program(
            spl_token::instruction::initialize_mint(&spl_token::id(), &collateral_key, &synchronizer_key, None, decimals).unwrap(),
            vec![&mut collateral_asset_mint, &mut rent_sysvar],
        ).unwrap();

        // Create and init fiat asset token
        let fiat_asset_key = Pubkey::new_unique();
        let mut fiat_asset_mint = SolanaAccount::new(mint_minimum_balance(), Mint::get_packed_len(), &spl_token::id());
        do_token_program(
            spl_token::instruction::initialize_mint(&spl_token::id(), &fiat_asset_key, &synchronizer_key, None, decimals).unwrap(),
            vec![&mut fiat_asset_mint, &mut rent_sysvar],
        ).unwrap();

        // Create and init token associated accounts for synchronizer
        let synchronizer_collateral_key = Pubkey::new_unique();
        let mut synchronizer_collateral_account = SolanaAccount::new(account_minimum_balance(), Account::get_packed_len(), &spl_token::id());
        do_token_program(
            spl_token::instruction::initialize_account(&spl_token::id(), &synchronizer_collateral_key, &collateral_key, &synchronizer_key).unwrap(),
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
            spl_token::instruction::initialize_account(&spl_token::id(), &user_collateral_key, &collateral_key, &user_key).unwrap(),
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
            spl_token::instruction::initialize_account(&spl_token::id(), &user_fiat_key, &fiat_asset_key, &user_key).unwrap(),
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
            spl_token::instruction::mint_to(&spl_token::id(), &collateral_key, &synchronizer_collateral_key, &synchronizer_key, &[], amount).unwrap(),
            vec![&mut collateral_asset_mint, &mut synchronizer_collateral_account, &mut synchronizer_account],
        ).unwrap();

        // Mint some collateral asset to user account
        let amount = spl_token::ui_amount_to_amount(500.0, decimals);
        do_token_program(
            spl_token::instruction::mint_to(&spl_token::id(), &collateral_key, &user_collateral_key, &synchronizer_key, &[], amount).unwrap(),
            vec![&mut collateral_asset_mint, &mut user_collateral_account, &mut synchronizer_account],
        ).unwrap();

        // Mint some fiat asset to user account
        let amount = spl_token::ui_amount_to_amount(500.0, decimals);
        do_token_program(
            spl_token::instruction::mint_to(&spl_token::id(), &fiat_asset_key, &user_fiat_key, &synchronizer_key, &[], amount).unwrap(),
            vec![&mut fiat_asset_mint, &mut user_fiat_account, &mut synchronizer_account],
        ).unwrap();

        // Initialize Syncronizer account
        do_process(
            crate::instruction::initialize_synchronizer_account(
                &id(),
                &collateral_key,
                spl_token::ui_amount_to_amount(500.0, decimals),
                0,
                oracles.len() as u8,
                &oracles,
                &synchronizer_key
            ).unwrap(),
            vec![&mut synchronizer_account, &mut rent_sysvar]
        ).unwrap();

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
        assert_eq!(
            Err(SynchronizerError::AccessDenied.into()),
            do_process(
                crate::instruction::sell_for(
                    program_id,
                    mul_stocks,
                    sell_fiat_amount,
                    fee,
                    &prices,
                    &oracles,
                    &fiat_asset_key,
                    &user_collateral_key,
                    &user_fiat_key,
                    &synchronizer_collateral_key,
                    &user_key,
                    &fake_synchronizer_key
                ).unwrap(),
                vec![
                    &mut fiat_asset_mint,
                    &mut user_collateral_account,
                    &mut user_fiat_account,
                    &mut synchronizer_collateral_account,
                    &mut user_account,
                    &mut fake_synchronizer_account,
                    &mut spl_token_account,
                    &mut oracle1_acc,
                    &mut oracle2_acc,
                ]
            )
        );

        assert_eq!(
            Err(SynchronizerError::NotEnoughOracles.into()),
            do_process(
                crate::instruction::sell_for(
                    program_id,
                    mul_stocks,
                    sell_fiat_amount,
                    fee,
                    &prices,
                    &oracles,
                    &fiat_asset_key,
                    &user_collateral_key,
                    &user_fiat_key,
                    &synchronizer_collateral_key,
                    &user_key,
                    &synchronizer_key
                ).unwrap(),
                vec![
                    &mut fiat_asset_mint,
                    &mut user_collateral_account,
                    &mut user_fiat_account,
                    &mut synchronizer_collateral_account,
                    &mut user_account,
                    &mut synchronizer_account,
                    &mut spl_token_account,
                    &mut oracle1_acc,
                ]
            )
        );

        assert_eq!(
            user_key,
            Account::unpack_unchecked(&user_fiat_account.data).unwrap().owner
        );

        let synchronizer = SynchronizerData::unpack(&synchronizer_account.data).unwrap();
        assert_eq!(synchronizer.remaining_dollar_cap, 500_000_000_000);
        assert_eq!(synchronizer.withdrawable_fee_amount, 0);

        do_process(
            crate::instruction::sell_for(
                program_id,
                mul_stocks,
                sell_fiat_amount,
                fee,
                &prices,
                &oracles,
                &fiat_asset_key,
                &user_collateral_key,
                &user_fiat_key,
                &synchronizer_collateral_key,
                &user_key,
                &synchronizer_key
            ).unwrap(),
            vec![
                &mut fiat_asset_mint,
                &mut user_collateral_account,
                &mut user_fiat_account,
                &mut synchronizer_collateral_account,
                &mut user_account,
                &mut synchronizer_account,
                &mut spl_token_account,
                &mut oracle1_acc,
                &mut oracle2_acc,
            ]
        ).unwrap();

        // Test buy_for instruction
        let buy_fiat_amount = spl_token::ui_amount_to_amount(50.0, decimals);
        // Case: bad synchronizer signer
        let fake_synchronizer_key = Pubkey::new_unique();
        let mut fake_synchronizer_account = SolanaAccount::default();
        assert_eq!(
            Err(SynchronizerError::AccessDenied.into()),
            do_process(
                crate::instruction::buy_for(
                    program_id,
                    mul_stocks,
                    buy_fiat_amount,
                    fee,
                    &prices,
                    &oracles,
                    &fiat_asset_key,
                    &user_collateral_key,
                    &user_fiat_key,
                    &synchronizer_collateral_key,
                    &user_key,
                    &fake_synchronizer_key
                ).unwrap(),
                vec![
                    &mut fiat_asset_mint,
                    &mut user_collateral_account,
                    &mut user_fiat_account,
                    &mut synchronizer_collateral_account,
                    &mut user_account,
                    &mut fake_synchronizer_account,
                    &mut spl_token_account,
                    &mut oracle1_acc,
                    &mut oracle2_acc,
                ]
            )
        );

        assert_eq!(
            Err(SynchronizerError::NotEnoughOracles.into()),
            do_process(
                crate::instruction::buy_for(
                    program_id,
                    mul_stocks,
                    buy_fiat_amount,
                    fee,
                    &prices,
                    &oracles,
                    &fiat_asset_key,
                    &user_collateral_key,
                    &user_fiat_key,
                    &synchronizer_collateral_key,
                    &user_key,
                    &synchronizer_key
                ).unwrap(),
                vec![
                    &mut fiat_asset_mint,
                    &mut user_collateral_account,
                    &mut user_fiat_account,
                    &mut synchronizer_collateral_account,
                    &mut user_account,
                    &mut synchronizer_account,
                    &mut spl_token_account,
                    &mut oracle1_acc,
                ]
            )
        );

        // Good case
        let synchronizer = SynchronizerData::unpack(&synchronizer_account.data).unwrap();
        assert_eq!(synchronizer.remaining_dollar_cap, 580_000_000_000);
        assert_eq!(synchronizer.withdrawable_fee_amount, 40_000_000);

        do_process(
            crate::instruction::buy_for(
                program_id,
                mul_stocks,
                buy_fiat_amount,
                fee,
                &prices,
                &oracles,
                &fiat_asset_key,
                &user_collateral_key,
                &user_fiat_key,
                &synchronizer_collateral_key,
                &user_key,
                &synchronizer_key
            ).unwrap(),
            vec![
                &mut fiat_asset_mint,
                &mut user_collateral_account,
                &mut user_fiat_account,
                &mut synchronizer_collateral_account,
                &mut user_account,
                &mut synchronizer_account,
                &mut spl_token_account,
                &mut oracle1_acc,
                &mut oracle2_acc,
            ]
        ).unwrap();

        // BadCase: too big buy amount
        let buy_fiat_amount = spl_token::ui_amount_to_amount(999999.0, decimals);
        assert_eq!(
            Err(SynchronizerError::InsufficientFunds.into()),
            do_process(
                crate::instruction::buy_for(
                    program_id,
                    mul_stocks,
                    buy_fiat_amount,
                    fee,
                    &prices,
                    &oracles,
                    &fiat_asset_key,
                    &user_collateral_key,
                    &user_fiat_key,
                    &synchronizer_collateral_key,
                    &user_key,
                    &synchronizer_key
                ).unwrap(),
                vec![
                    &mut fiat_asset_mint,
                    &mut user_collateral_account,
                    &mut user_fiat_account,
                    &mut synchronizer_collateral_account,
                    &mut user_account,
                    &mut synchronizer_account,
                    &mut spl_token_account,
                    &mut oracle1_acc,
                    &mut oracle2_acc,
                ]
            )
        );

        // BadCase: too big sell amount
        let sell_fiat_amount = spl_token::ui_amount_to_amount(999999.0, decimals);
        assert_eq!(
            Err(SynchronizerError::InsufficientFunds.into()),
            do_process(
                crate::instruction::sell_for(
                    program_id,
                    mul_stocks,
                    sell_fiat_amount,
                    fee,
                    &prices,
                    &oracles,
                    &fiat_asset_key,
                    &user_collateral_key,
                    &user_fiat_key,
                    &synchronizer_collateral_key,
                    &user_key,
                    &synchronizer_key
                ).unwrap(),
                vec![
                    &mut fiat_asset_mint,
                    &mut user_collateral_account,
                    &mut user_fiat_account,
                    &mut synchronizer_collateral_account,
                    &mut user_account,
                    &mut synchronizer_account,
                    &mut spl_token_account,
                    &mut oracle1_acc,
                    &mut oracle2_acc,
                ]
            )
        );

        // BadCase: wrong oracles
        let oracles = vec![Pubkey::new_unique(), Pubkey::new_unique()];
        assert_eq!(
            Err(SynchronizerError::BadOracle.into()),
            do_process(
                crate::instruction::sell_for(
                    program_id,
                    mul_stocks,
                    sell_fiat_amount,
                    fee,
                    &prices,
                    &oracles,
                    &fiat_asset_key,
                    &user_collateral_key,
                    &user_fiat_key,
                    &synchronizer_collateral_key,
                    &user_key,
                    &synchronizer_key
                ).unwrap(),
                vec![
                    &mut fiat_asset_mint,
                    &mut user_collateral_account,
                    &mut user_fiat_account,
                    &mut synchronizer_collateral_account,
                    &mut user_account,
                    &mut synchronizer_account,
                    &mut spl_token_account,
                    &mut oracle1_acc,
                    &mut oracle2_acc,
                ]
            )
        );

        assert_eq!(
            Err(SynchronizerError::BadOracle.into()),
            do_process(
                crate::instruction::buy_for(
                    program_id,
                    mul_stocks,
                    buy_fiat_amount,
                    fee,
                    &prices,
                    &oracles,
                    &fiat_asset_key,
                    &user_collateral_key,
                    &user_fiat_key,
                    &synchronizer_collateral_key,
                    &user_key,
                    &synchronizer_key
                ).unwrap(),
                vec![
                    &mut fiat_asset_mint,
                    &mut user_collateral_account,
                    &mut user_fiat_account,
                    &mut synchronizer_collateral_account,
                    &mut user_account,
                    &mut synchronizer_account,
                    &mut spl_token_account,
                    &mut oracle1_acc,
                    &mut oracle2_acc,
                ]
            )
        );

        // BadCase: too much oracles (11)
        let prices = vec![1, 1, 1, 1, 1, 1, 1, 1, 1, 1];
        let oracles = vec![
            Pubkey::new_unique(), Pubkey::new_unique(),
            Pubkey::new_unique(), Pubkey::new_unique(),
            Pubkey::new_unique(), Pubkey::new_unique(),
            Pubkey::new_unique(), Pubkey::new_unique(),
            Pubkey::new_unique(), Pubkey::new_unique(),
            Pubkey::new_unique(),
        ];
        assert_eq!(
            Err(SynchronizerError::MaxOraclesExceed.into()),
            do_process(
                crate::instruction::set_oracles(
                    program_id,
                    &oracles,
                    &synchronizer_key
                ).unwrap(),
                vec![
                    &mut synchronizer_account,
                ]
            )
        );

        let oracles = vec![
            Pubkey::new_unique(), Pubkey::new_unique(),
            Pubkey::new_unique(),
        ];
        let mut or1 = SolanaAccount::default();
        let mut or2 = SolanaAccount::default();
        let mut or3 = SolanaAccount::default();

        do_process(
            crate::instruction::set_oracles(
                program_id,
                &oracles,
                &synchronizer_key
            ).unwrap(),
            vec![
                &mut synchronizer_account,
            ]
        ).unwrap();

        let sell_fiat_amount = ui_amount_to_amount(10.0, decimals);
        do_process(
            crate::instruction::sell_for(
                program_id,
                mul_stocks,
                sell_fiat_amount,
                fee,
                &prices,
                &oracles,
                &fiat_asset_key,
                &user_collateral_key,
                &user_fiat_key,
                &synchronizer_collateral_key,
                &user_key,
                &synchronizer_key
            ).unwrap(),
            vec![&mut fiat_asset_mint, &mut user_collateral_account, &mut user_fiat_account,
                &mut synchronizer_collateral_account, &mut user_account, &mut synchronizer_account, &mut spl_token_account,
                &mut or1, &mut or2, &mut or3
            ]
        ).unwrap();
    }

    #[test]
    fn test_admin_setters() {
        let synchronizer_key = Pubkey::new_unique();
        let mut synchronizer_account = SolanaAccount::new(init_acc_minimum_balance(), SynchronizerData::get_packed_len(), &id());
        let mut rent_sysvar_account = create_account_for_test(&Rent::default());

        // BadCase: bad synchronizer account
        let mut fake_acc = SolanaAccount::default();
        assert_eq!(
            Err(SynchronizerError::AccessDenied.into()),
            do_process(
                crate::instruction::set_minimum_required_signature(&id(), 9, &synchronizer_key).unwrap(),
                vec![&mut fake_acc]
            )
        );
        assert_eq!(
            Err(SynchronizerError::AccessDenied.into()),
            do_process(
                crate::instruction::set_remaining_dollar_cap(&id(), 123456, &synchronizer_key).unwrap(),
                vec![&mut fake_acc]
            )
        );
        assert_eq!(
            Err(SynchronizerError::AccessDenied.into()),
            do_process(
                crate::instruction::set_collateral_token(&id(), &Pubkey::new_unique(), &synchronizer_key).unwrap(),
                vec![&mut fake_acc]
            )
        );

        // BadCase: Synchronizer account is not initialized
        assert_eq!(
            Err(SynchronizerError::NotInitialized.into()),
            do_process(
                crate::instruction::set_minimum_required_signature(&id(), 3, &synchronizer_key).unwrap(),
                vec![&mut synchronizer_account]
            )
        );
        assert_eq!(
            Err(SynchronizerError::NotInitialized.into()),
            do_process(
                crate::instruction::set_remaining_dollar_cap(&id(), 123456, &synchronizer_key).unwrap(),
                vec![&mut synchronizer_account]
            )
        );
        assert_eq!(
            Err(SynchronizerError::NotInitialized.into()),
            do_process(
                crate::instruction::set_collateral_token(&id(), &Pubkey::new_unique(), &synchronizer_key).unwrap(),
                vec![&mut synchronizer_account]
            )
        );

        // BadCase: limit exceed
        assert_eq!(
            Err(SynchronizerError::MaxOraclesExceed.into()),
            do_process(
                crate::instruction::set_minimum_required_signature(&id(), 123, &synchronizer_key).unwrap(),
                vec![&mut synchronizer_account]
            )
        );

        let start_collateral_token_key = Pubkey::new_unique();
        let oracles = vec![Pubkey::new_unique(), Pubkey::new_unique()];
        let start_remaining_dollar_cap: u64 = 10;
        let start_minimum_required_signature: u8 = oracles.len() as u8;
        do_process(
            crate::instruction::initialize_synchronizer_account(
                &id(),
                &start_collateral_token_key,
                start_remaining_dollar_cap,
                0,
                start_minimum_required_signature,
                &oracles,
                &synchronizer_key
            ).unwrap(),
            vec![&mut synchronizer_account, &mut rent_sysvar_account]
        ).unwrap();

        let sync_data = SynchronizerData::unpack(&synchronizer_account.data).unwrap();
        assert_eq!(sync_data.minimum_required_signature, start_minimum_required_signature);
        assert_eq!(sync_data.remaining_dollar_cap, start_remaining_dollar_cap);
        assert_eq!(sync_data.collateral_token_key, start_collateral_token_key);

        let minimum_required_signature = 3;
        do_process(
            crate::instruction::set_minimum_required_signature(&id(), minimum_required_signature, &synchronizer_key).unwrap(),
            vec![&mut synchronizer_account]
        ).unwrap();
        let sync_data = SynchronizerData::unpack(&synchronizer_account.data).unwrap();
        assert_eq!(sync_data.minimum_required_signature, minimum_required_signature);

        let remaining_dollar_cap: u64 = 123456;
        do_process(
            crate::instruction::set_remaining_dollar_cap(&id(), remaining_dollar_cap, &synchronizer_key).unwrap(),
            vec![&mut synchronizer_account]
        ).unwrap();
        let sync_data = SynchronizerData::unpack(&synchronizer_account.data).unwrap();
        assert_eq!(sync_data.remaining_dollar_cap, remaining_dollar_cap);

        let collateral_token_key = Pubkey::new_unique();
        do_process(
            crate::instruction::set_collateral_token(&id(), &collateral_token_key, &synchronizer_key).unwrap(),
            vec![&mut synchronizer_account]
        ).unwrap();
        let sync_data = SynchronizerData::unpack(&synchronizer_account.data).unwrap();
        assert_eq!(sync_data.collateral_token_key, collateral_token_key);
    }

    #[test]
    fn test_withdraw() {
        let synchronizer_key = Pubkey::new_unique();
        let mut synchronizer_account = SolanaAccount::new(init_acc_minimum_balance(), SynchronizerData::get_packed_len(), &id());
        let recipient_key = Pubkey::new_unique();
        let mut recipient_account = SolanaAccount::default();
        let mut rent_sysvar = create_account_for_test(&Rent::default());
        let mut spl_token_account = SolanaAccount::default();
        let collateral_token_key = Pubkey::new_unique();
        let oracles = vec![Pubkey::new_unique(), Pubkey::new_unique()];

        // Infrastructure preparing
        // Create and init collateral token
        let decimals = Processor::DEFAULT_DECIMALS;
        let mut collateral_asset_mint = SolanaAccount::new(mint_minimum_balance(), Mint::get_packed_len(), &spl_token::id());
        do_token_program(
            spl_token::instruction::initialize_mint(&spl_token::id(), &collateral_token_key, &synchronizer_key, None, decimals).unwrap(),
            vec![&mut collateral_asset_mint, &mut rent_sysvar],
        ).unwrap();

        // Create and init token associated accounts for synchronizer
        let synchronizer_collateral_key = Pubkey::new_unique();
        let mut synchronizer_collateral_account = SolanaAccount::new(account_minimum_balance(), Account::get_packed_len(), &spl_token::id());
        do_token_program(
            spl_token::instruction::initialize_account(&spl_token::id(), &synchronizer_collateral_key, &collateral_token_key, &synchronizer_key).unwrap(),
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
            spl_token::instruction::initialize_account(&spl_token::id(), &recipient_collateral_key, &collateral_token_key, &recipient_key).unwrap(),
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
            spl_token::instruction::mint_to(&spl_token::id(), &collateral_token_key, &synchronizer_collateral_key, &synchronizer_key, &[], amount).unwrap(),
            vec![&mut collateral_asset_mint, &mut synchronizer_collateral_account, &mut synchronizer_account],
        ).unwrap();

        // Mint some collateral asset to recipient account
        let amount = spl_token::ui_amount_to_amount(100.0, decimals);
        do_token_program(
            spl_token::instruction::mint_to(&spl_token::id(), &collateral_token_key, &recipient_collateral_key, &synchronizer_key, &[], amount).unwrap(),
            vec![&mut collateral_asset_mint, &mut recipient_collateral_account, &mut synchronizer_account],
        ).unwrap();

        // Initialize synchronizer account
        do_process(
            crate::instruction::initialize_synchronizer_account(
                &id(),
                &collateral_token_key,
                spl_token::ui_amount_to_amount(500.0, decimals),
                spl_token::ui_amount_to_amount(250.0, decimals),
                oracles.len() as u8,
                &oracles,
                &synchronizer_key
            ).unwrap(),
            vec![&mut synchronizer_account, &mut rent_sysvar]
        ).unwrap();

        let amount = spl_token::ui_amount_to_amount(300.0, decimals);
        assert_eq!(
            Err(SynchronizerError::InsufficientFunds.into()),
            do_process(
                crate::instruction::withdraw_fee(
                    &id(),
                    amount,
                    &synchronizer_collateral_key,
                    &recipient_collateral_key,
                    &synchronizer_key
                ).unwrap(),
                vec![
                    &mut synchronizer_collateral_account,
                    &mut recipient_collateral_account,
                    &mut synchronizer_account,
                    &mut spl_token_account
                ]
            )
        );

        let amount = spl_token::ui_amount_to_amount(3000.0, decimals);
        assert_eq!(
            Err(SynchronizerError::InsufficientFunds.into()),
            do_process(
                crate::instruction::withdraw_collateral(
                    &id(),
                    amount,
                    &synchronizer_collateral_key,
                    &recipient_collateral_key,
                    &synchronizer_key
                ).unwrap(),
                vec![
                    &mut synchronizer_collateral_account,
                    &mut recipient_collateral_account,
                    &mut synchronizer_account,
                    &mut spl_token_account
                ]
            )
        );
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
