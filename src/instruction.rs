//! Instructions supported by the Synchronizer.

use crate::{error::SynchronizerError, processor::check_program_account};
use solana_program::{instruction::{AccountMeta, Instruction}, program_error::ProgramError, pubkey::Pubkey, sysvar};
use std::{mem::size_of, convert::TryInto};

/// Maximum known oracles authorities
pub const MAX_ORACLES: usize = 3;
/// Maximum oracles signs in transaction
pub const MAX_SIGNERS: u8 = 3;

/// Instructions supported by the Synchronizer
#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub enum SynchronizerInstruction {
    /// User buys fiat asset for collateral tokens
    /// Accounts expected by this instruction:
    ///
    /// 0. `[writable]` The mint account of fiat asset
    /// 1. `[writable]` The user collateral token associated account (user source)
    /// 2. `[writable]` The user fiat asset token associated account (user destination)
    /// 3. `[writable]` The Synchronizer collateral token associated account (Synchronizer destination)
    /// 4. `[signer]` The user pubkey authority
    /// 5. `[writable, signer]` The Synchronizer account authority
    /// 6. `[]` Token program
    /// 7. `[]` N Oracles authority
    BuyFor {
        multiplier: u64,
        amount: u64,
        fee: u64,
        prices: Vec<u64>,
    },

    /// User sells fiat assets for collateral tokens
    ///
    /// Accounts expected by this instruction:
    /// 0. `[writable]` The mint account of fiat asset
    /// 1. `[writable]` The user collateral token associated account (user destination)
    /// 2. `[writable]` The user fiat asset token associated account (user source)
    /// 3. `[writable]` The Synchronizer collateral token associated account (Synchronizer source)
    /// 4. `[signer]` The user pubkey authority
    /// 5. `[writable, signer]` The Synchronizer account authority
    /// 6. `[]` Token program
    /// 7. `[]` N Oracles authority
    SellFor {
        multiplier: u64,
        amount: u64,
        fee: u64,
        prices: Vec<u64>,
    },

    /// Initialization of Synchronizer account
    /// Accounts expected by this instruction:
    ///
    /// 0. `[writable, signer]` The Synchronizer account authority
    /// 1. `[]` Rent sysvar
    InitializeSynchronizerAccount {
        collateral_token_key: Pubkey,
        remaining_dollar_cap: u64,
        withdrawable_fee_amount: u64,
        minimum_required_signature: u8,
        oracles: Vec<Pubkey>,
    },

    /// Set minimum required signature
    ///
    /// Accounts expected by this instruction:
    /// 0. `[signer]` The Synchronizer account authority
    SetMinimumRequiredSignature {
        minimum_required_signature: u8
    },

    /// Set collateral token key
    ///
    /// Accounts expected by this instruction:
    /// 0. `[signer]` The Synchronizer account authority
    SetCollateralToken {
        collateral_token_key: Pubkey
    },

    /// Set remaining dollar cap
    ///
    /// Accounts expected by this instruction:
    /// 0. `[signer]` The Synchronizer account authority
    SetRemainingDollarCap {
        remaining_dollar_cap: u64
    },

    /// Withdraw fee from Synchronizer account to recipient account
    ///
    /// Accounts expected by this instruction:
    /// 0. `[writable]` The Synchronizer collateral token associated account (source)
    /// 1. `[writable]` recipient collateral token associated account (detination)
    /// 2. `[writable, signer]` The Synchronizer account authority
    /// 3. `[]` Token program
    WithdrawFee {
        amount: u64
    },

    /// Withdraw collateral from Synchronizer account to recipient account
    ///
    /// Accounts expected by this instruction:
    /// 0. `[writable]` The Synchronizer collateral token associated account (source)
    /// 1. `[writable]` recipient collateral token associated account (detination)
    /// 2. `[writable, signer]` The Synchronizer account authority
    /// 3. `[]` Token program
    WithdrawCollateral {
        amount: u64
    },

    /// Set list of known oracles
    ///
    /// Accounts expected by this instruction:
    /// 0. `[writable, signer]` The Synchronizer account authority
    SetOracles {
        oracles: Vec<Pubkey>,
    }
}

