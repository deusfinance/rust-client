//! Instructions supported by the Synchronizer.

use crate::error::SynchronizerError;
use solana_program::{program_error::ProgramError, pubkey::Pubkey};
use std::{mem::size_of, convert::TryInto};

#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub enum SynchronizerInstruction {
    // Public Instructions

    // User buys fiat asset for collateral tokens
    // Accounts expected by this instruction:
    // 0. [writable] The mint account of fiat asset
    // 1. [writable] The user collateral token associated account (user source)
    // 2. [writable] The user fiat asset token associated account (user destination)
    // 3. [writable] The Synchronizer collateral token associated account (Synchronizer destination)
    // 4. [signer] The user pubkey authority
    // 5. [writable, signer] The Synchronizer account authority
    BuyFor {
        multiplier: u64,
        amount: u64,
        fee: u64,
        prices: Vec<u64>,
        oracles: Vec<Pubkey>,
    },

    // User sells fiat assets for collateral tokens
    // Accounts expected by this instruction:
    // 0. [writable] The mint account of fiat asset
    // 1. [writable] The user collateral token associated account (user destination)
    // 2. [writable] The user fiat asset token associated account (user source)
    // 3. [writable] The Synchronizer collateral token associated account (Synchronizer source)
    // 4. [signer] The user pubkey authority
    // 5. [writable, signer] The Synchronizer account authority
    SellFor {
        multiplier: u64,
        amount: u64,
        fee: u64,
        prices: Vec<u64>,
        oracles: Vec<Pubkey>,
    },

    // Admin Instructions
    // Initialization of Synchronizer account
    // Accounts expected by this instruction:
    // 0. [writable] The Synchronizer account authority
    // 1. [] Rent sysvar
    InitializeSynchronizerAccount {
        collateral_token_key: Pubkey,
        remaining_dollar_cap: u64,
        withdrawable_fee_amount: u64,
    },

    SetMinimumRequiredSignature,
    SetCollateralToken,
    SetRemainingDollarCap,

    WithdrawFee,
    WithdrawCollateral
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
                let (price_slice, rest) = rest.split_at(prices_num as usize * 8);
                for i in 0..prices_num {
                    let price = price_slice
                        .get(i as usize * 8 .. i as usize * 8 + 8)
                        .and_then(|slice| slice.try_into().ok())
                        .map(u64::from_le_bytes)
                        .ok_or(InvalidInstruction)?;
                    prices.push(price);
                }

                let (&oracles_num, rest) = rest.split_first().ok_or(InvalidInstruction)?;
                let mut oracles = Vec::with_capacity(oracles_num as usize);
                let (oracles_slice, _rest) = rest.split_at(oracles_num as usize * 32);
                for i in 0..oracles_num {
                    // let (oracle, oracles_slice) = Self::unpack_pubkey(oracles_slice).unwrap();
                    let oracle = oracles_slice.get(i as usize * 32 .. i as usize * 32 + 32).unwrap();
                    let (oracle, _) = Self::unpack_pubkey(oracle).unwrap();
                        // .and_then(|slice| Self::unpack_pubkey(slice).into()).unwrap();
                        // .map(Self::unpack_pubkey)
                        // .ok_or(InvalidInstruction)?;
                    // oracle = oracle.ok();
                    oracles.push(oracle);
                }

                match tag {
                    0 => Self::BuyFor {multiplier, amount, fee, prices, oracles},
                    1 => Self::SellFor {multiplier, amount, fee, prices, oracles},
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
                let (withdrawable_fee_amount, _rest) = rest.split_at(8);
                let withdrawable_fee_amount = withdrawable_fee_amount
                    .try_into()
                    .ok()
                    .map(u64::from_le_bytes)
                    .ok_or(InvalidInstruction)?;

                Self::InitializeSynchronizerAccount {
                    collateral_token_key,
                    remaining_dollar_cap,
                    withdrawable_fee_amount
                }
            }

            3 => { Self::SetMinimumRequiredSignature }
            4 => { Self::SetCollateralToken }
            5 => { Self:: SetRemainingDollarCap }
            6 => { Self::WithdrawFee }
            7 => { Self::WithdrawCollateral }

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
                ref oracles
            } => {
                buf.push(0);
                buf.extend_from_slice(&multiplier.to_le_bytes());
                buf.extend_from_slice(&amount.to_le_bytes());
                buf.extend_from_slice(&fee.to_le_bytes());
                buf.push(prices.len().try_into().unwrap());
                for price in prices {
                    buf.extend_from_slice(&price.to_le_bytes());
                }
                buf.push(oracles.len().try_into().unwrap());
                for key in oracles {
                    buf.extend_from_slice(key.as_ref());
                }
            },

            Self::SellFor {
                multiplier,
                amount,
                fee,
                ref prices,
                ref oracles
            } => {
                buf.push(1);
                buf.extend_from_slice(&multiplier.to_le_bytes());
                buf.extend_from_slice(&amount.to_le_bytes());
                buf.extend_from_slice(&fee.to_le_bytes());
                buf.push(prices.len().try_into().unwrap());
                for price in prices {
                    buf.extend_from_slice(&price.to_le_bytes());
                }
                buf.push(oracles.len().try_into().unwrap());
                for key in oracles {
                    buf.extend_from_slice(key.as_ref());
                }
            },

            // Admin Instructions
            Self::InitializeSynchronizerAccount {
                collateral_token_key,
                remaining_dollar_cap,
                withdrawable_fee_amount,
            } => {
                buf.push(2);
                buf.extend_from_slice(collateral_token_key.as_ref());
                buf.extend_from_slice(&remaining_dollar_cap.to_le_bytes());
                buf.extend_from_slice(&withdrawable_fee_amount.to_le_bytes());
            }
            Self::SetMinimumRequiredSignature => buf.push(3),
            Self::SetCollateralToken => buf.push(4),
            Self::SetRemainingDollarCap => buf.push(5),
            Self::WithdrawFee => buf.push(6),
            Self::WithdrawCollateral => buf.push(7),
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
            oracles: vec![
                Pubkey::from_str("D2YHis8gk2wRHkMEY7bULLsFUk277KdodWFR1nJ9SRgb").unwrap(),
                Pubkey::from_str("EExYKmkDnS5HuUhb33e5ZeGHdZPCdQKJcQXDQTyWSb4X").unwrap()
            ]
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
        expect.extend_from_slice(&[2]);
        expect.extend_from_slice(&[178, 177, 51, 164, 92, 30, 126, 138, 210, 146, 214, 193, 145, 103, 57, 185, 60, 120, 46, 119, 37, 184, 251, 108, 93, 90, 88, 249, 49, 176, 59, 160]);
        expect.extend_from_slice(&[196, 187, 71, 168, 43, 226, 204, 130, 198, 182, 91, 6, 240, 228, 232, 228, 89, 217, 65, 173, 197, 180, 93, 22, 141, 243, 103, 79, 210, 0, 211, 76]);
        assert_eq!(packed, expect);
        let unpacked = SynchronizerInstruction::unpack(&expect).unwrap();
        assert_eq!(unpacked, check);

        // TODO: other instruction packing tests
    }
}
