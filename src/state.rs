//! Synchronizer data

use solana_program::{program_error::ProgramError, program_pack::{IsInitialized, Pack, Sealed}, pubkey::Pubkey};
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};

use crate::instruction::MAX_ORACLES;

/// Synchronizer data.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SynchronizerData {
    /// Is `true` if this structure has been initialized
    pub is_initialized: bool,
    /// USDC Token address
    pub collateral_token_key: Pubkey,
    /// Remaining dollar cap
    pub remaining_dollar_cap: u64,
    /// Withdrawable fee amount
    pub withdrawable_fee_amount: u64,
    /// Minimum required signatures for sell_for/buy_for instruction
    pub minimum_required_signature: u8,
    /// Array of public keys of known oracles
    pub oracles: [Pubkey; MAX_ORACLES],
}
impl Sealed for SynchronizerData {}
impl IsInitialized for SynchronizerData {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}
impl Pack for SynchronizerData {
    /// 1 + 32 + 8 + 8 + 1 + 32 * MAX_ORACLES(10)
    const LEN: usize = 370;
    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let src = array_ref![src, 0, 370];
        let (
            is_initialized,
            collateral_token_key,
            remaining_dollar_cap,
            withdrawable_fee_amount,
            minminimum_required_signature,
            oracles_flat
        ) = array_refs![src, 1, 32, 8, 8, 1, 32 * MAX_ORACLES];

        let is_initialized = match is_initialized {
            [0] => false,
            [1] => true,
            _ => return Err(ProgramError::InvalidAccountData),
        };

        let mut result = SynchronizerData {
            is_initialized,
            collateral_token_key: Pubkey::new_from_array(*collateral_token_key),
            remaining_dollar_cap: u64::from_le_bytes(*remaining_dollar_cap),
            withdrawable_fee_amount: u64::from_le_bytes(*withdrawable_fee_amount),
            minimum_required_signature: u8::from_le_bytes(*minminimum_required_signature),
            oracles: [Pubkey::new_from_array([0u8; 32]); MAX_ORACLES],
        };
        for (src, dst) in oracles_flat.chunks(32).zip(result.oracles.iter_mut()) {
            *dst = Pubkey::new(src);
        }
        Ok(result)
    }

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, 370];
        let (
            is_initialized_dst,
            collateral_token_key_dst,
            remaining_dollar_cap_dst,
            withdrawable_fee_amount_dst,
            minimum_required_signature_dst,
            oracles_flat_dst,
        ) = mut_array_refs![dst, 1, 32, 8, 8, 1, 32 * MAX_ORACLES];

        is_initialized_dst[0] = self.is_initialized as u8;
        collateral_token_key_dst.copy_from_slice(self.collateral_token_key.as_ref());
        *remaining_dollar_cap_dst = self.remaining_dollar_cap.to_le_bytes();
        *withdrawable_fee_amount_dst = self.withdrawable_fee_amount.to_le_bytes();
        minimum_required_signature_dst[0] = self.minimum_required_signature as u8;
        for (i, src) in self.oracles.iter().enumerate() {
            let dst_array = array_mut_ref![oracles_flat_dst, 32 * i, 32];
            dst_array.copy_from_slice(src.as_ref());
        }
    }
}