impl SynchronizerInstruction {
    /// Unpacks a byte buffer into a SynchronizerInstruction.
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        use SynchronizerError::InvalidInstruction;

        let (&tag, rest) = input.split_first().ok_or(InvalidInstruction)?;
        Ok(match tag {
            // Public Instructions
            0 | 1 => {
                let (multiplier, rest) = rest.split_at(8);
                let multiplier = multiplier
                    .try_into()
                    .ok()
                    .map(u64::from_le_bytes)
                    .ok_or(InvalidInstruction)?;

                let (amount, rest) = rest.split_at(8);
                let amount = amount
                    .try_into()
                    .ok()
                    .map(u64::from_le_bytes)
                    .ok_or(InvalidInstruction)?;

                let (fee, rest) = rest.split_at(8);
                let fee = fee
                    .try_into()
                    .ok()
                    .map(u64::from_le_bytes)
                    .ok_or(InvalidInstruction)?;

                let (&prices_num, rest) = rest.split_first().ok_or(InvalidInstruction)?;
                let mut prices = Vec::with_capacity(prices_num as usize);
                let (price_slice, _rest) = rest.split_at(prices_num as usize * 8);
                for i in 0..prices_num {
                    let price = price_slice
                        .get(i as usize * 8 .. i as usize * 8 + 8)
                        .and_then(|slice| slice.try_into().ok())
                        .map(u64::from_le_bytes)
                        .ok_or(InvalidInstruction)?;
                    prices.push(price);
                }

                match tag {
                    0 => Self::BuyFor {multiplier, amount, fee, prices},
                    1 => Self::SellFor {multiplier, amount, fee, prices},
                    _ => unreachable!(),
                }
            }

            // Admin Instructions
            2 => {
                let (collateral_token_key, rest) = Self::unpack_pubkey(rest).unwrap();
                let (remaining_dollar_cap, rest) = rest.split_at(8);
                let remaining_dollar_cap = remaining_dollar_cap
                    .try_into()
                    .ok()
                    .map(u64::from_le_bytes)
                    .ok_or(InvalidInstruction)?;
                let (withdrawable_fee_amount, rest) = rest.split_at(8);
                let withdrawable_fee_amount = withdrawable_fee_amount
                    .try_into()
                    .ok()
                    .map(u64::from_le_bytes)
                    .ok_or(InvalidInstruction)?;

                let (&minimum_required_signature, rest) = rest.split_first().ok_or(InvalidInstruction)?;

                let (&oracles_num, rest) = rest.split_first().ok_or(InvalidInstruction)?;
                let mut oracles = Vec::with_capacity(oracles_num as usize);
                let (oracles_slice, _rest) = rest.split_at(oracles_num as usize * 32);
                for i in 0..oracles_num {
                    let oracle = oracles_slice.get(i as usize * 32 .. i as usize * 32 + 32).unwrap();
                    let (oracle, _) = Self::unpack_pubkey(oracle).unwrap();
                    oracles.push(oracle);
                }

                Self::InitializeSynchronizerAccount {
                    collateral_token_key,
                    remaining_dollar_cap,
                    withdrawable_fee_amount,
                    minimum_required_signature,
                    oracles
                }
            }

            3 => {
                let (&minimum_required_signature, _rest) = rest.split_first().ok_or(InvalidInstruction)?;

                Self::SetMinimumRequiredSignature {
                    minimum_required_signature
                }
            }

            4 => {
                let (collateral_token_key, _rest) = Self::unpack_pubkey(rest).unwrap();

                Self::SetCollateralToken {
                    collateral_token_key
                }
            }

            5 => {
                let (remaining_dollar_cap, _rest) = rest.split_at(8);
                let remaining_dollar_cap = remaining_dollar_cap
                    .try_into()
                    .ok()
                    .map(u64::from_le_bytes)
                    .ok_or(InvalidInstruction)?;

                Self:: SetRemainingDollarCap {
                    remaining_dollar_cap
                }
            }

            6 => {
                let (amount, _rest) = rest.split_at(8);
                let amount = amount
                    .try_into()
                    .ok()
                    .map(u64::from_le_bytes)
                    .ok_or(InvalidInstruction)?;

                Self::WithdrawFee {
                    amount
                }
            }

            7 => {
                let (amount, _rest) = rest.split_at(8);
                let amount = amount
                    .try_into()
                    .ok()
                    .map(u64::from_le_bytes)
                    .ok_or(InvalidInstruction)?;

                Self::WithdrawCollateral {
                    amount
                }
            }

            8 => {
                let (&oracles_num, rest) = rest.split_first().ok_or(InvalidInstruction)?;
                let mut oracles = Vec::with_capacity(oracles_num as usize);
                let (oracles_slice, _rest) = rest.split_at(oracles_num as usize * 32);
                for i in 0..oracles_num {
                    let oracle = oracles_slice.get(i as usize * 32 .. i as usize * 32 + 32).unwrap();
                    let (oracle, _) = Self::unpack_pubkey(oracle).unwrap();
                    oracles.push(oracle);
                }

                Self::SetOracles {
                    oracles
                }
            }

            _ => return Err(SynchronizerError::InvalidInstruction.into()),
        })
    }

    /// Packs a SynchronizerInstruction into a byte buffer.
    pub fn pack(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(size_of::<Self>());
        match self {
            // Public Instructions
            Self::BuyFor {
                multiplier,
                amount,
                fee,
                ref prices,
            } => {
                buf.push(0);
                buf.extend_from_slice(&multiplier.to_le_bytes());
                buf.extend_from_slice(&amount.to_le_bytes());
                buf.extend_from_slice(&fee.to_le_bytes());
                buf.push(prices.len().try_into().unwrap());
                for price in prices {
                    buf.extend_from_slice(&price.to_le_bytes());
                }
            },

            Self::SellFor {
                multiplier,
                amount,
                fee,
                ref prices,
            } => {
                buf.push(1);
                buf.extend_from_slice(&multiplier.to_le_bytes());
                buf.extend_from_slice(&amount.to_le_bytes());
                buf.extend_from_slice(&fee.to_le_bytes());
                buf.push(prices.len().try_into().unwrap());
                for price in prices {
                    buf.extend_from_slice(&price.to_le_bytes());
                }
            },

            // Admin Instructions
            Self::InitializeSynchronizerAccount {
                collateral_token_key,
                remaining_dollar_cap,
                withdrawable_fee_amount,
                minimum_required_signature,
                oracles
            } => {
                buf.push(2);
                buf.extend_from_slice(collateral_token_key.as_ref());
                buf.extend_from_slice(&remaining_dollar_cap.to_le_bytes());
                buf.extend_from_slice(&withdrawable_fee_amount.to_le_bytes());
                buf.push(*minimum_required_signature);
                buf.push(oracles.len().try_into().unwrap());
                for oracle in oracles {
                    buf.extend_from_slice(oracle.as_ref());
                }
            }

            Self::SetMinimumRequiredSignature {
                minimum_required_signature
            } => {
                buf.push(3);
                buf.push(*minimum_required_signature);
            },

            Self::SetCollateralToken {
                collateral_token_key
            } => {
                buf.push(4);
                buf.extend_from_slice(&collateral_token_key.as_ref());
            },

            Self::SetRemainingDollarCap {
                remaining_dollar_cap
            } => {
                buf.push(5);
                buf.extend_from_slice(&remaining_dollar_cap.to_le_bytes());
            },

            Self::WithdrawFee {
                amount
            } => {
                buf.push(6);
                buf.extend_from_slice(&amount.to_le_bytes());
            },

            Self::WithdrawCollateral {
                amount
            } => {
                buf.push(7);
                buf.extend_from_slice(&amount.to_le_bytes());
            },

            Self::SetOracles {
                oracles
            } => {
                buf.push(8);
                buf.push(oracles.len().try_into().unwrap());
                for oracle in oracles {
                    buf.extend_from_slice(oracle.as_ref());
                }
            }
        };
        buf
    }

    fn unpack_pubkey(input: &[u8]) -> Result<(Pubkey, &[u8]), ProgramError> {
        if input.len() >= 32 {
            let (key, rest) = input.split_at(32);
            let pk = Pubkey::new(key);
            Ok((pk, rest))
        } else {
            Err(SynchronizerError::InvalidInstruction.into())
        }
    }
}

/// Creates a `BuyFor` instruction
pub fn buy_for(
    program_id: &Pubkey,
    multiplier: u64,
    amount: u64,
    fee: u64,
    prices: &Vec<u64>,
    oracles: &Vec<Pubkey>,
    mint: &Pubkey,
    user_collateral_token_account: &Pubkey,
    user_fiat_token_account: &Pubkey,
    synchronizer_collateral_token_account: &Pubkey,
    user_authority: &Pubkey,
    synchronizer_authority: &Pubkey
) -> Result<Instruction, ProgramError> {
    check_program_account(program_id)?;
    let data = SynchronizerInstruction::BuyFor {
        amount,
        fee,
        multiplier,
        prices: prices.iter().cloned().collect(),
    }.pack();

    let mut accounts = Vec::with_capacity(7);
    accounts.push(AccountMeta::new(*mint, false));
    accounts.push(AccountMeta::new(*user_collateral_token_account, false));
    accounts.push(AccountMeta::new(*user_fiat_token_account, false));
    accounts.push(AccountMeta::new(*synchronizer_collateral_token_account, false));
    accounts.push(AccountMeta::new_readonly(*user_authority, true));
    accounts.push(AccountMeta::new(*synchronizer_authority, true));
    accounts.push(AccountMeta::new_readonly(spl_token::id(), false));
    for oracle in oracles {
        accounts.push(AccountMeta::new_readonly(*oracle, true));
    }

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data,
    })
}

/// Creates a `SellFor` instruction
pub fn sell_for(
    program_id: &Pubkey,
    multiplier: u64,
    amount: u64,
    fee: u64,
    prices: &Vec<u64>,
    oracles: &Vec<Pubkey>,
    mint: &Pubkey,
    user_collateral_token_account: &Pubkey,
    user_fiat_token_account: &Pubkey,
    synchronizer_collateral_token_account: &Pubkey,
    user_authority: &Pubkey,
    synchronizer_authority: &Pubkey
) -> Result<Instruction, ProgramError> {
    check_program_account(program_id)?;
    let data = SynchronizerInstruction::SellFor {
        amount,
        fee,
        multiplier,
        prices: prices.iter().cloned().collect(),
    }.pack();

    let mut accounts = Vec::with_capacity(7);
    accounts.push(AccountMeta::new(*mint, false));
    accounts.push(AccountMeta::new(*user_collateral_token_account, false));
    accounts.push(AccountMeta::new(*user_fiat_token_account, false));
    accounts.push(AccountMeta::new(*synchronizer_collateral_token_account, false));
    accounts.push(AccountMeta::new_readonly(*user_authority, true));
    accounts.push(AccountMeta::new(*synchronizer_authority, true));
    accounts.push(AccountMeta::new_readonly(spl_token::id(), false));
    for oracle in oracles {
        accounts.push(AccountMeta::new_readonly(*oracle, true));
    }

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data,
    })
}

/// Creates a `InitializeSynchronizerAccount` instruction
pub fn initialize_synchronizer_account(
    program_id: &Pubkey,
    collateral_token_key: &Pubkey,
    remaining_dollar_cap: u64,
    withdrawable_fee_amount: u64,
    minimum_required_signature: u8,
    oracles: &Vec<Pubkey>,
    synchronizer_authority: &Pubkey,
) -> Result<Instruction, ProgramError> {
    check_program_account(program_id)?;
    let data = SynchronizerInstruction::InitializeSynchronizerAccount {
        collateral_token_key: *collateral_token_key,
        remaining_dollar_cap,
        withdrawable_fee_amount,
        minimum_required_signature,
        oracles: oracles.iter().cloned().collect(),
    }.pack();

    let mut accounts = Vec::with_capacity(2);
    accounts.push(AccountMeta::new(*synchronizer_authority, true));
    accounts.push(AccountMeta::new_readonly(sysvar::rent::id(), false));

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data,
    })
}

/// Creates a `SetMinimumRequiredSignature` instruction
pub fn set_minimum_required_signature(
    program_id: &Pubkey,
    minimum_required_signature: u8,
    synchronizer_authority: &Pubkey,
) -> Result<Instruction, ProgramError> {
    check_program_account(program_id)?;
    let data = SynchronizerInstruction::SetMinimumRequiredSignature { minimum_required_signature }.pack();

    let mut accounts = Vec::with_capacity(1);
    accounts.push(AccountMeta::new(*synchronizer_authority, true));

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data,
    })
}

/// Creates a `SetCollateralToken` instruction
pub fn set_collateral_token(
    program_id: &Pubkey,
    collateral_token: &Pubkey,
    synchronizer_authority: &Pubkey,
) -> Result<Instruction, ProgramError> {
    check_program_account(program_id)?;
    let data = SynchronizerInstruction::SetCollateralToken { collateral_token_key: *collateral_token }.pack();

    let mut accounts = Vec::with_capacity(1);
    accounts.push(AccountMeta::new(*synchronizer_authority, true));

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data,
    })
}

/// Creates a `SetRemainingDollarCap` instruction
pub fn set_remaining_dollar_cap(
    program_id: &Pubkey,
    remaining_dollar_cap: u64,
    synchronizer_authority: &Pubkey,
) -> Result<Instruction, ProgramError> {
    check_program_account(program_id)?;
    let data = SynchronizerInstruction::SetRemainingDollarCap { remaining_dollar_cap }.pack();

    let mut accounts = Vec::with_capacity(1);
    accounts.push(AccountMeta::new(*synchronizer_authority, true));

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data,
    })
}

/// Creates a `WithdrawFee` instruction
pub fn withdraw_fee(
    program_id: &Pubkey,
    amount: u64,
    synchronizer_collateral_token_account: &Pubkey,
    recipient_collateral_token_account: &Pubkey,
    synchronizer_authority: &Pubkey,
) -> Result<Instruction, ProgramError> {
    check_program_account(program_id)?;
    let data = SynchronizerInstruction::WithdrawFee { amount }.pack();

    let mut accounts = Vec::with_capacity(4);
    accounts.push(AccountMeta::new(*synchronizer_collateral_token_account, false));
    accounts.push(AccountMeta::new(*recipient_collateral_token_account, false));
    accounts.push(AccountMeta::new(*synchronizer_authority, true));
    accounts.push(AccountMeta::new_readonly(spl_token::id(), false));

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data,
    })
}

/// Creates a `WithdrawCollateral` instruction
pub fn withdraw_collateral(
    program_id: &Pubkey,
    amount: u64,
    synchronizer_collateral_token_account: &Pubkey,
    recipient_collateral_token_account: &Pubkey,
    synchronizer_authority: &Pubkey,
) -> Result<Instruction, ProgramError> {
    check_program_account(program_id)?;
    let data = SynchronizerInstruction::WithdrawCollateral { amount }.pack();

    let mut accounts = Vec::with_capacity(4);
    accounts.push(AccountMeta::new(*synchronizer_collateral_token_account, false));
    accounts.push(AccountMeta::new(*recipient_collateral_token_account, false));
    accounts.push(AccountMeta::new(*synchronizer_authority, true));
    accounts.push(AccountMeta::new_readonly(spl_token::id(), false));

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data,
    })
}

/// Craetes a `SetOracles` instruction
pub fn set_oracles(
    program_id: &Pubkey,
    oracles: &Vec<Pubkey>,
    synchronizer_authority: &Pubkey,
) -> Result<Instruction, ProgramError> {
    check_program_account(program_id)?;
    let data = SynchronizerInstruction::SetOracles { oracles: oracles.iter().cloned().collect() }.pack();

    let mut accounts = Vec::with_capacity(1);
    accounts.push(AccountMeta::new(*synchronizer_authority, true));

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data,
    })
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn test_instruction_packing() {
        let check = SynchronizerInstruction::BuyFor {
            multiplier: 5,
            amount: 215,
            fee: 100,
            prices: vec![211, 123, 300],
        };
        let packed = check.pack();
        let mut expect = Vec::from([0u8]);
        expect.extend_from_slice(&[5, 0, 0, 0, 0, 0, 0, 0]);
        expect.extend_from_slice(&[215, 0, 0, 0, 0, 0, 0, 0]);
        expect.extend_from_slice(&[100, 0, 0, 0, 0, 0, 0, 0]);
        expect.extend_from_slice(&[3]);
        expect.extend_from_slice(&[211, 0, 0, 0, 0, 0, 0, 0]);
        expect.extend_from_slice(&[123, 0, 0, 0, 0, 0, 0, 0]);
        expect.extend_from_slice(&[44, 1, 0, 0, 0, 0, 0, 0]);
        assert_eq!(packed, expect);
        let unpacked = SynchronizerInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);

        let check = SynchronizerInstruction::SellFor {
            multiplier: 5,
            amount: 215,
            fee: 100,
            prices: vec![211, 123, 300],
        };
        let packed = check.pack();
        let mut expect = Vec::from([1u8]);
        expect.extend_from_slice(&[5, 0, 0, 0, 0, 0, 0, 0]);
        expect.extend_from_slice(&[215, 0, 0, 0, 0, 0, 0, 0]);
        expect.extend_from_slice(&[100, 0, 0, 0, 0, 0, 0, 0]);
        expect.extend_from_slice(&[3]);
        expect.extend_from_slice(&[211, 0, 0, 0, 0, 0, 0, 0]);
        expect.extend_from_slice(&[123, 0, 0, 0, 0, 0, 0, 0]);
        expect.extend_from_slice(&[44, 1, 0, 0, 0, 0, 0, 0]);
        assert_eq!(packed, expect);
        let unpacked = SynchronizerInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);

        let check = SynchronizerInstruction::InitializeSynchronizerAccount {
            collateral_token_key: Pubkey::from_str("D2YHis8gk2wRHkMEY7bULLsFUk277KdodWFR1nJ9SRgb").unwrap(),
            remaining_dollar_cap: 300,
            withdrawable_fee_amount: 200,
            minimum_required_signature: 3,
            oracles: vec![
                Pubkey::from_str("D2YHis8gk2wRHkMEY7bULLsFUk277KdodWFR1nJ9SRgb").unwrap(),
                Pubkey::from_str("EExYKmkDnS5HuUhb33e5ZeGHdZPCdQKJcQXDQTyWSb4X").unwrap()
            ],
        };
        let packed = check.pack();
        let mut expect = Vec::from([2u8]);
        expect.extend_from_slice(&[178, 177, 51, 164, 92, 30, 126, 138, 210, 146, 214, 193, 145, 103, 57, 185, 60, 120, 46, 119, 37, 184, 251, 108, 93, 90, 88, 249, 49, 176, 59, 160]);
        expect.extend_from_slice(&[44, 1, 0, 0, 0, 0, 0, 0]);
        expect.extend_from_slice(&[200, 0, 0, 0, 0, 0, 0, 0]);
        expect.extend_from_slice(&[3]);
        expect.extend_from_slice(&[2]);
        expect.extend_from_slice(&[178, 177, 51, 164, 92, 30, 126, 138, 210, 146, 214, 193, 145, 103, 57, 185, 60, 120, 46, 119, 37, 184, 251, 108, 93, 90, 88, 249, 49, 176, 59, 160]);
        expect.extend_from_slice(&[196, 187, 71, 168, 43, 226, 204, 130, 198, 182, 91, 6, 240, 228, 232, 228, 89, 217, 65, 173, 197, 180, 93, 22, 141, 243, 103, 79, 210, 0, 211, 76]);
        assert_eq!(packed, expect);
        let unpacked = SynchronizerInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);

        let check = SynchronizerInstruction::SetMinimumRequiredSignature {
            minimum_required_signature: 3,
        };
        let packed = check.pack();
        let mut expect = Vec::from([3u8]);
        expect.extend_from_slice(&[3]);
        assert_eq!(packed, expect);
        let unpacked = SynchronizerInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);

        let check = SynchronizerInstruction::SetCollateralToken {
            collateral_token_key: Pubkey::from_str("D2YHis8gk2wRHkMEY7bULLsFUk277KdodWFR1nJ9SRgb").unwrap()
        };
        let packed = check.pack();
        let mut expect = Vec::from([4u8]);
        expect.extend_from_slice(&[178, 177, 51, 164, 92, 30, 126, 138, 210, 146, 214, 193, 145, 103, 57, 185, 60, 120, 46, 119, 37, 184, 251, 108, 93, 90, 88, 249, 49, 176, 59, 160]);
        assert_eq!(packed, expect);
        let unpacked = SynchronizerInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);

        let check = SynchronizerInstruction::SetRemainingDollarCap {
            remaining_dollar_cap: 500_000_000_000
        };
        let packed = check.pack();
        let mut expect = Vec::from([5u8]);
        expect.extend_from_slice(&[0, 136, 82, 106, 116, 0, 0, 0]);
        assert_eq!(packed, expect);
        let unpacked = SynchronizerInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);

        let check = SynchronizerInstruction::WithdrawFee {
            amount: 500_000_000_000
        };
        let packed = check.pack();
        let mut expect = Vec::from([6u8]);
        expect.extend_from_slice(&[0, 136, 82, 106, 116, 0, 0, 0]);
        assert_eq!(packed, expect);
        let unpacked = SynchronizerInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);

        let check = SynchronizerInstruction::WithdrawCollateral {
            amount: 500_000_000_000
        };
        let packed = check.pack();
        let mut expect = Vec::from([7u8]);
        expect.extend_from_slice(&[0, 136, 82, 106, 116, 0, 0, 0]);
        assert_eq!(packed, expect);
        let unpacked = SynchronizerInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);

        let check = SynchronizerInstruction::SetOracles {
            oracles: vec![
                Pubkey::from_str("D2YHis8gk2wRHkMEY7bULLsFUk277KdodWFR1nJ9SRgb").unwrap(),
                Pubkey::from_str("EExYKmkDnS5HuUhb33e5ZeGHdZPCdQKJcQXDQTyWSb4X").unwrap(),
                Pubkey::from_str("48KTaZpsG3ksPfxGkGByQri85LVqMMnuZF1FKdgQuFFb").unwrap(),
                Pubkey::from_str("DUnw9uFDhmkqm6Wz4t5Z1fBYo5wNtU3FoNer4KLyRTab").unwrap(),
                Pubkey::from_str("7kVUKpM9Tz3H5aTPgfDFdWxgyCEWcx8orax7icVm629u").unwrap()
            ],
        };
        let packed = check.pack();
        let mut expect = Vec::from([8u8]);
        expect.extend_from_slice(&[5]);
        expect.extend_from_slice(&[178, 177, 51, 164, 92, 30, 126, 138, 210, 146, 214, 193, 145, 103, 57, 185, 60, 120, 46, 119, 37, 184, 251, 108, 93, 90, 88, 249, 49, 176, 59, 160]);
        expect.extend_from_slice(&[196, 187, 71, 168, 43, 226, 204, 130, 198, 182, 91, 6, 240, 228, 232, 228, 89, 217, 65, 173, 197, 180, 93, 22, 141, 243, 103, 79, 210, 0, 211, 76]);
        expect.extend_from_slice(&[46, 114, 255, 76, 55, 224, 86, 81, 80, 77, 238, 196, 54, 34, 220, 143, 51, 146, 94, 92, 180, 213, 127, 110, 44, 152, 35, 202, 56, 105, 199, 54, 185]);
        expect.extend_from_slice(&[106, 220, 194, 67, 146, 136, 46, 107, 170, 107, 183, 9, 110, 128, 95, 226, 235, 241, 127, 237, 200, 73, 88, 145, 62, 168, 192, 157, 143, 206, 100, 100]);
        expect.extend_from_slice(&[74, 73, 234, 151, 55, 189, 63, 169, 31, 58, 64, 4, 192, 129, 45, 231, 221, 183, 196, 119, 148, 203, 29, 109, 186, 145, 150, 226, 24, 95, 112]);
        assert_eq!(packed, expect);
        let unpacked = SynchronizerInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);
    }
}
